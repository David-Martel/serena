# Serena Performance Testing Strategy

## Overview

This document outlines the comprehensive performance testing strategy for Serena, covering both Python and Rust components. The strategy includes benchmarking, profiling, regression detection, and continuous monitoring.

## Performance Testing Goals

### Primary Objectives
1. **Latency**: MCP tool calls must complete within acceptable thresholds
2. **Throughput**: Support concurrent tool invocations efficiently
3. **Memory**: Minimize memory footprint for long-running sessions
4. **Startup**: Fast initialization for responsive user experience

### Target Metrics

| Operation | Target Mean | Target P95 | Target P99 |
|-----------|-------------|------------|------------|
| `find_symbol` | < 100ms | < 200ms | < 500ms |
| `read_file` | < 50ms | < 100ms | < 200ms |
| `replace_content` | < 150ms | < 300ms | < 500ms |
| `get_symbols_overview` | < 200ms | < 400ms | < 800ms |
| `search_for_pattern` | < 500ms | < 1000ms | < 2000ms |
| MCP server startup | < 2s | < 3s | < 5s |
| LSP server startup | < 5s | < 10s | < 15s |

## Testing Infrastructure

### Python Performance Testing

#### Dependencies
```toml
# pyproject.toml [dev] dependencies
pytest-benchmark>=4.0.0
pytest-timeout>=2.3.1
memray>=1.12.0
pyinstrument>=4.6.0
```

#### Running Benchmarks
```bash
# Run all benchmarks
uv run poe test-bench

# Run with comparison to baseline
uv run pytest test/performance/ -v --benchmark-autosave --benchmark-compare

# Generate HTML report
uv run pytest test/performance/ --benchmark-json=benchmark-results.json
```

#### Example Benchmark Test
```python
# test/performance/test_symbol_operations.py
import pytest
from test.harness.performance import PerformanceTracker, benchmark_operation

@pytest.mark.benchmark
class TestSymbolOperations:
    def test_find_symbol_speed(self, benchmark, language_server):
        """Benchmark find_symbol operation."""
        def find_symbol():
            return language_server.find_symbol("MyClass/method")

        result = benchmark.pedantic(
            find_symbol,
            iterations=50,
            rounds=5,
            warmup_rounds=2,
        )
        assert result is not None

    def test_symbol_cache_hit(self, benchmark, cached_symbols):
        """Benchmark cache hit performance."""
        def cache_lookup():
            return cached_symbols.get("MyClass")

        result = benchmark(cache_lookup)
        assert benchmark.stats["mean"] < 0.001  # < 1ms
```

#### Memory Profiling
```bash
# Profile memory usage
uv run python -m memray run -o output.bin test/performance/memory_test.py

# Generate flame graph
uv run python -m memray flamegraph output.bin -o flamegraph.html

# Check for memory leaks
uv run python -m memray tree output.bin
```

### Rust Performance Testing

#### Benchmark Configuration
```toml
# serena_core/Cargo.toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports", "async_tokio"] }

[[bench]]
name = "mcp_benchmarks"
harness = false

[[bench]]
name = "lsp_benchmarks"
harness = false
```

#### Running Rust Benchmarks
```bash
# Run all benchmarks
cargo bench --package serena_core

# Run specific benchmark group
cargo bench --bench mcp_benchmarks -- jsonrpc_parsing

# Generate comparison report
cargo bench -- --save-baseline main
cargo bench -- --baseline main

# Generate flamegraph (requires cargo-flamegraph)
cargo flamegraph --bench mcp_benchmarks
```

#### Benchmark Categories

1. **MCP Protocol (`benches/mcp_benchmarks.rs`)**
   - JSON-RPC message parsing
   - Tool schema generation
   - Message routing
   - Error response generation

2. **LSP Operations (`benches/lsp_benchmarks.rs`)**
   - Symbol tree traversal
   - Symbol lookup by path
   - Symbol caching
   - File content processing
   - Reference tracking

## Continuous Performance Monitoring

### CI/CD Integration

```yaml
# .github/workflows/performance.yml
name: Performance Tests

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  python-benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'
      - run: pip install uv && uv pip install -e ".[dev]"
      - run: uv run pytest test/performance/ --benchmark-json=results.json
      - uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'pytest'
          output-file-path: results.json
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true

  rust-benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo bench --package serena_core -- --noplot
```

### Performance Regression Detection

