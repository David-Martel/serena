"""Comprehensive Test Harness for Serena.

This module provides testing utilities, fixtures, and helpers for:
- Unit testing with mocking and isolation
- Integration testing with LSP servers
- Performance benchmarking
- Security testing
- Property-based testing with Hypothesis
"""

from .assertions import (
    assert_file_contains,
    assert_mcp_response_valid,
    assert_symbol_found,
)
from .fixtures import (
    mock_language_server,
    mock_mcp_server,
    performance_timer,
    temp_project_dir,
)
from .performance import (
    PerformanceTracker,
    benchmark_operation,
)

__all__ = [
    "PerformanceTracker",
    "assert_file_contains",
    "assert_mcp_response_valid",
    "assert_symbol_found",
    "benchmark_operation",
    "mock_language_server",
    "mock_mcp_server",
    "performance_timer",
    "temp_project_dir",
]
