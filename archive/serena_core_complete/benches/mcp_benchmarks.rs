//! MCP Protocol Benchmarks
//!
//! Benchmarks for MCP server operations including:
//! - Tool listing and invocation
//! - JSON-RPC message parsing
//! - Resource enumeration
//!
//! Run with: cargo bench --bench mcp_benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde_json::{json, Value};
use std::time::Duration;

/// Benchmark JSON-RPC message parsing
fn bench_jsonrpc_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonrpc_parsing");
    group.measurement_time(Duration::from_secs(5));

    // Sample MCP messages of varying complexity
    let messages = vec![
        (
            "simple_request",
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list"
            }),
        ),
        (
            "tool_call",
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "find_symbol",
                    "arguments": {
                        "name_path_pattern": "MyClass/my_method",
                        "include_body": true,
                        "depth": 2
                    }
                }
            }),
        ),
        (
            "large_response",
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "result": {
                    "content": [{
                        "type": "text",
                        "text": "a".repeat(10000)
                    }]
                }
            }),
        ),
    ];

    for (name, msg) in messages.iter() {
        let msg_str = msg.to_string();

        group.throughput(Throughput::Bytes(msg_str.len() as u64));
        group.bench_with_input(BenchmarkId::new("parse", name), &msg_str, |b, s| {
            b.iter(|| {
                let _: Value = serde_json::from_str(black_box(s)).unwrap();
            });
        });

        group.bench_with_input(BenchmarkId::new("serialize", name), msg, |b, m| {
            b.iter(|| {
                let _: String = serde_json::to_string(black_box(m)).unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark tool schema generation
fn bench_tool_schema(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_schema");
    group.measurement_time(Duration::from_secs(3));

    // Simulate tool schema generation
    let sample_schema = json!({
        "type": "object",
        "properties": {
            "name_path_pattern": {
                "type": "string",
                "description": "The pattern to match symbol names"
            },
            "relative_path": {
                "type": "string",
                "description": "Optional path to restrict search"
            },
            "include_body": {
                "type": "boolean",
                "default": false
            },
            "depth": {
                "type": "integer",
                "minimum": 0,
                "default": 0
            }
        },
        "required": ["name_path_pattern"]
    });

    group.bench_function("schema_serialization", |b| {
        b.iter(|| serde_json::to_string(black_box(&sample_schema)).unwrap());
    });

    group.finish();
}

/// Benchmark message routing (simulated)
fn bench_message_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_routing");

    let methods = vec![
        "tools/list",
        "tools/call",
        "resources/list",
        "resources/read",
        "prompts/list",
        "prompts/get",
        "ping",
        "initialize",
    ];

    group.bench_function("method_dispatch", |b| {
        b.iter(|| {
            for method in &methods {
                let _ = match *method {
                    "tools/list" => 1,
                    "tools/call" => 2,
                    "resources/list" => 3,
                    "resources/read" => 4,
                    "prompts/list" => 5,
                    "prompts/get" => 6,
                    "ping" => 7,
                    "initialize" => 8,
                    _ => 0,
                };
            }
        });
    });

    group.finish();
}

/// Benchmark error response generation
fn bench_error_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_generation");

    group.bench_function("build_error_response", |b| {
        b.iter(|| {
            json!({
                "jsonrpc": "2.0",
                "id": black_box(123),
                "error": {
                    "code": -32600,
                    "message": "Invalid request",
                    "data": {
                        "details": "Method not found",
                        "method": "unknown_method"
                    }
                }
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_jsonrpc_parsing,
    bench_tool_schema,
    bench_message_routing,
    bench_error_generation,
);

criterion_main!(benches);
