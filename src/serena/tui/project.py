from __future__ import annotations

from pathlib import Path


def find_project_root(start_dir: str | Path | None = None) -> Path:
    """Find project root by walking up from `start_dir` (or CWD).

    Checks for `.serena/project.yml` first (explicit Serena project), then `.git`.
    Falls back to the starting directory.
    """
    current = (Path(start_dir) if start_dir is not None else Path.cwd()).resolve()

    # First pass: look for .serena
    for directory in [current, *current.parents]:
        if (directory / ".serena" / "project.yml").is_file():
            return directory

    # Second pass: look for .git
    for directory in [current, *current.parents]:
        if (directory / ".git").exists():
            return directory

    return current
