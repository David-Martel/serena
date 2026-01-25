from __future__ import annotations

import argparse
import asyncio
import shlex
from pathlib import Path
from typing import Any

from serena.tui.exec import require_executable, run_capture, run_stream_lines
from serena.tui.project import find_project_root
from serena.tui.state import SerenaTuiState


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    p = argparse.ArgumentParser(prog="serena-tui", add_help=True)
    p.add_argument(
        "--project",
        default=None,
        help="Explicit project root (directory containing .serena/project.yml or .git). If omitted, auto-detect from CWD.",
    )
    p.add_argument(
        "--start-dir",
        default=None,
        help="Directory to start project auto-detection from (defaults to CWD). Ignored if --project is provided.",
    )
    return p.parse_args(argv)


def _try_import_textual() -> None:
    try:
        import textual  # noqa: F401

        return
    except Exception as e:
        # Keep this as a friendly runtime error rather than an ImportError stack trace.
        msg = (
            "Textual is not installed. Install the optional TUI dependencies and try again:\n\n"
            "  # from T:\\projects\\serena-source\n"
            '  uv pip install -p .\\.venv\\Scripts\\python.exe -e ".[tui]"\n\n'
            "(If you don't use the repo .venv, omit `-p ...` and use your active environment instead.)\n\n"
            f"Import error: {e}"
        )
        raise RuntimeError(msg) from e


def main() -> None:
    args = _parse_args()
    project_root = Path(args.project).resolve() if args.project else find_project_root(args.start_dir)
    run(project_root)


