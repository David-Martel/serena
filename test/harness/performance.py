"""Performance testing utilities for Serena.

Provides tools for:
- Performance tracking and benchmarking
- Memory usage monitoring
- Latency measurement
- Performance regression detection
"""

from __future__ import annotations

import functools
import gc
import json
import statistics
import time
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, TypeVar

T = TypeVar("T")


@dataclass
class PerformanceMetrics:
    """Container for performance metrics."""

    operation: str
    samples: list[float] = field(default_factory=list)
    memory_samples: list[float] = field(default_factory=list)

    @property
    def count(self) -> int:
        """Number of samples."""
        return len(self.samples)

    @property
    def mean(self) -> float:
        """Mean execution time in milliseconds."""
        if not self.samples:
            return 0.0
        return statistics.mean(self.samples)

    @property
    def median(self) -> float:
        """Median execution time in milliseconds."""
        if not self.samples:
            return 0.0
        return statistics.median(self.samples)

    @property
    def stdev(self) -> float:
        """Standard deviation of execution times."""
        if len(self.samples) < 2:
            return 0.0
        return statistics.stdev(self.samples)

    @property
    def min(self) -> float:
        """Minimum execution time."""
        if not self.samples:
            return 0.0
        return min(self.samples)

    @property
    def max(self) -> float:
        """Maximum execution time."""
        if not self.samples:
            return 0.0
        return max(self.samples)

    @property
    def p95(self) -> float:
        """95th percentile execution time."""
        if not self.samples:
            return 0.0
        sorted_samples = sorted(self.samples)
        idx = int(len(sorted_samples) * 0.95)
        return sorted_samples[min(idx, len(sorted_samples) - 1)]

    @property
    def p99(self) -> float:
        """99th percentile execution time."""
        if not self.samples:
            return 0.0
        sorted_samples = sorted(self.samples)
        idx = int(len(sorted_samples) * 0.99)
        return sorted_samples[min(idx, len(sorted_samples) - 1)]

    def add_sample(self, duration_ms: float, memory_mb: float | None = None) -> None:
        """Add a performance sample."""
        self.samples.append(duration_ms)
        if memory_mb is not None:
            self.memory_samples.append(memory_mb)

    def to_dict(self) -> dict[str, Any]:
        """Convert to dictionary for serialization."""
        return {
            "operation": self.operation,
            "count": self.count,
            "mean_ms": round(self.mean, 3),
            "median_ms": round(self.median, 3),
            "stdev_ms": round(self.stdev, 3),
            "min_ms": round(self.min, 3),
            "max_ms": round(self.max, 3),
            "p95_ms": round(self.p95, 3),
            "p99_ms": round(self.p99, 3),
        }

    def __str__(self) -> str:
        """Human-readable summary."""
        return (
            f"{self.operation}: "
            f"mean={self.mean:.2f}ms, "
            f"median={self.median:.2f}ms, "
            f"p95={self.p95:.2f}ms, "
            f"p99={self.p99:.2f}ms "
            f"(n={self.count})"
        )


class PerformanceTracker:
    """Track performance metrics across multiple operations.

    Example:
        tracker = PerformanceTracker()

        for _ in range(100):
            with tracker.track("symbol_lookup"):
                find_symbol("MyClass")

        print(tracker.get_metrics("symbol_lookup"))
        tracker.assert_performance("symbol_lookup", max_mean_ms=50)

    """

    def __init__(self, baseline_file: Path | None = None) -> None:
        self._metrics: dict[str, PerformanceMetrics] = {}
        self._baseline: dict[str, dict[str, float]] = {}
        if baseline_file and baseline_file.exists():
            self._load_baseline(baseline_file)

    def _load_baseline(self, path: Path) -> None:
        """Load baseline metrics from file."""
        with open(path) as f:
            self._baseline = json.load(f)

    def track(self, operation: str) -> "PerformanceContext":
        """Context manager for tracking an operation.

        Args:
            operation: Name of the operation being tracked

        Returns:
            Context manager that tracks execution time

        """
        if operation not in self._metrics:
            self._metrics[operation] = PerformanceMetrics(operation=operation)
        return PerformanceContext(self._metrics[operation])

    def get_metrics(self, operation: str) -> PerformanceMetrics | None:
        """Get metrics for an operation."""
        return self._metrics.get(operation)

    def all_metrics(self) -> dict[str, PerformanceMetrics]:
        """Get all tracked metrics."""
        return self._metrics.copy()

    def assert_performance(
        self,
        operation: str,
        max_mean_ms: float | None = None,
        max_p95_ms: float | None = None,
        max_p99_ms: float | None = None,
        regression_threshold: float = 0.2,
    ) -> None:
        """Assert that performance meets requirements.

        Args:
            operation: Name of the operation
            max_mean_ms: Maximum acceptable mean time
            max_p95_ms: Maximum acceptable 95th percentile
            max_p99_ms: Maximum acceptable 99th percentile
            regression_threshold: Maximum acceptable regression vs baseline (0.2 = 20%)

        Raises:
            AssertionError: If performance doesn't meet requirements

        """
        metrics = self._metrics.get(operation)
        if not metrics:
            raise AssertionError(f"No metrics tracked for operation: {operation}")

        if max_mean_ms is not None and metrics.mean > max_mean_ms:
            raise AssertionError(f"Performance regression: {operation} mean={metrics.mean:.2f}ms " f"exceeds max={max_mean_ms}ms")

        if max_p95_ms is not None and metrics.p95 > max_p95_ms:
            raise AssertionError(f"Performance regression: {operation} p95={metrics.p95:.2f}ms " f"exceeds max={max_p95_ms}ms")

        if max_p99_ms is not None and metrics.p99 > max_p99_ms:
            raise AssertionError(f"Performance regression: {operation} p99={metrics.p99:.2f}ms " f"exceeds max={max_p99_ms}ms")

        # Check against baseline
        if operation in self._baseline:
            baseline = self._baseline[operation]
            baseline_mean = baseline.get("mean_ms", 0)
            if baseline_mean > 0:
                regression = (metrics.mean - baseline_mean) / baseline_mean
                if regression > regression_threshold:
                    raise AssertionError(
                        f"Performance regression: {operation} regressed by "
                        f"{regression*100:.1f}% (current={metrics.mean:.2f}ms, "
                        f"baseline={baseline_mean:.2f}ms)"
                    )

    def save_baseline(self, path: Path) -> None:
        """Save current metrics as baseline."""
        baseline = {name: metrics.to_dict() for name, metrics in self._metrics.items()}
        with open(path, "w") as f:
            json.dump(baseline, f, indent=2)

    def report(self) -> str:
        """Generate a performance report."""
        lines = ["Performance Report", "=" * 50]
        for name, metrics in sorted(self._metrics.items()):
            lines.append(str(metrics))
        return "\n".join(lines)


