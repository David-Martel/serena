"""Custom assertions for Serena testing.

Provides domain-specific assertions for:
- Symbol verification
- File content checking
- MCP response validation
- LSP response verification
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any


class AssertionError(Exception):
    """Custom assertion error with detailed messages."""


def assert_symbol_found(
    symbols: list[dict[str, Any]],
    name: str,
    kind: int | None = None,
    in_file: str | None = None,
) -> dict[str, Any]:
    """Assert that a symbol with the given name exists.

    Args:
        symbols: List of symbol dictionaries from LSP
        name: Expected symbol name
        kind: Optional LSP symbol kind to match
        in_file: Optional file path to restrict search

    Returns:
        The matching symbol dictionary

    Raises:
        AssertionError: If no matching symbol is found

    Example:
        symbols = language_server.get_document_symbols("test.py")
        class_symbol = assert_symbol_found(symbols, "MyClass", kind=5)

    """
    matches = []
    for symbol in symbols:
        if symbol.get("name") == name:
            if kind is not None and symbol.get("kind") != kind:
                continue
            if in_file is not None:
                symbol_file = symbol.get("location", {}).get("uri", "")
                if in_file not in symbol_file:
                    continue
            matches.append(symbol)

    if not matches:
        symbol_names = [s.get("name", "?") for s in symbols]
        raise AssertionError(f"Symbol '{name}' not found. Available symbols: {symbol_names}")

    if len(matches) > 1:
        # Return first match but note there are multiple
        pass

    return matches[0]


def assert_symbol_not_found(
    symbols: list[dict[str, Any]],
    name: str,
) -> None:
    """Assert that a symbol with the given name does NOT exist.

    Args:
        symbols: List of symbol dictionaries from LSP
        name: Symbol name that should not exist

    Raises:
        AssertionError: If the symbol is found

    Example:
        symbols = language_server.get_document_symbols("test.py")
        assert_symbol_not_found(symbols, "DeletedClass")

    """
    for symbol in symbols:
        if symbol.get("name") == name:
            raise AssertionError(f"Symbol '{name}' was found but should not exist")


def assert_file_contains(
    file_path: Path | str,
    expected: str,
    regex: bool = False,
) -> None:
    r"""Assert that a file contains the expected content.

    Args:
        file_path: Path to the file
        expected: Expected content or regex pattern
        regex: If True, treat expected as a regex pattern

    Raises:
        AssertionError: If content not found
        FileNotFoundError: If file doesn't exist

    Example:
        assert_file_contains("test.py", "class MyClass:")
        assert_file_contains("test.py", r"def \w+\(", regex=True)

    """
    path = Path(file_path)
    if not path.exists():
        raise FileNotFoundError(f"File not found: {path}")

    content = path.read_text(encoding="utf-8")

    if regex:
        if not re.search(expected, content):
            raise AssertionError(f"Pattern '{expected}' not found in {path}")
    else:
        if expected not in content:
            # Show context around expected location
            lines = content.split("\n")[:20]
            preview = "\n".join(lines)
            raise AssertionError(f"Content '{expected}' not found in {path}\n" f"File preview:\n{preview}")


def assert_file_not_contains(
    file_path: Path | str,
    unexpected: str,
    regex: bool = False,
) -> None:
    """Assert that a file does NOT contain the specified content.

    Args:
        file_path: Path to the file
        unexpected: Content or regex pattern that should not exist
        regex: If True, treat unexpected as a regex pattern

    Raises:
        AssertionError: If content is found

    """
    path = Path(file_path)
    if not path.exists():
        return  # File doesn't exist, so it can't contain the content

    content = path.read_text(encoding="utf-8")

    if regex:
        match = re.search(unexpected, content)
        if match:
            raise AssertionError(f"Pattern '{unexpected}' should not be in {path}, " f"but found: {match.group()}")
    else:
        if unexpected in content:
            raise AssertionError(f"Content '{unexpected}' should not be in {path}")


def assert_mcp_response_valid(
    response: dict[str, Any],
    expected_keys: list[str] | None = None,
) -> None:
    """Assert that an MCP response is valid.

    Checks:
    - Response is a dictionary
    - No error field (unless expected)
    - Contains expected keys if specified

    Args:
        response: MCP response dictionary
        expected_keys: Optional list of keys that must be present

    Raises:
        AssertionError: If response is invalid

    Example:
        response = mcp_server.call("tools/list")
        assert_mcp_response_valid(response, expected_keys=["tools"])

    """
    if not isinstance(response, dict):
        raise AssertionError(f"MCP response must be a dictionary, got: {type(response)}")

    if "error" in response:
        error = response["error"]
        code = error.get("code", "?")
        message = error.get("message", "Unknown error")
        raise AssertionError(f"MCP response contains error: [{code}] {message}")

    if expected_keys:
        missing = [k for k in expected_keys if k not in response]
        if missing:
            raise AssertionError(f"MCP response missing keys: {missing}. " f"Available keys: {list(response.keys())}")


def assert_mcp_tool_exists(
    tools: list[dict[str, Any]],
    tool_name: str,
) -> dict[str, Any]:
    """Assert that an MCP tool exists in the tools list.

    Args:
        tools: List of tool dictionaries from tools/list
        tool_name: Expected tool name

    Returns:
        The matching tool dictionary

    Raises:
        AssertionError: If tool not found

    Example:
        response = mcp_server.call("tools/list")
        tool = assert_mcp_tool_exists(response["tools"], "find_symbol")

    """
    for tool in tools:
        if tool.get("name") == tool_name:
            return tool

    tool_names = [t.get("name", "?") for t in tools]
    raise AssertionError(f"Tool '{tool_name}' not found. Available tools: {tool_names}")


def assert_lsp_location_valid(location: dict[str, Any]) -> None:
    """Assert that an LSP location is valid.

    Args:
        location: LSP location dictionary

    Raises:
        AssertionError: If location is invalid

    """
    if "uri" not in location and "targetUri" not in location:
        raise AssertionError(f"LSP location missing uri/targetUri: {location}")

    range_key = "range" if "range" in location else "targetRange"
    if range_key not in location:
        raise AssertionError(f"LSP location missing range: {location}")

    range_data = location[range_key]
    for pos_key in ("start", "end"):
        if pos_key not in range_data:
            raise AssertionError(f"LSP range missing {pos_key}: {range_data}")
        pos = range_data[pos_key]
        if "line" not in pos or "character" not in pos:
            raise AssertionError(f"LSP position missing line/character: {pos}")


def assert_json_valid(content: str) -> dict[str, Any] | list[Any]:
    """Assert that a string is valid JSON.

    Args:
        content: String to parse as JSON

    Returns:
        Parsed JSON data

    Raises:
        AssertionError: If JSON is invalid

    """
    try:
        return json.loads(content)
    except json.JSONDecodeError as e:
        preview = content[:200] + "..." if len(content) > 200 else content
        raise AssertionError(f"Invalid JSON: {e}\nContent preview: {preview}")


def assert_within_range(
    value: float,
    min_value: float,
    max_value: float,
    description: str = "Value",
) -> None:
    """Assert that a value is within a specified range.

    Args:
        value: Value to check
        min_value: Minimum acceptable value (inclusive)
        max_value: Maximum acceptable value (inclusive)
        description: Description for error messages

    Raises:
        AssertionError: If value is out of range

    Example:
        assert_within_range(response_time_ms, 0, 100, "Response time")

    """
    if not (min_value <= value <= max_value):
        raise AssertionError(f"{description} {value} is out of range [{min_value}, {max_value}]")
