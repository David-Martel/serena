//! Legacy Serena Core Library
//!
//! **DEPRECATED**: This crate is being phased out in favor of the modular `crates/` workspace.
//!
//! Migration guide:
//! - `serena_core::lsp` → `serena-lsp`
//! - `serena_core::symbol_graph` → `serena-symbol::cache`
//! - `serena_core::web` → `serena-web`
//! - `serena_core::project_host` → `serena-lsp::LspClient` + `serena-symbol`
//!
//! This crate remains for:
//! - Python FFI bindings via PyO3
//! - Legacy integration compatibility

use anyhow::Result;
use ignore::WalkBuilder;
use lsp_types::Url;
use rayon::prelude::*;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

// ============================================================================
// DEPRECATED MODULES - Use crates/ versions instead
// ============================================================================

/// LSP client functionality
///
/// **DEPRECATED**: Use `serena-lsp` crate instead.
/// - `ResourceManager` → `serena_lsp::ResourceManager`
/// - `LspClient` → `serena_lsp::LspClient`
pub mod lsp;

/// Symbol caching and indexing
///
/// **DEPRECATED**: Use `serena-symbol::cache` instead.
/// - `SymbolGraph` → `serena_symbol::SymbolGraph`
/// - `SymbolNode` → `serena_symbol::SymbolNode`
pub mod symbol_graph;

// NOTE: mcp module was archived to archive/serena_core_legacy/
// Tool implementations now live in crates/serena-*/

// NOTE: project_host.rs archived - use serena-lsp::LspClient directly
// NOTE: web/mod.rs archived - use serena-web instead

// Test utilities (available in tests and benchmarks)
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