```python
# test/harness/regression.py
from pathlib import Path
import json

class PerformanceRegression:
    """Detect performance regressions against baseline."""

    def __init__(self, baseline_path: Path, threshold: float = 0.2):
        self.threshold = threshold
        self.baseline = self._load_baseline(baseline_path)

    def check_regression(self, current_results: dict) -> list[str]:
        """Check for regressions above threshold."""
        regressions = []
        for name, current in current_results.items():
            if name in self.baseline:
                baseline = self.baseline[name]
                change = (current["mean"] - baseline["mean"]) / baseline["mean"]
                if change > self.threshold:
                    regressions.append(
                        f"{name}: {change*100:.1f}% regression "
                        f"(baseline={baseline['mean']:.2f}ms, "
                        f"current={current['mean']:.2f}ms)"
                    )
        return regressions
```

## Profiling Workflows

### Python Profiling

```bash
# CPU profiling with pyinstrument
uv run python -m pyinstrument -r html test/profile_scenario.py

# Line-by-line profiling
uv run python -m line_profiler test/profile_scenario.py

# Statistical profiling
uv run python -m scalene test/profile_scenario.py
```

### Rust Profiling

```bash
# CPU profiling with flamegraph
cargo flamegraph --bench mcp_benchmarks

# Memory profiling with heaptrack
heaptrack ./target/release/serena-mcp-server

# Perf profiling (Linux)
perf record -g ./target/release/serena-mcp-server
perf report
```

## Load Testing

### Concurrent MCP Requests

```python
# test/performance/test_load.py
import asyncio
import pytest
from test.harness.fixtures import mock_mcp_server

@pytest.mark.asyncio
@pytest.mark.benchmark
class TestLoadPerformance:
    async def test_concurrent_tool_calls(self, mcp_client):
        """Test handling of concurrent tool calls."""
        async def make_call(i: int):
            return await mcp_client.call_tool("find_symbol", {
                "name_path_pattern": f"Symbol_{i}"
            })

        tasks = [make_call(i) for i in range(100)]
        results = await asyncio.gather(*tasks)

        assert len(results) == 100
        assert all(r.get("content") for r in results)
```

### Stress Testing

```bash
# Run stress tests with locust (if needed)
locust -f test/performance/locustfile.py --headless -u 100 -r 10 -t 60s
```

## Baseline Management

### Creating Baselines

```bash
# Python baseline
uv run pytest test/performance/ --benchmark-save=baseline

# Rust baseline
cargo bench -- --save-baseline baseline
```

### Baseline Storage

Store baselines in:
- `test/baselines/python/` - Python benchmark baselines
- `serena_core/baselines/` - Rust benchmark baselines

### Baseline Comparison

```bash
# Python comparison
uv run pytest test/performance/ --benchmark-compare=baseline

# Rust comparison
cargo bench -- --baseline baseline
```

## Performance Test Categories

### Unit Performance Tests
- Single operation benchmarks
- Cache hit/miss performance
- Serialization/deserialization speed

### Integration Performance Tests
- End-to-end tool call latency
- LSP server communication overhead
- File system operation performance

### System Performance Tests
- Memory usage over extended sessions
- CPU utilization under load
- Startup/shutdown timing

## Best Practices

### Writing Performance Tests

1. **Isolate Operations**: Test single operations in isolation
2. **Warmup**: Always include warmup rounds before measurement
3. **Statistical Significance**: Run enough iterations for stable results
4. **Realistic Data**: Use representative data sizes and patterns
5. **Clean State**: Reset caches and state between runs

### Avoiding False Positives

1. **Pin Dependencies**: Lock versions to prevent dependency-induced regressions
2. **Consistent Environment**: Run on consistent hardware/containers
3. **Multiple Runs**: Average across multiple CI runs for stability
4. **Noise Filtering**: Use statistical methods to filter outliers

### Documentation

1. **Baseline Reasoning**: Document why baselines were chosen
2. **Threshold Justification**: Explain performance thresholds
3. **Known Limitations**: Note any known performance constraints

## Troubleshooting

### Slow Benchmarks

1. Check for I/O bottlenecks (disk, network)
2. Verify no debug logging enabled
3. Ensure release builds for Rust
4. Check for contention in shared resources

### Inconsistent Results

1. Increase iteration count
2. Add warmup rounds
3. Isolate from other processes
4. Use dedicated CI runners

### Memory Issues

1. Profile with memray/heaptrack
2. Check for memory leaks in loops
3. Verify cleanup in fixtures
4. Monitor GC pressure (Python)

## Next Steps

1. **Implement baseline tests** for all core operations
2. **Set up CI/CD** for automatic regression detection
3. **Create dashboards** for performance visualization
4. **Document thresholds** for all critical paths
5. **Schedule regular reviews** of performance metrics
