"""Reusable test fixtures for Serena testing.

Provides fixtures for:
- MCP server mocking
- Language server mocking
- Temporary project directories
- Performance timing
- Test data generation
"""

from __future__ import annotations

import contextlib
import shutil
import tempfile
import time
from collections.abc import Generator, Iterator
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any
from unittest.mock import MagicMock, patch

import pytest


@dataclass
class MCPServerMock:
    """Mock MCP server for testing tool invocations."""

    tools: list[dict[str, Any]] = field(default_factory=list)
    resources: list[dict[str, Any]] = field(default_factory=list)
    prompts: list[dict[str, Any]] = field(default_factory=list)
    call_history: list[dict[str, Any]] = field(default_factory=list)

    def add_tool(self, name: str, description: str, schema: dict[str, Any] | None = None) -> None:
        """Add a tool to the mock server."""
        self.tools.append(
            {
                "name": name,
                "description": description,
                "inputSchema": schema or {"type": "object", "properties": {}},
            }
        )

    def handle_call(self, method: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        """Handle an MCP method call."""
        self.call_history.append({"method": method, "params": params})

        if method == "tools/list":
            return {"tools": self.tools}
        elif method == "resources/list":
            return {"resources": self.resources}
        elif method == "prompts/list":
            return {"prompts": self.prompts}
        elif method == "tools/call":
            tool_name = params.get("name") if params else None
            return {"content": [{"type": "text", "text": f"Mock response from {tool_name}"}]}

        return {"error": {"code": -32601, "message": f"Unknown method: {method}"}}

    def get_call_count(self, method: str) -> int:
        """Get the number of times a method was called."""
        return sum(1 for call in self.call_history if call["method"] == method)

    def reset(self) -> None:
        """Reset call history."""
        self.call_history.clear()


@pytest.fixture
def mock_mcp_server() -> Generator[MCPServerMock, None, None]:
    """Create a mock MCP server for testing.

    Example:
        def test_tool_call(mock_mcp_server):
            mock_mcp_server.add_tool("find_symbol", "Find a symbol")
            response = mock_mcp_server.handle_call("tools/list")
            assert len(response["tools"]) == 1

    """
    server = MCPServerMock()
    # Add default Serena tools
    server.add_tool("find_symbol", "Find symbols in the codebase")
    server.add_tool("read_file", "Read a file from the filesystem")
    server.add_tool("replace_content", "Replace content in a file")
    yield server


@dataclass
class LanguageServerMock:
    """Mock language server for testing symbol operations."""

    symbols: dict[str, list[dict[str, Any]]] = field(default_factory=dict)
    definitions: dict[str, dict[str, Any]] = field(default_factory=dict)
    references: dict[str, list[dict[str, Any]]] = field(default_factory=list)

    def add_symbol(
        self,
        file_path: str,
        name: str,
        kind: int,
        start_line: int,
        end_line: int,
        start_char: int = 0,
        end_char: int = 0,
    ) -> None:
        """Add a symbol to the mock server."""
        if file_path not in self.symbols:
            self.symbols[file_path] = []

        self.symbols[file_path].append(
            {
                "name": name,
                "kind": kind,
                "range": {
                    "start": {"line": start_line, "character": start_char},
                    "end": {"line": end_line, "character": end_char},
                },
            }
        )

    def get_document_symbols(self, file_path: str) -> list[dict[str, Any]]:
        """Get symbols for a document."""
        return self.symbols.get(file_path, [])

    def find_definition(self, file_path: str, line: int, character: int) -> dict[str, Any] | None:
        """Find definition at position."""
        key = f"{file_path}:{line}:{character}"
        return self.definitions.get(key)


@pytest.fixture
def mock_language_server() -> Generator[LanguageServerMock, None, None]:
    """Create a mock language server for testing.

    Example:
        def test_symbol_lookup(mock_language_server):
            mock_language_server.add_symbol("test.py", "MyClass", 5, 10, 50)
            symbols = mock_language_server.get_document_symbols("test.py")
            assert len(symbols) == 1

    """
    yield LanguageServerMock()


@pytest.fixture
def temp_project_dir() -> Generator[Path, None, None]:
    """Create a temporary project directory for testing.

    Creates a directory with basic project structure:
    - src/ directory
    - test/ directory
    - .serena/ directory

    Example:
        def test_project_creation(temp_project_dir):
            src_file = temp_project_dir / "src" / "main.py"
            src_file.write_text("def main(): pass")
            assert src_file.exists()

    """
    temp_dir = Path(tempfile.mkdtemp(prefix="serena_test_"))
    try:
        # Create basic structure
        (temp_dir / "src").mkdir()
        (temp_dir / "test").mkdir()
        (temp_dir / ".serena").mkdir()
        (temp_dir / ".serena" / "memories").mkdir()

        # Create minimal project config
        (temp_dir / ".serena" / "project.yml").write_text("name: test_project\nlanguage: python\n")

        yield temp_dir
    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)


