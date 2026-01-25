"""Performance tests for symbol operations.

These tests measure the performance of symbol-related operations
to detect regressions and ensure acceptable latency.

Run with: uv run poe test-bench
"""

import pytest

from test.harness.performance import PerformanceTracker

# Skip if benchmark dependencies not available
pytest.importorskip("pytest_benchmark")


@pytest.fixture
def performance_tracker():
    """Create a fresh performance tracker for tests."""
    return PerformanceTracker()


@pytest.fixture
def mock_symbol_data():
    """Generate mock symbol data for testing."""
    return [{"name": f"Symbol_{i}", "kind": 5, "range": {"start": {"line": i * 10}, "end": {"line": i * 10 + 5}}} for i in range(100)]


@pytest.mark.benchmark
class TestSymbolLookupPerformance:
    """Benchmark tests for symbol lookup operations."""

    def test_find_symbol_by_name(self, benchmark, mock_symbol_data):
        """Benchmark finding a symbol by exact name match."""

        def find_symbol(name: str):
            for symbol in mock_symbol_data:
                if symbol["name"] == name:
                    return symbol
            return None

        result = benchmark.pedantic(
            find_symbol,
            args=("Symbol_50",),
            iterations=100,
            rounds=5,
            warmup_rounds=2,
        )

        assert result is not None
        assert result["name"] == "Symbol_50"

    def test_find_symbol_by_pattern(self, benchmark, mock_symbol_data):
        """Benchmark finding symbols by pattern match."""
        import re

        pattern = re.compile(r"Symbol_5\d")

        def find_by_pattern():
            return [s for s in mock_symbol_data if pattern.match(s["name"])]

        results = benchmark.pedantic(
            find_by_pattern,
            iterations=100,
            rounds=5,
            warmup_rounds=2,
        )

        assert len(results) == 10  # Symbol_50 through Symbol_59

    def test_symbol_cache_performance(self, benchmark):
        """Benchmark symbol cache hit performance."""
        cache = {f"file_{i}.py": [{"name": f"Class_{i}"}] for i in range(1000)}

        def cache_lookup():
            return cache.get("file_500.py")

        result = benchmark.pedantic(
            cache_lookup,
            iterations=1000,
            rounds=10,
            warmup_rounds=3,
        )

        assert result is not None


@pytest.mark.benchmark
class TestFileOperationsPerformance:
    """Benchmark tests for file operations."""

    def test_file_content_line_counting(self, benchmark, tmp_path):
        """Benchmark counting lines in a file."""
        # Create a test file with many lines
        test_file = tmp_path / "large_file.py"
        content = "\n".join([f"line_{i} = {i}" for i in range(10000)])
        test_file.write_text(content)

        def count_lines():
            with open(test_file) as f:
                return sum(1 for _ in f)

        result = benchmark.pedantic(
            count_lines,
            iterations=50,
            rounds=5,
            warmup_rounds=2,
        )

        assert result == 10000

    def test_file_pattern_search(self, benchmark, tmp_path):
        """Benchmark searching for patterns in file content."""
        import re

        test_file = tmp_path / "code.py"
        content = "\n".join([f"def function_{i}(x: int) -> int:" for i in range(1000)])
        test_file.write_text(content)

        pattern = re.compile(r"def (\w+)\(")

        def search_pattern():
            with open(test_file) as f:
                text = f.read()
            return pattern.findall(text)

        results = benchmark.pedantic(
            search_pattern,
            iterations=50,
            rounds=5,
            warmup_rounds=2,
        )

        assert len(results) == 1000


@pytest.mark.benchmark
class TestMCPProtocolPerformance:
    """Benchmark tests for MCP protocol operations."""

    def test_json_serialization(self, benchmark):
        """Benchmark JSON serialization of MCP messages."""
        import json

        message = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "find_symbol",
                "arguments": {
                    "name_path_pattern": "MyClass/my_method",
                    "include_body": True,
                    "depth": 2,
                },
            },
        }

        def serialize():
            return json.dumps(message)

        result = benchmark.pedantic(
            serialize,
            iterations=1000,
            rounds=10,
            warmup_rounds=3,
        )

        assert result is not None

    def test_json_deserialization(self, benchmark):
        """Benchmark JSON deserialization of MCP messages."""
        import json

        json_str = '{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"Symbol found at line 42"}]}}'

        def deserialize():
            return json.loads(json_str)

        result = benchmark.pedantic(
            deserialize,
            iterations=1000,
            rounds=10,
            warmup_rounds=3,
        )

        assert result is not None
        assert "result" in result


@pytest.mark.benchmark
class TestPerformanceTracker:
    """Tests for the PerformanceTracker utility itself."""

    def test_tracker_overhead(self, performance_tracker):
        """Ensure tracking overhead is minimal."""
        # Measure overhead of tracking
        for _ in range(100):
            with performance_tracker.track("noop"):
                pass

        metrics = performance_tracker.get_metrics("noop")
        assert metrics is not None
        # Overhead should be less than 1ms per operation
        assert metrics.mean < 1.0

    def test_multiple_operations(self, performance_tracker):
        """Track multiple different operations."""
        import time

        for i in range(10):
            with performance_tracker.track("fast_op"):
                time.sleep(0.001)  # 1ms

            with performance_tracker.track("slow_op"):
                time.sleep(0.005)  # 5ms

        fast_metrics = performance_tracker.get_metrics("fast_op")
        slow_metrics = performance_tracker.get_metrics("slow_op")

        assert fast_metrics.count == 10
        assert slow_metrics.count == 10
        assert slow_metrics.mean > fast_metrics.mean


@pytest.mark.benchmark
class TestRegressionDetection:
    """Tests for performance regression detection."""

    def test_detect_regression(self, performance_tracker, tmp_path):
        """Test that performance regression is detected."""
        import json
        import time

        # Create baseline
        baseline_file = tmp_path / "baseline.json"
        baseline = {
            "test_operation": {
                "mean_ms": 10.0,
                "p95_ms": 15.0,
            }
        }
        baseline_file.write_text(json.dumps(baseline))

        # Load tracker with baseline
        tracker = PerformanceTracker(baseline_file=baseline_file)

        # Simulate slow operation (regression)
        for _ in range(10):
            with tracker.track("test_operation"):
                time.sleep(0.020)  # 20ms (2x slower than baseline)

        # Should raise due to regression
        with pytest.raises(AssertionError, match="regression"):
            tracker.assert_performance(
                "test_operation",
                regression_threshold=0.5,  # 50% threshold
            )

    def test_no_false_positive(self, performance_tracker, tmp_path):
        """Test that acceptable performance doesn't trigger regression."""
        import json
        import time

        # Create baseline
        baseline_file = tmp_path / "baseline.json"
        baseline = {
            "test_operation": {
                "mean_ms": 10.0,
                "p95_ms": 15.0,
            }
        }
        baseline_file.write_text(json.dumps(baseline))

        tracker = PerformanceTracker(baseline_file=baseline_file)

        # Simulate similar performance
        for _ in range(10):
            with tracker.track("test_operation"):
                time.sleep(0.010)  # 10ms (same as baseline)

        # Should not raise
        tracker.assert_performance(
            "test_operation",
            regression_threshold=0.2,  # 20% threshold
        )