use symbol_graph::SymbolGraph;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchLine {
    pub line_number: usize,
    pub content: String,
    pub match_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileMatch {
    pub path: String,
    pub lines: Vec<MatchLine>,
}

// Global SymbolGraph instance (for brute force simplicity)
static SYMBOL_GRAPH: std::sync::OnceLock<Arc<SymbolGraph>> = std::sync::OnceLock::new();

fn get_graph() -> Arc<SymbolGraph> {
    SYMBOL_GRAPH
        .get_or_init(|| Arc::new(SymbolGraph::new()))
        .clone()
}

pub fn search_files(
    pattern: &str,
    root: &str,
    relative_paths: Vec<String>,
    context_lines_before: usize,
    context_lines_after: usize,
) -> Result<Vec<FileMatch>> {
    let re = RegexBuilder::new(pattern)
        .dot_matches_new_line(true)
        .build()
        .map_err(|e| anyhow::anyhow!("Invalid regex pattern: {e}"))?;

    let root_path = PathBuf::from(root);

    // Parallelise across files using rayon.
    let results: Vec<FileMatch> = relative_paths
        .into_par_iter()
        .filter_map(|rel_path| {
            let full_path = root_path.join(&rel_path);
            let content = match fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => return None,
            };

            let matches =
                match search_in_content(&content, &re, context_lines_before, context_lines_after) {
                    Ok(m) if !m.is_empty() => m,
                    _ => return None,
                };

            // Flatten matches into FileMatch structs
            Some(
                matches
                    .into_iter()
                    .map(|m| FileMatch {
                        path: rel_path.clone(),
                        lines: m,
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .flatten()
        .collect();

    Ok(results)
}

fn search_in_content(
    content: &str,
    re: &regex::Regex,
    context_before: usize,
    context_after: usize,
) -> Result<Vec<Vec<MatchLine>>> {
    let mut matches = Vec::new();
    let mut line_starts = Vec::new();
    line_starts.push(0usize);
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            line_starts.push(idx + 1);
        }
    }

    let offset_to_line = |offset: usize, line_starts: &Vec<usize>| -> usize {
        match line_starts.binary_search(&offset) {
            Ok(i) => i + 1,
            Err(i) => {
                if i == 0 {
                    1
                } else {
                    i
                }
            }
        }
    };

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    for m in re.find_iter(content) {
        let start = m.start();
        let end = m.end();

        let start_line_num = offset_to_line(start, &line_starts);
        let end_line_num = offset_to_line(end, &line_starts);

        let context_start = if start_line_num > context_before {
            start_line_num.saturating_sub(context_before)
        } else {
            1
        };
        let mut context_end = end_line_num + context_after;
        if context_end > total_lines {
            context_end = total_lines;
        }

        let mut out_lines = Vec::new();
        for line_num in context_start..=context_end {
            let idx = line_num - 1;
            if idx >= lines.len() {
                break;
            }
            let line_content = lines[idx].to_string();
            let match_type = if line_num < start_line_num {
                "prefix".to_string()
            } else if line_num > end_line_num {
                "postfix".to_string()
            } else {
                "match".to_string()
            };
            out_lines.push(MatchLine {
                line_number: line_num,
                content: line_content,
                match_type,
            });
        }

        if !out_lines.is_empty() {
            matches.push(out_lines);
        }
    }

    Ok(matches)
}

pub fn walk_files_gitignored(root: &str, start: Option<&str>) -> Result<Vec<String>> {
    let root_path = PathBuf::from(root);
    let start_path = match start {
        Some(rel) if !rel.is_empty() => root_path.join(rel),
        _ => root_path.clone(),
    };

    let mut builder = WalkBuilder::new(&start_path);
    builder
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .hidden(false)
        .follow_links(true);

    let walker = builder.build();
    let mut results = Vec::new();

    for dent in walker {
        let entry = match dent {
            Ok(e) => e,
            Err(_) => continue,
        };
        let md = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !md.is_file() {
            continue;
        }

        let path = entry.path();
        let rel = match path.strip_prefix(&root_path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        results.push(rel_str);
    }

    Ok(results)
}

/// Start the Rust web backend server
///
/// **DEPRECATED**: Use `serena-web::WebServer` directly instead.
/// This function is kept for Python FFI compatibility.
#[deprecated(
    since = "0.2.0",
    note = "Use serena-web::WebServer instead"
)]
pub fn start_rust_backend(_port: u16, _dashboard_path: String) -> Result<()> {
    // Web module archived - use serena-web crate instead
    anyhow::bail!("start_rust_backend is deprecated. Use serena-web::WebServer instead.")
}

/// Ensure a tool (e.g., language server) is installed
///
/// **DEPRECATED**: Use `serena_lsp::ResourceManager` directly instead.
/// This function is kept for Python FFI compatibility.
#[deprecated(
    since = "0.2.0",
    note = "Use serena_lsp::ResourceManager instead"
)]
pub fn ensure_tool(
    _tool_name: String,
    _url: String,
    _executable_name: String,
    _root_dir: String,
) -> Result<String> {
    // ResourceManager migrated to serena-lsp crate
    anyhow::bail!("ensure_tool is deprecated. Use serena_lsp::ResourceManager instead.")
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: i32,
    pub detail: Option<String>,
    pub uri: String,
    pub range: RangeInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RangeInfo {
    pub start: PositionInfo,
    pub end: PositionInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PositionInfo {
    pub line: u32,
    pub character: u32,
}

/// Find symbols matching a pattern
///
/// **DEPRECATED**: Use `serena_symbol::SymbolGraph::search()` instead.
#[deprecated(
    since = "0.2.0",
    note = "Use serena_symbol::SymbolGraph::search() instead"
)]
pub fn find_symbol(pattern: String) -> Result<Vec<SymbolInfo>> {
    let graph = get_graph();
    let symbols = graph.search(&pattern);

    let mut out = Vec::new();
    for sym in symbols {
        let kind_val = serde_json::to_value(sym.kind)?;
        let kind_int = kind_val.as_i64().unwrap_or(0) as i32;

        out.push(SymbolInfo {
            name: sym.name.clone(),
            kind: kind_int,
            detail: sym.detail.clone(),
            uri: sym.uri.to_string(),
            range: RangeInfo {
                start: PositionInfo {
                    line: sym.range.start.line,
                    character: sym.range.start.character,
                },
                end: PositionInfo {
                    line: sym.range.end.line,
                    character: sym.range.end.character,
                },
            },
        });
    }
    Ok(out)
}

/// Get symbols overview for a file
///
/// **DEPRECATED**: Use `serena_symbol::SymbolGraph::get_file_symbols()` instead.
#[deprecated(
    since = "0.2.0",
    note = "Use serena_symbol::SymbolGraph::get_file_symbols() instead"
)]
pub fn get_symbol_overview(uri_str: String) -> Result<Vec<SymbolInfo>> {
    let graph = get_graph();
    let uri = Url::parse(&uri_str)?;

    if let Some(symbols) = graph.get_file_symbols(&uri) {
        let mut out = Vec::new();
        for sym in symbols {
            let kind_val = serde_json::to_value(sym.kind)?;
            let kind_int = kind_val.as_i64().unwrap_or(0) as i32;

            out.push(SymbolInfo {
                name: sym.name.clone(),
                kind: kind_int,
                detail: sym.detail.clone(),
                uri: sym.uri.to_string(),
                range: RangeInfo {
                    start: PositionInfo {
                        line: sym.range.start.line,
                        character: sym.range.start.character,
                    },
                    end: PositionInfo {
                        line: sym.range.end.line,
                        character: sym.range.end.character,
                    },
                },
            });
        }
        Ok(out)
    } else {
        Ok(vec![])
    }
}