def run(project_root: Path) -> None:
    _try_import_textual()

    # Import after dependency check (keeps base install lightweight)
    from textual.app import App, ComposeResult
    from textual.containers import Horizontal, Vertical, VerticalScroll
    from textual.widgets import Button, Checkbox, Footer, Header, Input, Label, RichLog, Select, Static

    class SerenaTuiApp(App):
        CSS = """
        #root {
            height: 1fr;
        }
        #sidebar {
            width: 44;
            border: tall $primary;
            padding: 1;
        }
        #main {
            border: tall $primary;
            padding: 1;
        }
        #actions {
            height: 18;
            border: tall $surface;
            padding: 0 1;
        }
        #log {
            height: 1fr;
            border: tall $surface;
        }
        .row {
            height: auto;
            margin: 0 0 1 0;
        }
        """

        BINDINGS = [
            ("q", "quit", "Quit"),
            ("s", "save_state", "Save"),
            ("r", "reload_state", "Reload"),
            ("k", "stop_mcp", "Stop MCP"),
        ]

        def __init__(self, project: Path):
            super().__init__()
            self.project_root = project
            self.state = SerenaTuiState.load(project)
            self._mcp_proc: asyncio.subprocess.Process | None = None

        def compose(self) -> ComposeResult:
            yield Header()
            with Horizontal(id="root"):
                with Vertical(id="sidebar"):
                    yield Static("Serena", classes="row")
                    yield Label(f"Project: {self.project_root}", classes="row")
                    yield Label(f"State: {SerenaTuiState.state_path(self.project_root)}", classes="row")

                    yield Label("Context", classes="row")
                    yield Input(value=self.state.context, id="context", classes="row")

                    yield Label("Modes (comma-separated; empty = defaults)", classes="row")
                    yield Input(value=", ".join(self.state.modes), id="modes", classes="row")

                    yield Label("Transport", classes="row")
                    yield Select(
                        [("stdio", "stdio"), ("sse", "sse"), ("streamable-http", "streamable-http")],
                        value=self.state.transport,
                        id="transport",
                        classes="row",
                    )

                    yield Label("Host", classes="row")
                    yield Input(value=self.state.host, id="host", classes="row")

                    yield Label("Port", classes="row")
                    yield Input(value=str(self.state.port), id="port", classes="row")

                    yield Checkbox("Disable web dashboard", value=bool(self.state.disableDashboard), id="disableDashboard")
                    yield Checkbox("Disable GUI log window", value=bool(self.state.disableGuiLogWindow), id="disableGuiLogWindow")

                    yield Button("Save state", id="save", variant="primary", classes="row")
                    yield Button("Reload state", id="reload", classes="row")

                with Vertical(id="main"):
                    with VerticalScroll(id="actions"):
                        yield Button("Config: edit", id="action_config_edit")
                        yield Button("Project: create", id="action_project_create")
                        yield Button("Project: index", id="action_project_index")
                        yield Button("Health check", id="action_health_check")
                        yield Button("Tools: list", id="action_tools_list")
                        yield Button("Tools: description (uses Tool input below)", id="action_tools_description")
                        yield Button("Print system prompt", id="action_print_system_prompt")
                        yield Button("Start MCP server", id="action_start_mcp")
                        yield Button("Stop MCP server", id="action_stop_mcp")
                        yield Button("Run custom args (uses Custom input below)", id="action_run_custom")

                    yield Label("Tool", classes="row")
                    yield Input(placeholder="e.g. search_for_pattern", id="tool_name", classes="row")

                    yield Label("Custom args", classes="row")
                    yield Input(placeholder="e.g. project index .  OR  tools list", id="custom_args", classes="row")

                    yield RichLog(id="log", highlight=False, markup=False)

            yield Footer()

        def _append_log(self, text: str) -> None:
            log = self.query_one("#log", RichLog)
            log.write(text)

        def _sync_state_from_ui(self) -> None:
            self.state.context = self.query_one("#context", Input).value.strip() or "ide-assistant"
            modes_raw = self.query_one("#modes", Input).value
            self.state.modes = [m for m in (x.strip() for x in modes_raw.split(",")) if m]
            self.state.transport = str(self.query_one("#transport", Select).value)
            self.state.host = self.query_one("#host", Input).value.strip() or "127.0.0.1"
            self.state.port = int(self.query_one("#port", Input).value.strip() or "8000")
            self.state.disableDashboard = bool(self.query_one("#disableDashboard", Checkbox).value)
            self.state.disableGuiLogWindow = bool(self.query_one("#disableGuiLogWindow", Checkbox).value)

        def action_save_state(self) -> None:
            try:
                self._sync_state_from_ui()
                self.state.save(self.project_root)
                self._append_log("[state] saved")
            except Exception as e:
                self._append_log(f"[state] save failed: {e}")

        def action_reload_state(self) -> None:
            try:
                self.state = SerenaTuiState.load(self.project_root)
                self.query_one("#context", Input).value = self.state.context
                self.query_one("#modes", Input).value = ", ".join(self.state.modes)
                self.query_one("#transport", Select).value = self.state.transport
                self.query_one("#host", Input).value = self.state.host
                self.query_one("#port", Input).value = str(self.state.port)
                self.query_one("#disableDashboard", Checkbox).value = bool(self.state.disableDashboard)
                self.query_one("#disableGuiLogWindow", Checkbox).value = bool(self.state.disableGuiLogWindow)
                self._append_log("[state] reloaded")
            except Exception as e:
                self._append_log(f"[state] reload failed: {e}")

        async def _run_serena(self, args: list[str]) -> None:
            exe = require_executable("serena")
            argv = [exe, *args]
            self._append_log(f"$ {' '.join(argv)}")
            rc, output = await run_capture(argv, cwd=str(self.project_root))
            if output.strip():
                self._append_log(output.rstrip("\n"))
            self._append_log(f"[exit {rc}]")

        async def _start_mcp(self) -> None:
            if self._mcp_proc is not None:
                self._append_log("[mcp] already running")
                return

            self._sync_state_from_ui()
            self.state.save(self.project_root)

            if self.state.transport == "stdio":
                self._append_log(
                    "[mcp] transport=stdio is not useful from inside the TUI (no client attached). Use sse or streamable-http."
                )
                return

            exe = require_executable("serena-mcp-server")
            argv = [
                exe,
                "--transport",
                self.state.transport,
                "--context",
                self.state.context,
                "--host",
                self.state.host,
                "--port",
                str(self.state.port),
                "--project",
                str(self.project_root),
            ]

            for mode in self.state.modes:
                argv += ["--mode", mode]

            if self.state.disableDashboard:
                argv += ["--enable-web-dashboard", "false"]
            if self.state.disableGuiLogWindow:
                argv += ["--enable-gui-log-window", "false"]

            self._append_log(f"$ {' '.join(argv)}")

            def on_line(line: str) -> None:
                self._append_log(line)

            self._mcp_proc = await run_stream_lines(argv, cwd=str(self.project_root), on_line=on_line)
            self._append_log("[mcp] started")

        def action_stop_mcp(self) -> None:
            if self._mcp_proc is None:
                self._append_log("[mcp] not running")
                return
            try:
                self._mcp_proc.terminate()
            except Exception:
                try:
                    self._mcp_proc.kill()
                except Exception:
                    pass
            self._mcp_proc = None
            self._append_log("[mcp] stopped")

        async def on_button_pressed(self, event: Button.Pressed) -> None:
            bid = event.button.id or ""
            if bid == "save":
                self.action_save_state()
                return
            if bid == "reload":
                self.action_reload_state()
                return

            try:
                if bid == "action_config_edit":
                    await self._run_serena(["config", "edit"])
                elif bid == "action_project_create":
                    await self._run_serena(["project", "create", str(self.project_root)])
                elif bid == "action_project_index":
                    await self._run_serena(["project", "index", str(self.project_root)])
                elif bid == "action_health_check":
                    await self._run_serena(["health-check", str(self.project_root)])
                elif bid == "action_tools_list":
                    await self._run_serena(["tools", "list"])
                elif bid == "action_tools_description":
                    tool = self.query_one("#tool_name", Input).value.strip()
                    if not tool:
                        self._append_log("[tools] enter a tool name first")
                        return
                    await self._run_serena(["tools", "description", tool, "--context", self.query_one("#context", Input).value.strip()])
                elif bid == "action_print_system_prompt":
                    await self._run_serena(
                        ["print-system-prompt", str(self.project_root), "--context", self.query_one("#context", Input).value.strip()]
                    )
                elif bid == "action_start_mcp":
                    await self._start_mcp()
                elif bid == "action_stop_mcp":
                    self.action_stop_mcp()
                elif bid == "action_run_custom":
                    raw = self.query_one("#custom_args", Input).value.strip()
                    if not raw:
                        self._append_log("[custom] enter args first")
                        return
                    import os

                    parts = shlex.split(raw, posix=(os.name != "nt"))
                    await self._run_serena(parts)
            except Exception as e:
                self._append_log(f"[error] {e}")

        def on_shutdown(self, event: Any | None = None) -> None:
            # Best-effort cleanup if the user quits while MCP is running.
            if self._mcp_proc is not None:
                try:
                    self._mcp_proc.terminate()
                except Exception:
                    pass

    SerenaTuiApp(project_root).run()
