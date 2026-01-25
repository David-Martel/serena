#!/usr/bin/env python
import argparse
import json
from pathlib import Path
from typing import Any

from sensai.util import logging
from sensai.util.logging import LogTime

from serena.agent import SerenaAgent
from serena.config.serena_config import ProjectConfig, SerenaConfig
from serena.perf import benchmark_gather_backends, benchmark_search_backends
from serena.tools import GetSymbolsOverviewTool, SearchForPatternTool

DEFAULT_PROJECT = r"T:\projects\rust-mistral\mistral.rs"

log = logging.getLogger(__name__)


def resolve_project_root(raw_path: str) -> str:
    """Resolve a usable Serena project root from an arbitrary path.

    Handles cases where the user points at the workspace root, a subdirectory,
    or directly at Cargo.toml.
    """
    p = Path(raw_path)
    if p.is_file():
        if p.name.lower() == "cargo.toml":
            return str(p.parent.resolve())
        p = p.parent

    cur = p.resolve()
    while True:
        # Prefer a Cargo workspace root if present
        if (cur / "Cargo.toml").exists():
            return str(cur)
        # Fall back to an existing Serena project configuration if found
        if (cur / ProjectConfig.rel_path_to_project_yml()).exists():
            return str(cur)
        # Or a git root as a last-resort heuristic
        if (cur / ".git").is_dir():
            return str(cur)
        if cur.parent == cur:
            break
        cur = cur.parent

    return str(p.resolve())


def create_agent(project_root: str) -> SerenaAgent:
    """Create a SerenaAgent bound to the given project root.

    The agent is configured for non-interactive benchmarking (no GUI/log window,
    no dashboard).
    """
    cfg = SerenaConfig.from_config_file()
    cfg.log_level = logging.INFO
    cfg.gui_log_window_enabled = False
    cfg.web_dashboard = False
    cfg.jetbrains = False

    agent = SerenaAgent(project=project_root, serena_config=cfg)
    # Warm up language server / project activation path once so that subsequent
    # tool calls are representative.
    agent.execute_task(lambda: log.info("Language server ready for %s", project_root))
    return agent


def _serialize_timing_results(results: dict[str, Any]) -> dict[str, Any]:
    """Convert TimingResult mapping to plain JSON-serializable dict."""
    out: dict[str, Any] = {}
    for name, res in results.items():
        out[name] = {
            "backend": res.backend,
            "seconds": res.seconds,
            "runs": res.runs,
            "extra": res.extra or {},
        }
    return out


def run_core_benchmarks(project_root: str, runs: int) -> dict[str, Any]:
    """Run gather/search backend benchmarks on the given project.

    This exercises the PyO3-backed gather/search primitives directly.
    """
    gather_results = benchmark_gather_backends(
        project_root=project_root,
        relative_path="",
        runs=runs,
    )

    patterns = ["mistral", "unwrap\\(", "expect\\(", "panic!", "unsafe"]
    search_results: dict[str, Any] = {}
    for pattern in patterns:
        res = benchmark_search_backends(
            project_root=project_root,
            pattern=pattern,
            relative_path="",
            runs=runs,
        )
        search_results[pattern] = _serialize_timing_results(res)

    return {
        "project_root": project_root,
        "runs": runs,
        "gather": _serialize_timing_results(gather_results),
        "search": search_results,
    }


def run_pattern_tool_counts(agent: SerenaAgent, patterns: list[str]) -> dict[str, dict[str, int]]:
    """Use SearchForPatternTool to count matches per file for each pattern.

    This exercises the higher-level tool layer that ultimately uses the
    Rust-accelerated search implementation where available.
    """
    tool = agent.get_tool(SearchForPatternTool)
    counts: dict[str, dict[str, int]] = {}

    for pattern in patterns:
        log.info("SearchForPatternTool: pattern %r", pattern)
        try:
            with LogTime(f"pattern_search_{pattern}"):
                result = tool.apply(
                    substring_pattern=pattern,
                    context_lines_before=0,
                    context_lines_after=0,
                    relative_path="",
                    restrict_search_to_code_files=True,
                )
        except Exception as exc:  # pragma: no cover - defensive
            log.warning("SearchForPatternTool failed for %r: %s", pattern, exc)
            counts[pattern] = {}
            continue

        if not result:
            counts[pattern] = {}
            continue

        try:
            mapping = json.loads(result)
        except json.JSONDecodeError:
            log.warning("Non-JSON result from SearchForPatternTool for %r; skipping.", pattern)
            counts[pattern] = {}
            continue

        per_file: dict[str, int] = {}
        for path, matches in mapping.items():
            per_file[path] = len(matches)
        counts[pattern] = per_file

    return counts


