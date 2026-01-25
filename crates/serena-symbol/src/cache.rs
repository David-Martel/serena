//! Symbol caching with concurrent thread-safe access
//!
//! Provides a high-performance symbol graph for indexing and searching symbols
//! across a codebase. Uses DashMap for lock-free concurrent access.

use dashmap::DashMap;
use lsp_types::{DocumentSymbol, SymbolKind};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use url::Url;

/// A cached representation of a code symbol
///
/// Contains all the information needed for symbol search and navigation
/// without requiring LSP round-trips.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    /// Symbol name (e.g., function name, class name)
    pub name: String,
    /// Kind of symbol (function, class, variable, etc.)
    pub kind: SymbolKind,
    /// URI of the file containing the symbol
    pub uri: Url,
    /// Full range of the symbol in the document
    pub range: lsp_types::Range,
    /// The range that should be selected when navigating to the symbol
    pub selection_range: lsp_types::Range,
    /// Optional detail string (e.g., type information)
    pub detail: Option<String>,
    /// Child symbols (for nested structures like classes with methods)
    pub children: Vec<SymbolNode>,
}

/// Thread-safe symbol graph for fast symbol lookup
///
/// Maintains two concurrent maps:
/// - `file_map`: URI -> List of top-level symbols in that file
/// - `symbol_map`: Symbol path/name -> List of matching symbols
///
/// This allows O(1) lookups by file or by symbol name, with fallback
/// to substring search for fuzzy matching.
#[derive(Clone)]
pub struct SymbolGraph {
    /// Maps file URIs to their top-level symbols
    file_map: Arc<DashMap<Url, Vec<SymbolNode>>>,

    /// Maps symbol paths/names to their locations
    /// Keys include both full paths ("Class/method") and simple names ("method")
    symbol_map: Arc<DashMap<String, Vec<SymbolNode>>>,
}

impl Default for SymbolGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolGraph {
    /// Create a new empty symbol graph
    pub fn new() -> Self {
        Self {
            file_map: Arc::new(DashMap::new()),
            symbol_map: Arc::new(DashMap::new()),
        }
    }

    /// Index symbols from an LSP documentSymbol response
    ///
    /// Processes the symbol tree recursively, indexing each symbol by:
    /// - Full path (e.g., "MyClass/my_method")
    /// - Simple name (e.g., "my_method")
    pub fn insert_document_symbols(&self, uri: &Url, symbols: Vec<DocumentSymbol>) {
        let mut nodes = Vec::new();
        for sym in &symbols {
            nodes.push(self.process_symbol(sym, uri, ""));
        }
        self.file_map.insert(uri.clone(), nodes);
    }

    /// Recursively process a symbol and its children
    fn process_symbol(&self, sym: &DocumentSymbol, uri: &Url, parent_path: &str) -> SymbolNode {
        let my_path = if parent_path.is_empty() {
            sym.name.clone()
        } else {
            format!("{}/{}", parent_path, sym.name)
        };

        // Process children recursively
        let mut children = Vec::new();
        if let Some(sym_children) = &sym.children {
            for child in sym_children {
                children.push(self.process_symbol(child, uri, &my_path));
            }
        }

        let node = SymbolNode {
            name: sym.name.clone(),
            kind: sym.kind,
            uri: uri.clone(),
            range: sym.range,
            selection_range: sym.selection_range,
            detail: sym.detail.clone(),
            children,
        };

        // Index by full path (e.g., "Class/method")
        self.symbol_map
            .entry(my_path.clone())
            .or_default()
            .push(node.clone());

        // Also index by simple name for easier lookup, but only if different
        // from the path (avoids duplicate entries for top-level symbols)
        if my_path != sym.name {
            self.symbol_map
                .entry(sym.name.clone())
                .or_default()
                .push(node.clone());
        }

        node
    }

    /// Search for symbols matching a query
    ///
    /// First tries exact match on symbol path or name.
    /// Falls back to substring search if no exact match found.
    pub fn search(&self, query: &str) -> Vec<SymbolNode> {
        // Try exact match first (O(1) lookup)
        if let Some(results) = self.symbol_map.get(query) {
            return results.clone();
        }

        // Fall back to substring search
        // For very large codebases, consider using Aho-Corasick or a suffix tree
        let mut results = Vec::new();
        for r in self.symbol_map.iter() {
            if r.key().contains(query) {
                results.extend(r.value().clone());
            }
        }
        results
    }

