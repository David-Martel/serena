//! Symbol operation tools for Serena MCP server
//!
//! These tools wrap the LSP client to provide semantic code navigation
//! and editing capabilities.

use lsp_types::{
    DocumentSymbolParams, DocumentSymbolResponse, Position, Range, ReferenceContext,
    ReferenceParams, RenameParams, TextDocumentIdentifier, TextDocumentPositionParams,
    WorkspaceEdit, WorkspaceSymbolParams,
};
use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::mcp_2025_06_18::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{CallToolResult, TextContent};

use super::services::SymbolService;
use super::utils::truncate_output;

// ============================================================================
// get_symbols_overview
// ============================================================================

#[mcp_tool(
    name = "get_symbols_overview",
    description = "Get a high-level overview of code symbols in a file. Returns top-level symbols (classes, functions, etc.) with optional depth for children.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct GetSymbolsOverviewTool {
    /// The relative path to the file to analyze.
    pub relative_path: String,

    /// Depth of descendants to retrieve (0 = top-level only, 1 = immediate children).
    #[serde(default)]
    pub depth: u64,

    /// Maximum characters to return. -1 for unlimited. Default: -1
    #[serde(default = "default_max_chars")]
    pub max_answer_chars: i32,
}

fn default_max_chars() -> i32 {
    -1
}