class PerformanceContext:
    """Context manager for tracking a single operation."""

    def __init__(self, metrics: PerformanceMetrics) -> None:
        self._metrics = metrics
        self._start_time: float | None = None

    def __enter__(self) -> "PerformanceContext":
        gc.collect()  # Reduce GC noise
        self._start_time = time.perf_counter()
        return self

    def __exit__(self, *args: Any) -> None:
        if self._start_time is not None:
            duration_ms = (time.perf_counter() - self._start_time) * 1000
            self._metrics.add_sample(duration_ms)


def benchmark_operation(
    func: Callable[..., T],
    *args: Any,
    iterations: int = 100,
    warmup: int = 10,
    **kwargs: Any,
) -> PerformanceMetrics:
    """Benchmark a function.

    Args:
        func: Function to benchmark
        *args: Arguments to pass to the function
        iterations: Number of benchmark iterations
        warmup: Number of warmup iterations (not counted)
        **kwargs: Keyword arguments to pass to the function

    Returns:
        PerformanceMetrics with benchmark results

    Example:
        metrics = benchmark_operation(
            find_symbol,
            "MyClass",
            iterations=100,
        )
        print(f"Mean: {metrics.mean:.2f}ms")

    """
    metrics = PerformanceMetrics(operation=func.__name__)

    # Warmup phase
    for _ in range(warmup):
        func(*args, **kwargs)

    # Benchmark phase
    gc.collect()
    for _ in range(iterations):
        start = time.perf_counter()
        func(*args, **kwargs)
        duration_ms = (time.perf_counter() - start) * 1000
        metrics.add_sample(duration_ms)

    return metrics


def performance_test(
    max_mean_ms: float | None = None,
    max_p95_ms: float | None = None,
    iterations: int = 10,
) -> Callable[[Callable[..., T]], Callable[..., T]]:
    """Decorator to mark a test as a performance test.

    Args:
        max_mean_ms: Maximum acceptable mean execution time
        max_p95_ms: Maximum acceptable 95th percentile
        iterations: Number of times to run the test

    Example:
        @performance_test(max_mean_ms=50, iterations=20)
        def test_symbol_lookup_speed():
            find_symbol("MyClass")

    """

    def decorator(func: Callable[..., T]) -> Callable[..., T]:
        @functools.wraps(func)
        def wrapper(*args: Any, **kwargs: Any) -> T:
            metrics = benchmark_operation(
                func,
                *args,
                iterations=iterations,
                warmup=2,
                **kwargs,
            )

            if max_mean_ms is not None and metrics.mean > max_mean_ms:
                raise AssertionError(f"Performance test failed: {func.__name__} " f"mean={metrics.mean:.2f}ms > max={max_mean_ms}ms")

            if max_p95_ms is not None and metrics.p95 > max_p95_ms:
                raise AssertionError(f"Performance test failed: {func.__name__} " f"p95={metrics.p95:.2f}ms > max={max_p95_ms}ms")

            # Return result from last iteration
            return func(*args, **kwargs)

        return wrapper

    return decorator


# Performance baselines for common operations
DEFAULT_BASELINES = {
    "find_symbol": {"max_mean_ms": 100, "max_p95_ms": 200},
    "read_file": {"max_mean_ms": 50, "max_p95_ms": 100},
    "replace_content": {"max_mean_ms": 150, "max_p95_ms": 300},
    "get_symbols_overview": {"max_mean_ms": 200, "max_p95_ms": 400},
    "search_for_pattern": {"max_mean_ms": 500, "max_p95_ms": 1000},
}