    /// Search with case-insensitive matching
    pub fn search_case_insensitive(&self, query: &str) -> Vec<SymbolNode> {
        let query_lower = query.to_lowercase();

        let mut results = Vec::new();
        for r in self.symbol_map.iter() {
            if r.key().to_lowercase().contains(&query_lower) {
                results.extend(r.value().clone());
            }
        }
        results
    }

    /// Get all symbols for a specific file
    pub fn get_file_symbols(&self, uri: &Url) -> Option<Vec<SymbolNode>> {
        self.file_map.get(uri).map(|v| v.clone())
    }

    /// Remove symbols for a specific file
    pub fn remove_file(&self, uri: &Url) {
        // Remove from file map
        if let Some((_, symbols)) = self.file_map.remove(uri) {
            // Also clean up symbol map entries
            for sym in symbols {
                self.remove_symbol_entries(&sym);
            }
        }
    }

    /// Helper to remove symbol map entries recursively
    fn remove_symbol_entries(&self, sym: &SymbolNode) {
        // Remove entries that match this symbol's URI
        self.symbol_map.retain(|_key, symbols| {
            symbols.retain(|s| s.uri != sym.uri || s.name != sym.name);
            !symbols.is_empty()
        });

        // Recursively remove children
        for child in &sym.children {
            self.remove_symbol_entries(child);
        }
    }

    /// Clear all cached symbols
    pub fn clear(&self) {
        self.file_map.clear();
        self.symbol_map.clear();
    }

    /// Get statistics about the cache
    pub fn stats(&self) -> SymbolGraphStats {
        let file_count = self.file_map.len();
        let symbol_count = self.symbol_map.len();
        let total_symbols: usize = self.symbol_map.iter().map(|r| r.value().len()).sum();

        SymbolGraphStats {
            file_count,
            unique_symbol_names: symbol_count,
            total_symbol_entries: total_symbols,
        }
    }
}

/// Statistics about the symbol graph cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolGraphStats {
    /// Number of files indexed
    pub file_count: usize,
    /// Number of unique symbol names/paths
    pub unique_symbol_names: usize,
    /// Total number of symbol entries (including duplicates across files)
    pub total_symbol_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{Position, Range};

    fn make_test_symbol(name: &str, kind: SymbolKind) -> DocumentSymbol {
        #[allow(deprecated)]
        DocumentSymbol {
            name: name.to_string(),
            kind,
            range: Range::new(Position::new(0, 0), Position::new(10, 0)),
            selection_range: Range::new(Position::new(0, 0), Position::new(0, name.len() as u32)),
            detail: Some("test".to_string()),
            children: None,
            tags: None,
            deprecated: None,
        }
    }

    #[test]
    fn test_insert_and_search() {
        let graph = SymbolGraph::new();
        let uri = Url::parse("file:///test.rs").unwrap();

        let symbols = vec![
            make_test_symbol("MyClass", SymbolKind::CLASS),
            make_test_symbol("my_function", SymbolKind::FUNCTION),
        ];

        graph.insert_document_symbols(&uri, symbols);

        // Exact match
        let results = graph.search("MyClass");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "MyClass");

        // Substring search
        let results = graph.search("func");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "my_function");
    }

    #[test]
    fn test_get_file_symbols() {
        let graph = SymbolGraph::new();
        let uri = Url::parse("file:///test.rs").unwrap();

        let symbols = vec![make_test_symbol("Test", SymbolKind::CLASS)];
        graph.insert_document_symbols(&uri, symbols);

        let file_symbols = graph.get_file_symbols(&uri);
        assert!(file_symbols.is_some());
        assert_eq!(file_symbols.unwrap().len(), 1);
    }

    #[test]
    fn test_clear() {
        let graph = SymbolGraph::new();
        let uri = Url::parse("file:///test.rs").unwrap();

        graph.insert_document_symbols(&uri, vec![make_test_symbol("Test", SymbolKind::CLASS)]);
        assert!(!graph.search("Test").is_empty());

        graph.clear();
        assert!(graph.search("Test").is_empty());
    }

    #[test]
    fn test_stats() {
        let graph = SymbolGraph::new();
        let uri = Url::parse("file:///test.rs").unwrap();

        graph.insert_document_symbols(
            &uri,
            vec![
                make_test_symbol("ClassA", SymbolKind::CLASS),
                make_test_symbol("ClassB", SymbolKind::CLASS),
            ],
        );

        let stats = graph.stats();
        assert_eq!(stats.file_count, 1);
        assert!(stats.unique_symbol_names >= 2);
    }
}
