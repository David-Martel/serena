from __future__ import annotations

import asyncio
import os
import shutil
import sys
from collections.abc import Callable, Sequence
from pathlib import Path
from typing import Any

_BACKGROUND_TASKS: set[asyncio.Task[Any]] = set()


def _track_background_task(task: asyncio.Task[Any]) -> None:
    _BACKGROUND_TASKS.add(task)
    task.add_done_callback(_BACKGROUND_TASKS.discard)


def _candidate_names(base: str) -> list[str]:
    if os.name == "nt":
        return [f"{base}.exe", base]
    return [base]


def find_executable(base: str) -> str | None:
    """Find an executable by name (cross-platform).

    Prefers PATH, then falls back to the venv Scripts/bin directory containing the current Python.
    """
    for name in _candidate_names(base):
        found = shutil.which(name)
        if found:
            return found

    # Fallback: same directory as current Python (common in venvs)
    py_dir = Path(sys.executable).resolve().parent
    for name in _candidate_names(base):
        candidate = py_dir / name
        if candidate.exists():
            return str(candidate)

    return None


def require_executable(base: str) -> str:
    exe = find_executable(base)
    if not exe:
        raise FileNotFoundError(f"Could not find '{base}' on PATH or next to the active Python")
    return exe


async def run_capture(
    argv: Sequence[str],
    cwd: str | None = None,
    env: dict[str, str] | None = None,
    timeout_s: float | None = None,
) -> tuple[int, str]:
    """Run a command and capture combined stdout/stderr."""
    proc = await asyncio.create_subprocess_exec(
        *argv,
        cwd=cwd,
        env=env,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
    )

    async def read_all() -> str:
        assert proc.stdout is not None
        out = await proc.stdout.read()
        return out.decode(errors="replace")

    try:
        if timeout_s is None:
            output = await read_all()
        else:
            output = await asyncio.wait_for(read_all(), timeout=timeout_s)
    finally:
        # Ensure process is reaped
        try:
            rc = await asyncio.wait_for(proc.wait(), timeout=1)
        except Exception:
            proc.kill()
            rc = await proc.wait()

    return rc, output


async def run_stream_lines(
    argv: Sequence[str],
    cwd: str | None,
    on_line: Callable[[str], None],
    env: dict[str, str] | None = None,
) -> asyncio.subprocess.Process:
    """Run a process and stream stdout/stderr lines to callback. Returns the process handle."""
    proc = await asyncio.create_subprocess_exec(
        *argv,
        cwd=cwd,
        env=env,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
    )

    async def pump() -> None:
        assert proc.stdout is not None
        while True:
            line = await proc.stdout.readline()
            if not line:
                break
            on_line(line.decode(errors="replace").rstrip("\n"))

    task = asyncio.create_task(pump())
    _track_background_task(task)
    return proc
