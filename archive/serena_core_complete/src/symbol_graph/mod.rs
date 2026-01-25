use dashmap::DashMap;
use lsp_types::{DocumentSymbol, SymbolKind, Url};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    pub name: String,
    pub kind: SymbolKind,
    pub uri: Url,
    pub range: lsp_types::Range,
    pub selection_range: lsp_types::Range,
    pub detail: Option<String>,
    pub children: Vec<SymbolNode>,
}

#[derive(Clone)]
pub struct SymbolGraph {
    // Maps URI -> List of top-level symbols
    file_map: Arc<DashMap<Url, Vec<SymbolNode>>>,

    // Flat map of "NamePath" -> List of Symbols
    symbol_map: Arc<DashMap<String, Vec<SymbolNode>>>,
}

impl SymbolGraph {
    pub fn new() -> Self {
        Self {
            file_map: Arc::new(DashMap::new()),
            symbol_map: Arc::new(DashMap::new()),
        }
    }

    pub fn insert_document_symbols(&self, uri: &Url, symbols: Vec<DocumentSymbol>) {
        let mut nodes = Vec::new();
        for sym in &symbols {
            nodes.push(self.process_symbol(sym, uri, ""));
        }
        self.file_map.insert(uri.clone(), nodes);
    }

    fn process_symbol(&self, sym: &DocumentSymbol, uri: &Url, parent_path: &str) -> SymbolNode {
        let my_path = if parent_path.is_empty() {
            sym.name.clone()
        } else {
            format!("{}/{}", parent_path, sym.name)
        };

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

        // Index by full path and simple name
        self.symbol_map
            .entry(my_path)
            .or_default()
            .push(node.clone());
        self.symbol_map
            .entry(sym.name.clone())
            .or_default()
            .push(node.clone());

        node
    }

    pub fn search(&self, query: &str) -> Vec<SymbolNode> {
        // Exact match on NamePath or simple name
        if let Some(results) = self.symbol_map.get(query) {
            return results.clone();
        }

        // Brute force substring search if not found directly
        // This is where Rust speed helps
        // Note: For very large codebases, we might want an Aho-Corasick automaton or suffix tree.
        let mut results = Vec::new();
        for r in self.symbol_map.iter() {
            if r.key().contains(query) {
                results.extend(r.value().clone());
            }
        }
        results
    }

    pub fn get_file_symbols(&self, uri: &Url) -> Option<Vec<SymbolNode>> {
        self.file_map.get(uri).map(|v| v.clone())
    }
}
