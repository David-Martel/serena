from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

STATE_VERSION = 2
STATE_FILE_RELATIVE = Path(".serena") / "serena_tui_state.json"


def _safe_int(value: Any, default: int) -> int:
    try:
        return int(value)
    except Exception:
        return default


def _safe_bool(value: Any, default: bool) -> bool:
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        v = value.strip().lower()
        if v in {"true", "1", "yes", "y", "on"}:
            return True
        if v in {"false", "0", "no", "n", "off"}:
            return False
    return default


@dataclass
class SerenaTuiState:
    """State persisted per project (stored under `<project>/.serena/serena_tui_state.json`)."""

    version: int = STATE_VERSION
    lastProjectRoot: str | None = None

    # CLI/config selections
    context: str = "ide-assistant"
    modes: list[str] = field(default_factory=list)
    transport: str = "streamable-http"  # stdio|sse|streamable-http
    host: str = "127.0.0.1"
    port: int = 8000

    # Overrides for agent-like runs
    disableDashboard: bool = True
    disableGuiLogWindow: bool = True

    def to_json_dict(self) -> dict[str, Any]:
        return {
            "version": self.version,
            "lastProjectRoot": self.lastProjectRoot,
            "context": self.context,
            "modes": list(self.modes),
            "transport": self.transport,
            "host": self.host,
            "port": self.port,
            "disableDashboard": bool(self.disableDashboard),
            "disableGuiLogWindow": bool(self.disableGuiLogWindow),
        }

    @classmethod
    def from_json_dict(cls, data: dict[str, Any]) -> "SerenaTuiState":
        # Back-compat: older wrapper only stored disableDashboard + transport/context.
        modes = data.get("modes")
        if isinstance(modes, str):
            modes_list: list[str] = [m for m in (x.strip() for x in modes.split(",")) if m]
        elif isinstance(modes, list):
            modes_list = [str(x) for x in modes if str(x).strip()]
        else:
            modes_list = []

        return cls(
            version=_safe_int(data.get("version"), STATE_VERSION),
            lastProjectRoot=data.get("lastProjectRoot"),
            context=str(data.get("context") or "ide-assistant"),
            modes=modes_list,
            transport=str(data.get("transport") or "streamable-http"),
            host=str(data.get("host") or "127.0.0.1"),
            port=_safe_int(data.get("port"), 8000),
            disableDashboard=_safe_bool(data.get("disableDashboard"), True),
            disableGuiLogWindow=_safe_bool(data.get("disableGuiLogWindow"), True),
        )

    @staticmethod
    def state_path(project_root: Path) -> Path:
        return project_root / STATE_FILE_RELATIVE

    @classmethod
    def load(cls, project_root: Path) -> "SerenaTuiState":
        path = cls.state_path(project_root)
        if not path.exists():
            return cls(lastProjectRoot=str(project_root))
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
            if not isinstance(data, dict):
                raise ValueError("state json is not an object")
            state = cls.from_json_dict(data)
            state.lastProjectRoot = str(project_root)
            return state
        except Exception:
            return cls(lastProjectRoot=str(project_root))

    def save(self, project_root: Path) -> None:
        path = self.state_path(project_root)
        path.parent.mkdir(parents=True, exist_ok=True)
        self.lastProjectRoot = str(project_root)
        path.write_text(json.dumps(self.to_json_dict(), indent=2) + "\n", encoding="utf-8")
