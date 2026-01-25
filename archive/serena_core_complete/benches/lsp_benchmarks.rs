//! LSP Operations Benchmarks
//!
//! Benchmarks for Language Server Protocol operations including:
//! - Symbol parsing and caching
//! - File content processing
//! - Reference tracking
//!
//! Run with: cargo bench --bench lsp_benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::time::Duration;

/// Mock symbol structure for benchmarking
#[derive(Clone, Debug)]
struct MockSymbol {
    name: String,
    kind: u32,
    start_line: u32,
    end_line: u32,
    children: Vec<MockSymbol>,
}

impl MockSymbol {
    fn new(name: &str, kind: u32, start: u32, end: u32) -> Self {
        Self {
            name: name.to_string(),
            kind,
            start_line: start,
            end_line: end,
            children: Vec::new(),
        }
    }

    fn with_children(mut self, children: Vec<MockSymbol>) -> Self {
        self.children = children;
        self
    }
}

/// Generate a mock symbol tree of specified depth and breadth
fn generate_symbol_tree(depth: usize, breadth: usize, prefix: &str) -> Vec<MockSymbol> {
    if depth == 0 {
        return vec![];
    }

    (0..breadth)
        .map(|i| {
            let name = format!("{}_{}", prefix, i);
            let start = (i * 10) as u32;
            let end = start + 9;
            MockSymbol::new(&name, 5, start, end).with_children(generate_symbol_tree(
                depth - 1,
                breadth,
                &name,
            ))
        })
        .collect()
}

/// Count all symbols in tree
fn count_symbols(symbols: &[MockSymbol]) -> usize {
    symbols
        .iter()
        .fold(0, |acc, s| acc + 1 + count_symbols(&s.children))
}

/// Find symbol by name path (e.g., "Class/method")
fn find_symbol_by_path<'a>(symbols: &'a [MockSymbol], path: &[&str]) -> Option<&'a MockSymbol> {
    if path.is_empty() {
        return None;
    }

    for symbol in symbols {
        if symbol.name.ends_with(path[0]) {
            if path.len() == 1 {
                return Some(symbol);
            }
            return find_symbol_by_path(&symbol.children, &path[1..]);
        }
    }
    None
}

/// Benchmark symbol tree traversal
fn bench_symbol_traversal(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbol_traversal");
    group.measurement_time(Duration::from_secs(5));

    // Generate trees of different sizes
    let sizes = vec![
        ("small", 3, 5),  // ~156 symbols
        ("medium", 4, 8), // ~4681 symbols
        ("large", 5, 10), // ~111111 symbols
    ];

    for (name, depth, breadth) in sizes {
        let tree = generate_symbol_tree(depth, breadth, "Symbol");
        let symbol_count = count_symbols(&tree);

        group.throughput(Throughput::Elements(symbol_count as u64));

        group.bench_with_input(BenchmarkId::new("count", name), &tree, |b, tree| {
            b.iter(|| count_symbols(black_box(tree)));
        });
    }

    group.finish();
}

/// Benchmark symbol lookup by path
fn bench_symbol_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbol_lookup");
    group.measurement_time(Duration::from_secs(3));

    let tree = generate_symbol_tree(4, 8, "Class");

    // Different path lengths
    let paths = vec![
        vec!["Class_0"],
        vec!["Class_0", "Class_0_0"],
        vec!["Class_0", "Class_0_0", "Class_0_0_0"],
        vec!["Class_0", "Class_0_0", "Class_0_0_0", "Class_0_0_0_0"],
    ];

    for path in paths {
        let path_str = path.join("/");
        group.bench_with_input(BenchmarkId::new("find", &path_str), &path, |b, path| {
            b.iter(|| find_symbol_by_path(black_box(&tree), black_box(&path[..])));
        });
    }

    group.finish();
}

/// Benchmark symbol caching operations
fn bench_symbol_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbol_cache");
    group.measurement_time(Duration::from_secs(3));

    // Simulate cache operations
    let mut cache: HashMap<String, Vec<MockSymbol>> = HashMap::new();

    // Pre-populate cache with some files
    for i in 0..100 {
        let key = format!("file_{}.rs", i);
        cache.insert(key, generate_symbol_tree(3, 5, &format!("File{}", i)));
    }

    group.bench_function("cache_lookup_hit", |b| {
        b.iter(|| cache.get(black_box("file_50.rs")));
    });

    group.bench_function("cache_lookup_miss", |b| {
        b.iter(|| cache.get(black_box("nonexistent.rs")));
    });

    group.bench_function("cache_insert", |b| {
        let symbols = generate_symbol_tree(3, 5, "New");
        b.iter(|| {
            let mut c = cache.clone();
            c.insert(black_box("new_file.rs".to_string()), symbols.clone());
        });
    });

    group.finish();
}

/// Benchmark file content processing (line counting, etc.)
fn bench_content_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_processing");

    // Generate sample file content
    let small_content: String = (0..100)
        .map(|i| format!("line {} with some content\n", i))
        .collect();

    let large_content: String = (0..10000)
        .map(|i| {
            format!(
                "line {} with some content and more text to make it longer\n",
                i
            )
        })
        .collect();

    group.throughput(Throughput::Bytes(small_content.len() as u64));
    group.bench_function("count_lines_small", |b| {
        b.iter(|| black_box(&small_content).lines().count());
    });

    group.throughput(Throughput::Bytes(large_content.len() as u64));
    group.bench_function("count_lines_large", |b| {
        b.iter(|| black_box(&large_content).lines().count());
    });

    // Pattern matching benchmark
    let pattern = regex::Regex::new(r"line \d+").unwrap();

    group.bench_function("pattern_match_small", |b| {
        b.iter(|| pattern.find_iter(black_box(&small_content)).count());
    });

    group.bench_function("pattern_match_large", |b| {
        b.iter(|| pattern.find_iter(black_box(&large_content)).count());
    });

    group.finish();
}

/// Benchmark reference tracking
fn bench_reference_tracking(c: &mut Criterion) {
    let mut group = c.benchmark_group("reference_tracking");

    // Simulate a reference map: symbol -> list of references
    let mut references: HashMap<String, Vec<(String, u32)>> = HashMap::new();

    // Add references for many symbols
    for i in 0..1000 {
        let symbol = format!("Symbol_{}", i);
        let refs: Vec<(String, u32)> = (0..i % 50)
            .map(|j| (format!("file_{}.rs", j), j * 10))
            .collect();
        references.insert(symbol, refs);
    }

    group.bench_function("find_references", |b| {
        b.iter(|| references.get(black_box("Symbol_500")));
    });

    group.bench_function("collect_all_refs", |b| {
        b.iter(|| references.values().flatten().count());
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_symbol_traversal,
    bench_symbol_lookup,
    bench_symbol_cache,
    bench_content_processing,
    bench_reference_tracking,
);

criterion_main!(benches);