impl GetSymbolsOverviewTool {
    pub async fn run_tool(self, service: &SymbolService) -> Result<CallToolResult, CallToolError> {
        let client = service
            .get_client()
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let file_path = service.project_root().join(&self.relative_path);
        let uri = lsp_types::Url::from_file_path(&file_path).map_err(|_| {
            CallToolError::from_message(format!("Invalid path: {}", self.relative_path))
        })?;

        // Request document symbols from LSP
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let response: Option<DocumentSymbolResponse> = client
            .send_request::<lsp_types::request::DocumentSymbolRequest>(params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let result = match response {
            Some(DocumentSymbolResponse::Nested(symbols)) => {
                format_symbols_overview(&symbols, self.depth as usize)
            }
            Some(DocumentSymbolResponse::Flat(symbols)) => format_flat_symbols(&symbols),
            None => "No symbols found".to_string(),
        };

        let final_result = truncate_output(result, self.max_answer_chars);

        Ok(CallToolResult::text_content(vec![TextContent::from(
            final_result,
        )]))
    }
}

fn format_symbols_overview(symbols: &[lsp_types::DocumentSymbol], max_depth: usize) -> String {
    fn format_symbol(
        symbol: &lsp_types::DocumentSymbol,
        depth: usize,
        max_depth: usize,
        indent: usize,
    ) -> String {
        let kind_str = format!("{:?}", symbol.kind);
        let range = format!(
            "{}:{}-{}:{}",
            symbol.range.start.line + 1,
            symbol.range.start.character,
            symbol.range.end.line + 1,
            symbol.range.end.character
        );

        let indent_str = "  ".repeat(indent);
        let mut result = format!("{}{} {} [{}]\n", indent_str, kind_str, symbol.name, range);

        if depth < max_depth {
            if let Some(children) = &symbol.children {
                for child in children {
                    result.push_str(&format_symbol(child, depth + 1, max_depth, indent + 1));
                }
            }
        }

        result
    }

    let mut result = String::new();
    for symbol in symbols {
        result.push_str(&format_symbol(symbol, 0, max_depth, 0));
    }

    if result.is_empty() {
        "No symbols found".to_string()
    } else {
        result
    }
}

fn format_flat_symbols(symbols: &[lsp_types::SymbolInformation]) -> String {
    let mut result = String::new();
    for symbol in symbols {
        let kind_str = format!("{:?}", symbol.kind);
        let loc = &symbol.location;
        let range = format!(
            "{}:{}-{}:{}",
            loc.range.start.line + 1,
            loc.range.start.character,
            loc.range.end.line + 1,
            loc.range.end.character
        );
        result.push_str(&format!("{} {} [{}]\n", kind_str, symbol.name, range));
    }
    if result.is_empty() {
        "No symbols found".to_string()
    } else {
        result
    }
}

// ============================================================================
// find_symbol
// ============================================================================

#[mcp_tool(
    name = "find_symbol",
    description = "Find symbols matching a name path pattern. Supports simple names, relative paths (class/method), and absolute paths (/class/method).",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct FindSymbolTool {
    /// The name path pattern to search for (e.g., "method", "Class/method", "/Class/method").
    pub name_path_pattern: String,

    /// Optional file or directory to restrict search.
    #[serde(default)]
    pub relative_path: Option<String>,

    /// Depth of descendants to retrieve.
    #[serde(default)]
    pub depth: u64,

    /// Whether to include the symbol's source code body.
    #[serde(default)]
    pub include_body: bool,

    /// Use substring matching for the last element.
    #[serde(default)]
    pub substring_matching: bool,

    /// Maximum characters to return. -1 for unlimited. Default: -1
    #[serde(default = "default_max_chars")]
    pub max_answer_chars: i32,
}

impl FindSymbolTool {
    pub async fn run_tool(self, service: &SymbolService) -> Result<CallToolResult, CallToolError> {
        let client = service
            .get_client()
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        // Parse the name path pattern
        let parts: Vec<&str> = self
            .name_path_pattern
            .trim_start_matches('/')
            .split('/')
            .collect();
        let search_name = parts.last().copied().unwrap_or("");

        // Get the search path
        let search_path = match &self.relative_path {
            Some(p) if !p.is_empty() => service.project_root().join(p),
            _ => service.project_root().to_path_buf(),
        };

        // Use workspace symbol search
        let params = WorkspaceSymbolParams {
            query: search_name.to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let response: Option<lsp_types::WorkspaceSymbolResponse> = client
            .send_request::<lsp_types::request::WorkspaceSymbolRequest>(params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let symbols = match response {
            Some(lsp_types::WorkspaceSymbolResponse::Flat(s)) => s,
            Some(lsp_types::WorkspaceSymbolResponse::Nested(s)) => {
                // Convert nested to flat representation
                s.into_iter()
                    .map(|ws| lsp_types::SymbolInformation {
                        name: ws.name,
                        kind: ws.kind,
                        tags: ws.tags,
                        deprecated: None,
                        location: match ws.location {
                            lsp_types::OneOf::Left(loc) => loc,
                            lsp_types::OneOf::Right(loc) => lsp_types::Location {
                                uri: loc.uri,
                                range: Range::default(),
                            },
                        },
                        container_name: ws.container_name,
                    })
                    .collect()
            }
            None => Vec::new(),
        };

        // Filter symbols by path pattern and location
        let mut results = Vec::new();
        for symbol in symbols {
            if let Ok(path) = symbol.location.uri.to_file_path() {
                // Check if within search path
                if !path.starts_with(&search_path) {
                    continue;
                }

                // Match name pattern
                let name_matches = if self.substring_matching {
                    symbol.name.contains(search_name)
                } else {
                    symbol.name == search_name
                };

                if name_matches {
                    let rel_path = path
                        .strip_prefix(service.project_root())
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");

                    let line = symbol.location.range.start.line;
                    let char_pos = symbol.location.range.start.character;

                    let mut entry = serde_json::json!({
                        "name": symbol.name,
                        "kind": format!("{:?}", symbol.kind),
                        "path": rel_path,
                        "line": line + 1,
                        "character": char_pos
                    });

                    if self.include_body {
                        // Read the file and extract the symbol body
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let lines: Vec<&str> = content.lines().collect();
                            let start_line = line as usize;
                            // Simple heuristic: get next 20 lines or until blank line
                            let end_line = (start_line + 20).min(lines.len());
                            let body: String = lines[start_line..end_line].join("\n");
                            entry["body"] = serde_json::Value::String(body);
                        }
                    }

                    results.push(entry);
                }
            }
        }

        let json_result = serde_json::to_string_pretty(&results).map_err(|e| {
            CallToolError::from_message(format!("Failed to serialize results: {}", e))
        })?;

        let final_result = truncate_output(json_result, self.max_answer_chars);

        Ok(CallToolResult::text_content(vec![TextContent::from(
            final_result,
        )]))
    }
}

// ============================================================================
// find_referencing_symbols
// ============================================================================

#[mcp_tool(
    name = "find_referencing_symbols",
    description = "Find all references to a symbol at the given location.",
    destructive_hint = false,
    idempotent_hint = true,
    read_only_hint = true
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct FindReferencingSymbolsTool {
    /// Name path of the symbol to find references for.
    pub name_path: String,

    /// The relative path to the file containing the symbol.
    pub relative_path: String,

    /// Maximum characters to return. -1 for unlimited. Default: -1
    #[serde(default = "default_max_chars")]
    pub max_answer_chars: i32,
}

impl FindReferencingSymbolsTool {
    pub async fn run_tool(self, service: &SymbolService) -> Result<CallToolResult, CallToolError> {
        let client = service
            .get_client()
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let file_path = service.project_root().join(&self.relative_path);
        let uri = lsp_types::Url::from_file_path(&file_path).map_err(|_| {
            CallToolError::from_message(format!("Invalid path: {}", self.relative_path))
        })?;

        // First find the symbol to get its position
        let doc_params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let response: Option<DocumentSymbolResponse> = client
            .send_request::<lsp_types::request::DocumentSymbolRequest>(doc_params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let symbols = match response {
            Some(DocumentSymbolResponse::Nested(s)) => s,
            _ => Vec::new(),
        };

        let target_name = self.name_path.split('/').last().unwrap_or(&self.name_path);

        // Find the symbol position
        let position = find_symbol_position(&symbols, target_name).ok_or_else(|| {
            CallToolError::from_message(format!("Symbol not found: {}", self.name_path))
        })?;

        // Get references
        let ref_params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };

        let references: Option<Vec<lsp_types::Location>> = client
            .send_request::<lsp_types::request::References>(ref_params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let mut results = Vec::new();
        for loc in references.unwrap_or_default() {
            if let Ok(path) = loc.uri.to_file_path() {
                let rel_path = path
                    .strip_prefix(service.project_root())
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");

                // Get context around the reference
                let context = if let Ok(content) = std::fs::read_to_string(&path) {
                    let lines: Vec<&str> = content.lines().collect();
                    let line_num = loc.range.start.line as usize;
                    let start = line_num.saturating_sub(1);
                    let end = (line_num + 2).min(lines.len());
                    lines[start..end]
                        .iter()
                        .enumerate()
                        .map(|(i, l)| format!("{}: {}", start + i + 1, l))
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    String::new()
                };

                results.push(serde_json::json!({
                    "path": rel_path,
                    "line": loc.range.start.line + 1,
                    "character": loc.range.start.character,
                    "context": context
                }));
            }
        }

        let json_result = serde_json::to_string_pretty(&results).map_err(|e| {
            CallToolError::from_message(format!("Failed to serialize references: {}", e))
        })?;

        let final_result = truncate_output(json_result, self.max_answer_chars);

        Ok(CallToolResult::text_content(vec![TextContent::from(
            final_result,
        )]))
    }
}

fn find_symbol_position(symbols: &[lsp_types::DocumentSymbol], name: &str) -> Option<Position> {
    for symbol in symbols {
        if symbol.name == name {
            return Some(symbol.selection_range.start);
        }
        if let Some(children) = &symbol.children {
            if let Some(pos) = find_symbol_position(children, name) {
                return Some(pos);
            }
        }
    }
    None
}

// ============================================================================
// replace_symbol_body
// ============================================================================

#[mcp_tool(
    name = "replace_symbol_body",
    description = "Replace the entire body of a symbol with new content.",
    destructive_hint = true,
    idempotent_hint = false,
    read_only_hint = false
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ReplaceSymbolBodyTool {
    /// Name path of the symbol to replace.
    pub name_path: String,

    /// The relative path to the file containing the symbol.
    pub relative_path: String,

    /// The new body content for the symbol.
    pub body: String,
}

impl ReplaceSymbolBodyTool {
    pub async fn run_tool(self, service: &SymbolService) -> Result<CallToolResult, CallToolError> {
        let client = service
            .get_client()
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let file_path = service.project_root().join(&self.relative_path);
        let uri = lsp_types::Url::from_file_path(&file_path).map_err(|_| {
            CallToolError::from_message(format!("Invalid path: {}", self.relative_path))
        })?;

        // Get document symbols to find the target
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let response: Option<DocumentSymbolResponse> = client
            .send_request::<lsp_types::request::DocumentSymbolRequest>(params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let symbols = match response {
            Some(DocumentSymbolResponse::Nested(s)) => s,
            _ => Vec::new(),
        };

        let target_name = self.name_path.split('/').last().unwrap_or(&self.name_path);

        // Find the symbol's range
        let range = find_symbol_range(&symbols, target_name).ok_or_else(|| {
            CallToolError::from_message(format!("Symbol not found: {}", self.name_path))
        })?;

        // Read the file
        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let lines: Vec<&str> = content.lines().collect();

        // Replace the symbol body
        let mut new_content = String::new();

        // Lines before the symbol
        for line in &lines[..range.start.line as usize] {
            new_content.push_str(line);
            new_content.push('\n');
        }

        // New body
        new_content.push_str(&self.body);
        if !self.body.ends_with('\n') {
            new_content.push('\n');
        }

        // Lines after the symbol
        let end_line = (range.end.line as usize + 1).min(lines.len());
        for line in &lines[end_line..] {
            new_content.push_str(line);
            new_content.push('\n');
        }

        // Write back
        std::fs::write(&file_path, &new_content)
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format!(
                "Successfully replaced symbol '{}' in {}",
                self.name_path, self.relative_path
            ),
        )]))
    }
}

