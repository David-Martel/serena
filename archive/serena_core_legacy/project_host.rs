use crate::lsp::LspClient;
use crate::symbol_graph::{SymbolGraph, SymbolNode};
use anyhow::Result;
use lsp_types::{
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument},
    request::{DocumentSymbolRequest, GotoDefinition, HoverRequest, References, Rename},
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentSymbolResponse, GotoDefinitionParams, HoverParams, Position, ReferenceContext,
    ReferenceParams, RenameParams, TextDocumentContentChangeEvent, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentPositionParams, Url, VersionedTextDocumentIdentifier,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

static TRACING_INIT: std::sync::Once = std::sync::Once::new();

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocationInfo {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: i32,
    pub detail: Option<String>,
    pub uri: String,
    pub range: RangeInfo,
    pub selection_range: RangeInfo,
    pub children: Vec<SymbolInfo>,
    pub location: LocationInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextEdit {
    pub new_text: String,
    pub range: RangeInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RenameResult {
    pub changes: HashMap<String, Vec<TextEdit>>,
}

pub struct ProjectHost {
    root_uri: Url,
    runtime: Runtime,
    lsp_client: Option<Arc<LspClient>>,
    symbol_graph: Arc<SymbolGraph>,
}

impl ProjectHost {
    pub fn new(root_path: String) -> Result<Self> {
        TRACING_INIT.call_once(|| {
            tracing_subscriber::fmt().with_env_filter("debug").init();
        });

        let root_path = PathBuf::from(&root_path);
        let root_uri = Url::from_directory_path(&root_path)
            .map_err(|_| anyhow::anyhow!("Invalid root path"))?;

        let runtime = Runtime::new()?;

        Ok(Self {
            root_uri,
            runtime,
            lsp_client: None,
            symbol_graph: Arc::new(SymbolGraph::new()),
        })
    }

    pub fn start_lsp(&mut self, cmd: String, args: Vec<String>) -> Result<()> {
        let client = self
            .runtime
            .block_on(async { LspClient::new(cmd, args).await })?;

        let client = Arc::new(client);
        self.lsp_client = Some(client.clone());

        // Initialize
        let root_uri = self.root_uri.clone();
        self.runtime
            .block_on(async { client.initialize(root_uri).await })?;

        Ok(())
    }

    pub fn did_open(
        &self,
        relative_path: String,
        content: String,
        language_id: String,
    ) -> Result<()> {
        let client = self.get_client()?;
        let uri = self.resolve_uri(&relative_path)?;

        self.runtime.block_on(async move {
            let params = DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id,
                    version: 0,
                    text: content,
                },
            };
            client
                .send_notification::<DidOpenTextDocument>(params)
                .await
        })?;
        Ok(())
    }

    pub fn did_change(&self, relative_path: String, content: String, version: i32) -> Result<()> {
        let client = self.get_client()?;
        let uri = self.resolve_uri(&relative_path)?;

        self.runtime.block_on(async move {
            let params = DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri,
                    version: version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None, // Full content sync
                    range_length: None,
                    text: content,
                }],
            };
            client
                .send_notification::<DidChangeTextDocument>(params)
                .await
        })?;
        Ok(())
    }

    pub fn did_close(&self, relative_path: String) -> Result<()> {
        let client = self.get_client()?;
        let uri = self.resolve_uri(&relative_path)?;

        self.runtime.block_on(async move {
            let params = DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri },
            };
            client
                .send_notification::<DidCloseTextDocument>(params)
                .await
        })?;
        Ok(())
    }

    pub fn definition(
        &self,
        relative_path: String,
        line: u32,
        character: u32,
    ) -> Result<Vec<LocationInfo>> {
        let client = self.get_client()?;
        let uri = self.resolve_uri(&relative_path)?;

        let result = self.runtime.block_on(async move {
            let params = GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position { line, character },
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            };
            client.send_request::<GotoDefinition>(params).await
        })?;

        let mut out = Vec::new();
        if let Some(locations) = result {
            match locations {
                lsp_types::GotoDefinitionResponse::Scalar(loc) => {
                    out.push(self.location_to_info(&loc));
                }
                lsp_types::GotoDefinitionResponse::Array(locs) => {
                    for loc in locs {
                        out.push(self.location_to_info(&loc));
                    }
                }
                lsp_types::GotoDefinitionResponse::Link(_links) => {
                    // Handle LocationLink if needed
                }
            }
        }
        Ok(out)
    }

    pub fn references(
        &self,
        relative_path: String,
        line: u32,
        character: u32,
        include_decl: bool,
    ) -> Result<Vec<LocationInfo>> {
        let client = self.get_client()?;
        let uri = self.resolve_uri(&relative_path)?;

        let result = self.runtime.block_on(async move {
            let params = ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position { line, character },
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
                context: ReferenceContext {
                    include_declaration: include_decl,
                },
            };
            client.send_request::<References>(params).await
        })?;

        let mut out = Vec::new();
        if let Some(locations) = result {
            for loc in locations {
                out.push(self.location_to_info(&loc));
            }
        }
        Ok(out)
    }

    pub fn hover(
        &self,
        relative_path: String,
        line: u32,
        character: u32,
    ) -> Result<Option<String>> {
        let client = self.get_client()?;
        let uri = self.resolve_uri(&relative_path)?;

        let result = self.runtime.block_on(async move {
            let params = HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position { line, character },
                },
                work_done_progress_params: Default::default(),
            };
            client.send_request::<HoverRequest>(params).await
        })?;

        if let Some(hover) = result {
            match hover.contents {
                lsp_types::HoverContents::Scalar(marked_string) => match marked_string {
                    lsp_types::MarkedString::String(s) => Ok(Some(s)),
                    lsp_types::MarkedString::LanguageString(ls) => Ok(Some(ls.value)),
                },
                lsp_types::HoverContents::Array(arr) => {
                    let mut s = String::new();
                    for item in arr {
                        match item {
                            lsp_types::MarkedString::String(v) => s.push_str(&v),
                            lsp_types::MarkedString::LanguageString(ls) => s.push_str(&ls.value),
                        }
                        s.push('\n');
                    }
                    Ok(Some(s))
                }
                lsp_types::HoverContents::Markup(markup) => Ok(Some(markup.value)),
            }
        } else {
            Ok(None)
        }
    }

    pub fn rename(
        &self,
        relative_path: String,
        line: u32,
        character: u32,
        new_name: String,
    ) -> Result<Option<RenameResult>> {
        let client = self.get_client()?;
        let uri = self.resolve_uri(&relative_path)?;

        let result = self.runtime.block_on(async move {
            let params = RenameParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position { line, character },
                },
                new_name,
                work_done_progress_params: Default::default(),
            };
            client.send_request::<Rename>(params).await
        })?;

        if let Some(edit) = result {
            if let Some(changes) = edit.changes {
                let mut changes_map = HashMap::new();
                for (uri, edits) in changes {
                    let text_edits: Vec<TextEdit> = edits
                        .into_iter()
                        .map(|te| TextEdit {
                            new_text: te.new_text,
                            range: RangeInfo {
                                start: PositionInfo {
                                    line: te.range.start.line,
                                    character: te.range.start.character,
                                },
                                end: PositionInfo {
                                    line: te.range.end.line,
                                    character: te.range.end.character,
                                },
                            },
                        })
                        .collect();
                    changes_map.insert(uri.to_string(), text_edits);
                }
                Ok(Some(RenameResult {
                    changes: changes_map,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn index_file(&self, relative_path: String) -> Result<()> {
        let client = self.get_client()?;
        let graph = self.symbol_graph.clone();
        let uri = self.resolve_uri(&relative_path)?;

        self.runtime.block_on(async move {
            let params = lsp_types::DocumentSymbolParams {
                text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
                partial_result_params: Default::default(),
                work_done_progress_params: Default::default(),
            };

            match client.send_request::<DocumentSymbolRequest>(params).await {
                Ok(Some(DocumentSymbolResponse::Nested(symbols))) => {
                    graph.insert_document_symbols(&uri, symbols);
                }
                Ok(Some(DocumentSymbolResponse::Flat(_symbols))) => {
                    tracing::warn!(
                        "Received flat symbols for {}, indexing not fully supported yet",
                        uri
                    );
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::error!("Failed to get symbols for {}: {}", uri, e);
                }
            }
        });

        Ok(())
    }

    pub fn find_symbol(&self, query: String) -> Result<Vec<SymbolInfo>> {
        let symbols = self.symbol_graph.search(&query);
        let mut out = Vec::new();
        for sym in symbols {
            out.push(self.symbol_to_info(&sym)?);
        }
        Ok(out)
    }

    pub fn get_document_symbols(&self, relative_path: String) -> Result<Vec<SymbolInfo>> {
        let uri = self.resolve_uri(&relative_path)?;
        if let Some(symbols) = self.symbol_graph.get_file_symbols(&uri) {
            let mut out = Vec::new();
            for sym in symbols {
                out.push(self.symbol_to_info(&sym)?);
            }
            Ok(out)
        } else {
            // Not in graph? Try indexing it now?
            self.index_file(relative_path.clone())?;
            // Try getting again
            if let Some(symbols) = self.symbol_graph.get_file_symbols(&uri) {
                let mut out = Vec::new();
                for sym in symbols {
                    out.push(self.symbol_to_info(&sym)?);
                }
                Ok(out)
            } else {
                Ok(vec![])
            }
        }
    }

    // Helper methods
    fn get_client(&self) -> Result<Arc<LspClient>> {
        self.lsp_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("LSP not started"))
            .map(|c| c.clone())
    }

    fn resolve_uri(&self, relative_path: &str) -> Result<Url> {
        self.root_uri
            .join(relative_path)
            .map_err(|e| anyhow::anyhow!("Invalid URI: {}", e))
    }

    fn location_to_info(&self, loc: &lsp_types::Location) -> LocationInfo {
        LocationInfo {
            uri: loc.uri.to_string(),
            range: RangeInfo {
                start: PositionInfo {
                    line: loc.range.start.line,
                    character: loc.range.start.character,
                },
                end: PositionInfo {
                    line: loc.range.end.line,
                    character: loc.range.end.character,
                },
            },
        }
    }

    fn symbol_to_info(&self, node: &SymbolNode) -> Result<SymbolInfo> {
        let kind_val = serde_json::to_value(node.kind)?;
        let kind_int = kind_val.as_i64().unwrap_or(0) as i32;

        let range = RangeInfo {
            start: PositionInfo {
                line: node.range.start.line,
                character: node.range.start.character,
            },
            end: PositionInfo {
                line: node.range.end.line,
                character: node.range.end.character,
            },
        };

        let selection_range = RangeInfo {
            start: PositionInfo {
                line: node.selection_range.start.line,
                character: node.selection_range.start.character,
            },
            end: PositionInfo {
                line: node.selection_range.end.line,
                character: node.selection_range.end.character,
            },
        };

        let mut children = Vec::new();
        for child in &node.children {
            children.push(self.symbol_to_info(child)?);
        }

        Ok(SymbolInfo {
            name: node.name.clone(),
            kind: kind_int,
            detail: node.detail.clone(),
            uri: node.uri.to_string(),
            range: range.clone(),
            selection_range,
            children,
            location: LocationInfo {
                uri: node.uri.to_string(),
                range,
            },
        })
    }
}