@dataclass
class PerformanceResult:
    """Result of a performance measurement."""

    operation: str
    duration_ms: float
    memory_mb: float | None = None
    iterations: int = 1

    @property
    def avg_duration_ms(self) -> float:
        """Average duration per iteration."""
        return self.duration_ms / self.iterations


class PerformanceTimer:
    """Context manager for timing operations."""

    def __init__(self, operation: str) -> None:
        self.operation = operation
        self.start_time: float | None = None
        self.end_time: float | None = None
        self._result: PerformanceResult | None = None

    def __enter__(self) -> "PerformanceTimer":
        self.start_time = time.perf_counter()
        return self

    def __exit__(self, *args: Any) -> None:
        self.end_time = time.perf_counter()
        if self.start_time is not None:
            duration_ms = (self.end_time - self.start_time) * 1000
            self._result = PerformanceResult(
                operation=self.operation,
                duration_ms=duration_ms,
            )

    @property
    def result(self) -> PerformanceResult | None:
        """Get the performance result."""
        return self._result

    @property
    def duration_ms(self) -> float:
        """Get the duration in milliseconds."""
        if self._result:
            return self._result.duration_ms
        return 0.0


@pytest.fixture
def performance_timer() -> type[PerformanceTimer]:
    """Fixture providing the PerformanceTimer class.

    Example:
        def test_operation_speed(performance_timer):
            with performance_timer("symbol_lookup") as timer:
                # perform operation
                pass
            assert timer.duration_ms < 100  # Must complete in 100ms

    """
    return PerformanceTimer


@contextlib.contextmanager
def patch_mcp_transport() -> Iterator[MagicMock]:
    """Patch MCP transport for testing without actual IPC.

    Example:
        with patch_mcp_transport() as mock_transport:
            mock_transport.send.return_value = {"result": "ok"}
            # Test MCP operations

    """
    with patch("mcp.server.stdio.stdio_server") as mock_server:
        mock_transport = MagicMock()
        mock_server.return_value.__aenter__.return_value = (mock_transport, mock_transport)
        yield mock_transport


@pytest.fixture
def sample_python_file(temp_project_dir: Path) -> Path:
    """Create a sample Python file for testing.

    Returns the path to a Python file with various symbols.
    """
    content = '''"""Sample module for testing."""

from typing import Optional


class SampleClass:
    """A sample class with methods."""

    def __init__(self, name: str) -> None:
        self.name = name
        self._value: int = 0

    def get_value(self) -> int:
        """Get the current value."""
        return self._value

    def set_value(self, value: int) -> None:
        """Set a new value."""
        self._value = value


def standalone_function(x: int, y: int) -> int:
    """Add two numbers."""
    return x + y


CONSTANT_VALUE: int = 42
'''
    file_path = temp_project_dir / "src" / "sample.py"
    file_path.write_text(content)
    return file_path


@pytest.fixture
def sample_typescript_file(temp_project_dir: Path) -> Path:
    """Create a sample TypeScript file for testing.

    Returns the path to a TypeScript file with various symbols.
    """
    content = """/**
 * Sample TypeScript module for testing.
 */

export interface SampleInterface {
    name: string;
    getValue(): number;
}

export class SampleClass implements SampleInterface {
    private _value: number = 0;

    constructor(public name: string) {}

    getValue(): number {
        return this._value;
    }

    setValue(value: number): void {
        this._value = value;
    }
}

export function standaloneFunction(x: number, y: number): number {
    return x + y;
}

export const CONSTANT_VALUE: number = 42;
"""
    file_path = temp_project_dir / "src" / "sample.ts"
    file_path.write_text(content)
    return file_path


@pytest.fixture
def mock_subprocess() -> Generator[MagicMock, None, None]:
    """Mock subprocess for testing command execution."""
    with patch("subprocess.Popen") as mock_popen:
        mock_process = MagicMock()
        mock_process.communicate.return_value = (b"output", b"")
        mock_process.returncode = 0
        mock_process.poll.return_value = 0
        mock_popen.return_value = mock_process
        yield mock_popen