fn find_symbol_range(symbols: &[lsp_types::DocumentSymbol], name: &str) -> Option<Range> {
    for symbol in symbols {
        if symbol.name == name {
            return Some(symbol.range);
        }
        if let Some(children) = &symbol.children {
            if let Some(range) = find_symbol_range(children, name) {
                return Some(range);
            }
        }
    }
    None
}

// ============================================================================
// rename_symbol
// ============================================================================

#[mcp_tool(
    name = "rename_symbol",
    description = "Rename a symbol across the entire codebase using LSP rename.",
    destructive_hint = true,
    idempotent_hint = false,
    read_only_hint = false
)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, JsonSchema)]
pub struct RenameSymbolTool {
    /// Name path of the symbol to rename.
    pub name_path: String,

    /// The relative path to the file containing the symbol.
    pub relative_path: String,

    /// The new name for the symbol.
    pub new_name: String,
}

impl RenameSymbolTool {
    pub async fn run_tool(self, service: &SymbolService) -> Result<CallToolResult, CallToolError> {
        let client = service
            .get_client()
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let file_path = service.project_root().join(&self.relative_path);
        let uri = lsp_types::Url::from_file_path(&file_path).map_err(|_| {
            CallToolError::from_message(format!("Invalid path: {}", self.relative_path))
        })?;

        // Get document symbols to find the target
        let doc_params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let response: Option<DocumentSymbolResponse> = client
            .send_request::<lsp_types::request::DocumentSymbolRequest>(doc_params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let symbols = match response {
            Some(DocumentSymbolResponse::Nested(s)) => s,
            _ => Vec::new(),
        };

        let target_name = self.name_path.split('/').last().unwrap_or(&self.name_path);

        // Find the symbol position
        let position = find_symbol_position(&symbols, target_name).ok_or_else(|| {
            CallToolError::from_message(format!("Symbol not found: {}", self.name_path))
        })?;

        // Perform rename via LSP
        let rename_params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            new_name: self.new_name.clone(),
            work_done_progress_params: Default::default(),
        };

        let edit: Option<WorkspaceEdit> = client
            .send_request::<lsp_types::request::Rename>(rename_params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        // Apply the workspace edit
        let mut files_changed = 0;
        if let Some(edit) = edit {
            if let Some(changes) = edit.changes {
                for (uri, edits) in changes {
                    if let Ok(path) = uri.to_file_path() {
                        let content = std::fs::read_to_string(&path)
                            .map_err(|e| CallToolError::from_message(e.to_string()))?;

                        let new_content = apply_text_edits(&content, &edits);

                        std::fs::write(&path, &new_content)
                            .map_err(|e| CallToolError::from_message(e.to_string()))?;

                        files_changed += 1;
                    }
                }
            }
        }

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format!(
                "Renamed '{}' to '{}' in {} files",
                target_name, self.new_name, files_changed
            ),
        )]))
    }
}