def run_symbol_overview_stats(agent: SerenaAgent, project_root: str, max_files: int = 50) -> dict[str, Any]:
    """Collect symbol statistics for the largest Rust source files.

    For the top-N .rs files by size, this uses GetSymbolsOverviewTool to
    gather a rough picture of symbol density per file (number of symbols,
    counts per kind), plus simple file-size/line-count metrics.
    """
    project = agent.get_active_project_or_raise()
    all_files = [p for p in project.gather_source_files() if p.endswith(".rs")]
    base = Path(project_root)

    def _file_size(rel: str) -> int:
        path = base / rel
        try:
            return path.stat().st_size
        except OSError:
            return 0

    sorted_files = sorted(all_files, key=_file_size, reverse=True)[:max_files]

    tool = agent.get_tool(GetSymbolsOverviewTool)
    stats: dict[str, Any] = {}

    for rel in sorted_files:
        log.info("GetSymbolsOverviewTool: %s", rel)
        with LogTime(f"symbols_{rel}"):
            try:
                raw = tool.apply(relative_path=rel)
            except Exception as exc:  # pragma: no cover - defensive
                log.warning("GetSymbolsOverviewTool failed for %s: %s", rel, exc)
                continue
        if not raw:
            continue

        try:
            symbols = json.loads(raw)
        except json.JSONDecodeError:
            log.warning("Non-JSON result from GetSymbolsOverviewTool for %s; skipping.", rel)
            continue

        # Basic per-file metrics
        path = base / rel
        try:
            size_bytes = path.stat().st_size
        except OSError:
            size_bytes = 0

        try:
            with path.open("r", encoding="utf-8", errors="ignore") as f:
                num_lines = sum(1 for _ in f)
        except OSError:
            num_lines = 0

        by_kind: dict[str, int] = {}
        for s in symbols:
            kind = str(s.get("kind", "unknown"))
            by_kind[kind] = by_kind.get(kind, 0) + 1

        stats[rel] = {
            "size_bytes": size_bytes,
            "num_lines": num_lines,
            "num_symbols": len(symbols),
            "symbols_by_kind": by_kind,
        }

    return stats


def main() -> None:
    parser = argparse.ArgumentParser(description="Benchmark Serena PyO3 backends and tool usage on mistral.rs.")
    parser.add_argument(
        "--project",
        type=str,
        default=DEFAULT_PROJECT,
        help=("Path to mistral.rs project root or any path within it (Cargo.toml, subdir, etc.)."),
    )
    parser.add_argument("--runs", type=int, default=5, help="Number of runs per backend.")
    parser.add_argument(
        "--out",
        type=str,
        default="mistral_rs_serena_bench.json",
        help="Output JSON file for aggregated results.",
    )
    parser.add_argument(
        "--max-symbol-files",
        type=int,
        default=50,
        help="Maximum number of .rs files to inspect with GetSymbolsOverviewTool.",
    )
    args = parser.parse_args()

    project_root = resolve_project_root(args.project)
    logging.configure(level=logging.INFO)
    log.info("Using project root: %s", project_root)

    # Core gather/search benchmarks (Python vs Rust backends)
    perf = run_core_benchmarks(project_root=project_root, runs=args.runs)

    # Tool-level analysis using the SerenaAgent and serena.tools.*
    agent = create_agent(project_root)
    patterns = ["mistral", "unwrap\\(", "expect\\(", "panic!", "unsafe"]
    pattern_counts = run_pattern_tool_counts(agent, patterns)
    symbol_stats = run_symbol_overview_stats(agent, project_root, max_files=args.max_symbol_files)

    result: dict[str, Any] = {
        "project_root": project_root,
        "runs": args.runs,
        "perf": perf,
        "pattern_counts": pattern_counts,
        "symbol_overview": symbol_stats,
    }

    out_path = Path(args.out)
    out_path.write_text(json.dumps(result, indent=2), encoding="utf-8")
    log.info("Wrote results to %s", out_path)


if __name__ == "__main__":
    main()