fn apply_text_edits(content: &str, edits: &[lsp_types::TextEdit]) -> String {
    // Sort edits in reverse order to apply from end to start
    let mut sorted_edits: Vec<_> = edits.iter().collect();
    sorted_edits.sort_by(|a, b| {
        let line_cmp = b.range.start.line.cmp(&a.range.start.line);
        if line_cmp == std::cmp::Ordering::Equal {
            b.range.start.character.cmp(&a.range.start.character)
        } else {
            line_cmp
        }
    });

    let mut result = content.to_string();
    let lines: Vec<&str> = content.lines().collect();

    for edit in sorted_edits {
        let start_line = edit.range.start.line as usize;
        let end_line = edit.range.end.line as usize;

        if start_line >= lines.len() {
            continue;
        }

        // Calculate byte offsets
        let mut start_offset = 0;
        for (i, line) in lines.iter().enumerate() {
            if i == start_line {
                start_offset += edit.range.start.character as usize;
                break;
            }
            start_offset += line.len() + 1; // +1 for newline
        }

        let mut end_offset = 0;
        for (i, line) in lines.iter().enumerate() {
            if i == end_line {
                end_offset += edit.range.end.character as usize;
                break;
            }
            end_offset += line.len() + 1;
        }

        if start_offset <= result.len() && end_offset <= result.len() {
            result = format!(
                "{}{}{}",
                &result[..start_offset],
                &edit.new_text,
                &result[end_offset..]
            );
        }
    }

    result
}
