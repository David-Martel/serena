//! Symbol operation tools for Serena MCP server
//!
//! These tools wrap the LSP client to provide semantic code navigation
//! and editing capabilities.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use serena_core::{LanguageServer, Range, SerenaError, SymbolInfo, Tool, ToolError, ToolResult};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

// ============================================================================
// Helper Functions
// ============================================================================

/// Truncate output to specified max characters
fn truncate_output(text: String, max_chars: i32) -> String {
    if max_chars < 0 {
        text
    } else {
        let max = max_chars as usize;
        if text.len() > max {
            format!("{}... (truncated)", &text[..max])
        } else {
            text
        }
    }
}

/// Find symbol position by name in a list of symbols
fn find_symbol_position(symbols: &[SymbolInfo], name: &str) -> Option<(u32, u32)> {
    for symbol in symbols {
        if symbol.name == name {
            return Some((symbol.location.range.start.line, symbol.location.range.start.character));
        }
        // Recursively search in children
        if !symbol.children.is_empty() {
            if let Some(pos) = find_symbol_position(&symbol.children, name) {
                return Some(pos);
            }
        }
    }
    None
}

/// Find symbol range by name in a list of symbols
fn find_symbol_range(symbols: &[SymbolInfo], name: &str) -> Option<Range> {
    for symbol in symbols {
        if symbol.name == name {
            return Some(symbol.location.range);
        }
        // Recursively search in children
        if !symbol.children.is_empty() {
            if let Some(range) = find_symbol_range(&symbol.children, name) {
                return Some(range);
            }
        }
    }
    None
}

/// Default value for max_answer_chars
fn default_max_chars() -> i32 {
    -1
}

// ============================================================================
// get_symbols_overview Tool
// ============================================================================

/// Tool for getting a high-level overview of code symbols in a file
pub struct GetSymbolsOverviewTool {
    project_root: PathBuf,
    lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
}

#[derive(Debug, Deserialize)]
struct GetSymbolsOverviewParams {
    relative_path: String,
    #[serde(default)]
    depth: u64,
    #[serde(default = "default_max_chars")]
    max_answer_chars: i32,
}

impl GetSymbolsOverviewTool {
    /// Create a new GetSymbolsOverviewTool
    pub fn new(
        project_root: impl Into<PathBuf>,
        lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
    ) -> Self {
        Self {
            project_root: project_root.into(),
            lsp_client,
        }
    }

    fn format_symbols(&self, symbols: &[SymbolInfo], max_depth: u64) -> String {
        fn format_symbol(symbol: &SymbolInfo, depth: u64, max_depth: u64, indent: usize) -> String {
            let kind_str = format!("{:?}", symbol.kind);
            let range = &symbol.location.range;
            let range_str = format!(
                "{}:{}-{}:{}",
                range.start.line + 1,
                range.start.character,
                range.end.line + 1,
                range.end.character
            );

            let indent_str = "  ".repeat(indent);
            let mut result = format!("{}{} {} [{}]\n", indent_str, kind_str, symbol.name, range_str);

            if depth < max_depth && !symbol.children.is_empty() {
                for child in &symbol.children {
                    result.push_str(&format_symbol(child, depth + 1, max_depth, indent + 1));
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
}

#[async_trait]
impl Tool for GetSymbolsOverviewTool {
    fn name(&self) -> &str {
        "get_symbols_overview"
    }

    fn description(&self) -> &str {
        "Get a high-level overview of code symbols in a file. Returns top-level symbols \
        (classes, functions, etc.) with optional depth for children."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file to analyze"
                },
                "depth": {
                    "type": "integer",
                    "description": "Depth of descendants to retrieve (0 = top-level only)",
                    "default": 0
                },
                "max_answer_chars": {
                    "type": "integer",
                    "description": "Maximum characters to return. -1 for unlimited",
                    "default": -1
                }
            },
            "required": ["relative_path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: GetSymbolsOverviewParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let file_path = self.project_root.join(&params.relative_path);
        let uri = format!("file://{}", file_path.display());

        debug!("Getting symbols overview for: {}", uri);

        // Get document symbols from LSP
        let client = self.lsp_client.read().await;
        let text_document = lsp_types::TextDocumentIdentifier {
            uri: uri.parse::<lsp_types::Uri>().map_err(|e| {
                SerenaError::InvalidParameter(format!("Invalid URI: {}", e))
            })?,
        };

        let symbols = client.document_symbols(text_document).await.map_err(|e| {
            SerenaError::Tool(ToolError::ExecutionFailed(format!(
                "LSP document symbols failed: {}",
                e
            )))
        })?;

        let result = self.format_symbols(&symbols, params.depth);
        let final_result = truncate_output(result, params.max_answer_chars);

        Ok(ToolResult::success(json!({
            "symbols": final_result
        })))
    }

    fn tags(&self) -> Vec<String> {
        vec!["symbol".to_string(), "read".to_string(), "lsp".to_string()]
    }
}

// ============================================================================
// find_symbol Tool
// ============================================================================

/// Tool for finding symbols matching a name path pattern
pub struct FindSymbolTool {
    project_root: PathBuf,
    lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
}

#[derive(Debug, Deserialize)]
struct FindSymbolParams {
    name_path_pattern: String,
    #[serde(default)]
    relative_path: Option<String>,
    #[serde(default)]
    #[allow(dead_code)] // May be used for future recursive depth limiting
    depth: u64,
    #[serde(default)]
    include_body: bool,
    #[serde(default)]
    substring_matching: bool,
    #[serde(default = "default_max_chars")]
    max_answer_chars: i32,
}

impl FindSymbolTool {
    /// Create a new FindSymbolTool
    pub fn new(
        project_root: impl Into<PathBuf>,
        lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
    ) -> Self {
        Self {
            project_root: project_root.into(),
            lsp_client,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn find_matching_symbols(
        &self,
        symbols: &[SymbolInfo],
        search_name: &str,
        substring_matching: bool,
        include_body: bool,
        file_path: &std::path::Path,
    ) -> Vec<Value> {
        let mut results = Vec::new();

        for symbol in symbols {
            let name_matches = if substring_matching {
                symbol.name.contains(search_name)
            } else {
                symbol.name == search_name
            };

            if name_matches {
                let range = &symbol.location.range;
                let mut entry = json!({
                    "name": symbol.name,
                    "kind": format!("{:?}", symbol.kind),
                    "line": range.start.line + 1,
                    "character": range.start.character
                });

                if include_body {
                    // Read file and extract symbol body
                    if let Ok(content) = std::fs::read_to_string(file_path) {
                        let lines: Vec<&str> = content.lines().collect();
                        let start_line = range.start.line as usize;
                        let end_line = (range.end.line as usize + 1).min(lines.len());
                        if start_line < lines.len() {
                            let body: String = lines[start_line..end_line].join("\n");
                            entry["body"] = json!(body);
                        }
                    }
                }

                results.push(entry);
            }

            // Recursively search in children
            if !symbol.children.is_empty() {
                results.extend(self.find_matching_symbols(
                    &symbol.children,
                    search_name,
                    substring_matching,
                    include_body,
                    file_path,
                ));
            }
        }

        results
    }
}

#[async_trait]
impl Tool for FindSymbolTool {
    fn name(&self) -> &str {
        "find_symbol"
    }

    fn description(&self) -> &str {
        "Find symbols matching a name path pattern. Supports simple names, relative paths \
        (class/method), and absolute paths (/class/method)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name_path_pattern": {
                    "type": "string",
                    "description": "The name path pattern to search for (e.g., 'method', 'Class/method')"
                },
                "relative_path": {
                    "type": "string",
                    "description": "Optional file or directory to restrict search"
                },
                "depth": {
                    "type": "integer",
                    "description": "Depth of descendants to retrieve",
                    "default": 0
                },
                "include_body": {
                    "type": "boolean",
                    "description": "Whether to include the symbol's source code body",
                    "default": false
                },
                "substring_matching": {
                    "type": "boolean",
                    "description": "Use substring matching for the last element",
                    "default": false
                },
                "max_answer_chars": {
                    "type": "integer",
                    "description": "Maximum characters to return. -1 for unlimited",
                    "default": -1
                }
            },
            "required": ["name_path_pattern"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: FindSymbolParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let parts: Vec<&str> = params
            .name_path_pattern
            .trim_start_matches('/')
            .split('/')
            .collect();
        let search_name = parts.last().copied().unwrap_or("");

        debug!("Finding symbol: {}", search_name);

        let mut results = Vec::new();

        // If a specific file is provided, search within it
        if let Some(ref rel_path) = params.relative_path {
            let file_path = self.project_root.join(rel_path);
            if file_path.is_file() {
                let uri = format!("file://{}", file_path.display());
                let client = self.lsp_client.read().await;
                let text_document = lsp_types::TextDocumentIdentifier {
                    uri: uri.parse::<lsp_types::Uri>().map_err(|e| {
                        SerenaError::InvalidParameter(format!("Invalid URI: {}", e))
                    })?,
                };

                let symbols = client.document_symbols(text_document).await.map_err(|e| {
                    SerenaError::Tool(ToolError::ExecutionFailed(format!(
                        "LSP document symbols failed: {}",
                        e
                    )))
                })?;

                results = self.find_matching_symbols(
                    &symbols,
                    search_name,
                    params.substring_matching,
                    params.include_body,
                    &file_path,
                );

                // Add path to each result
                for entry in &mut results {
                    if let Some(obj) = entry.as_object_mut() {
                        obj.insert("path".to_string(), json!(rel_path));
                    }
                }
            }
        }

        let json_result = serde_json::to_string_pretty(&results)
            .map_err(|e| SerenaError::InvalidParameter(format!("Serialization failed: {}", e)))?;

        let final_result = truncate_output(json_result, params.max_answer_chars);

        Ok(ToolResult::success(json!({
            "matches": serde_json::from_str::<Value>(&final_result).unwrap_or(json!([])),
            "count": results.len()
        })))
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "symbol".to_string(),
            "search".to_string(),
            "lsp".to_string(),
        ]
    }
}

// ============================================================================
// find_referencing_symbols Tool
// ============================================================================

/// Tool for finding all references to a symbol
pub struct FindReferencingSymbolsTool {
    project_root: PathBuf,
    lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
}

#[derive(Debug, Deserialize)]
struct FindReferencingSymbolsParams {
    name_path: String,
    relative_path: String,
    #[serde(default = "default_max_chars")]
    max_answer_chars: i32,
}

impl FindReferencingSymbolsTool {
    /// Create a new FindReferencingSymbolsTool
    pub fn new(
        project_root: impl Into<PathBuf>,
        lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
    ) -> Self {
        Self {
            project_root: project_root.into(),
            lsp_client,
        }
    }
}

#[async_trait]
impl Tool for FindReferencingSymbolsTool {
    fn name(&self) -> &str {
        "find_referencing_symbols"
    }

    fn description(&self) -> &str {
        "Find all references to a symbol at the given location."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name_path": {
                    "type": "string",
                    "description": "Name path of the symbol to find references for"
                },
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file containing the symbol"
                },
                "max_answer_chars": {
                    "type": "integer",
                    "description": "Maximum characters to return. -1 for unlimited",
                    "default": -1
                }
            },
            "required": ["name_path", "relative_path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: FindReferencingSymbolsParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let file_path = self.project_root.join(&params.relative_path);
        let uri = format!("file://{}", file_path.display());

        debug!(
            "Finding references for {} in {}",
            params.name_path, params.relative_path
        );

        let client = self.lsp_client.read().await;

        // First get document symbols to find the target position
        let text_document = lsp_types::TextDocumentIdentifier {
            uri: uri.parse::<lsp_types::Uri>().map_err(|e| {
                SerenaError::InvalidParameter(format!("Invalid URI: {}", e))
            })?,
        };

        let symbols = client.document_symbols(text_document.clone()).await.map_err(|e| {
            SerenaError::Tool(ToolError::ExecutionFailed(format!(
                "LSP document symbols failed: {}",
                e
            )))
        })?;

        let target_name = params.name_path.split('/').next_back().unwrap_or(&params.name_path);

        // Find matching symbol to get its position
        let (line, character) = find_symbol_position(&symbols, target_name).ok_or_else(|| {
            SerenaError::NotFound(format!("Symbol not found: {}", params.name_path))
        })?;

        let position = lsp_types::Position { line, character };

        // Get references using LSP
        let text_doc_pos = lsp_types::TextDocumentPositionParams {
            text_document,
            position,
        };

        let references = client.find_references(text_doc_pos).await.map_err(|e| {
            SerenaError::Tool(ToolError::ExecutionFailed(format!(
                "LSP find references failed: {}",
                e
            )))
        })?;

        let mut results = Vec::new();
        for loc in references {
            // Parse the URI to extract the file path
            let uri_str = loc.uri.to_string();
            let path_str = uri_str.strip_prefix("file://").unwrap_or(&uri_str);
            let path = PathBuf::from(path_str);

            let rel_path = path
                .strip_prefix(&self.project_root)
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

            results.push(json!({
                "path": rel_path,
                "line": loc.range.start.line + 1,
                "character": loc.range.start.character,
                "context": context
            }));
        }

        let json_result = serde_json::to_string_pretty(&results)
            .map_err(|e| SerenaError::InvalidParameter(format!("Serialization failed: {}", e)))?;

        let final_result = truncate_output(json_result, params.max_answer_chars);

        Ok(ToolResult::success(json!({
            "references": serde_json::from_str::<Value>(&final_result).unwrap_or(json!([])),
            "count": results.len()
        })))
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "symbol".to_string(),
            "references".to_string(),
            "lsp".to_string(),
        ]
    }
}

// ============================================================================
// replace_symbol_body Tool
// ============================================================================

/// Tool for replacing the body of a symbol
pub struct ReplaceSymbolBodyTool {
    project_root: PathBuf,
    lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
}

#[derive(Debug, Deserialize)]
struct ReplaceSymbolBodyParams {
    name_path: String,
    relative_path: String,
    body: String,
}

impl ReplaceSymbolBodyTool {
    /// Create a new ReplaceSymbolBodyTool
    pub fn new(
        project_root: impl Into<PathBuf>,
        lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
    ) -> Self {
        Self {
            project_root: project_root.into(),
            lsp_client,
        }
    }
}

#[async_trait]
impl Tool for ReplaceSymbolBodyTool {
    fn name(&self) -> &str {
        "replace_symbol_body"
    }

    fn description(&self) -> &str {
        "Replace the entire body of a symbol with new content."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name_path": {
                    "type": "string",
                    "description": "Name path of the symbol to replace"
                },
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file containing the symbol"
                },
                "body": {
                    "type": "string",
                    "description": "The new body content for the symbol"
                }
            },
            "required": ["name_path", "relative_path", "body"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: ReplaceSymbolBodyParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let file_path = self.project_root.join(&params.relative_path);
        let uri = format!("file://{}", file_path.display());

        debug!(
            "Replacing symbol {} in {}",
            params.name_path, params.relative_path
        );

        let client = self.lsp_client.read().await;

        // Get document symbols to find the target
        let text_document = lsp_types::TextDocumentIdentifier {
            uri: uri.parse::<lsp_types::Uri>().map_err(|e| {
                SerenaError::InvalidParameter(format!("Invalid URI: {}", e))
            })?,
        };

        let symbols = client.document_symbols(text_document).await.map_err(|e| {
            SerenaError::Tool(ToolError::ExecutionFailed(format!(
                "LSP document symbols failed: {}",
                e
            )))
        })?;

        let target_name = params.name_path.split('/').next_back().unwrap_or(&params.name_path);

        // Find matching symbol to get its range
        let range = find_symbol_range(&symbols, target_name).ok_or_else(|| {
            SerenaError::NotFound(format!("Symbol not found: {}", params.name_path))
        })?;

        // Read the file
        let content = std::fs::read_to_string(&file_path)
            .map_err(SerenaError::Io)?;

        let lines: Vec<&str> = content.lines().collect();

        // Replace the symbol body
        let mut new_content = String::new();

        // Lines before the symbol
        for line in &lines[..range.start.line as usize] {
            new_content.push_str(line);
            new_content.push('\n');
        }

        // New body
        new_content.push_str(&params.body);
        if !params.body.ends_with('\n') {
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
            .map_err(SerenaError::Io)?;

        Ok(ToolResult::success(json!({
            "message": format!("Successfully replaced symbol '{}' in {}", params.name_path, params.relative_path)
        })))
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "symbol".to_string(),
            "edit".to_string(),
            "lsp".to_string(),
        ]
    }
}

// ============================================================================
// rename_symbol Tool
// ============================================================================

/// Tool for renaming a symbol across the codebase
pub struct RenameSymbolTool {
    project_root: PathBuf,
    lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
}

#[derive(Debug, Deserialize)]
struct RenameSymbolParams {
    name_path: String,
    relative_path: String,
    new_name: String,
}

impl RenameSymbolTool {
    /// Create a new RenameSymbolTool
    pub fn new(
        project_root: impl Into<PathBuf>,
        lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
    ) -> Self {
        Self {
            project_root: project_root.into(),
            lsp_client,
        }
    }
}

#[async_trait]
impl Tool for RenameSymbolTool {
    fn name(&self) -> &str {
        "rename_symbol"
    }

    fn description(&self) -> &str {
        "Rename a symbol across the entire codebase using LSP rename."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name_path": {
                    "type": "string",
                    "description": "Name path of the symbol to rename"
                },
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file containing the symbol"
                },
                "new_name": {
                    "type": "string",
                    "description": "The new name for the symbol"
                }
            },
            "required": ["name_path", "relative_path", "new_name"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: RenameSymbolParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let file_path = self.project_root.join(&params.relative_path);
        let uri = format!("file://{}", file_path.display());

        debug!(
            "Renaming symbol {} to {} in {}",
            params.name_path, params.new_name, params.relative_path
        );

        let client = self.lsp_client.read().await;

        // Get document symbols to find the target position
        let text_document = lsp_types::TextDocumentIdentifier {
            uri: uri.parse::<lsp_types::Uri>().map_err(|e| {
                SerenaError::InvalidParameter(format!("Invalid URI: {}", e))
            })?,
        };

        let symbols = client.document_symbols(text_document).await.map_err(|e| {
            SerenaError::Tool(ToolError::ExecutionFailed(format!(
                "LSP document symbols failed: {}",
                e
            )))
        })?;

        let target_name = params.name_path.split('/').next_back().unwrap_or(&params.name_path);

        // Find matching symbol to get its position
        let (line, character) = find_symbol_position(&symbols, target_name).ok_or_else(|| {
            SerenaError::NotFound(format!("Symbol not found: {}", params.name_path))
        })?;

        // Perform rename via LSP
        let rename_params = lsp_types::RenameParams {
            text_document_position: lsp_types::TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: uri.parse::<lsp_types::Uri>().map_err(|e| {
                        SerenaError::InvalidParameter(format!("Invalid URI: {}", e))
                    })?,
                },
                position: lsp_types::Position { line, character },
            },
            new_name: params.new_name.clone(),
            work_done_progress_params: Default::default(),
        };

        let edit = client.rename(rename_params).await.map_err(|e| {
            SerenaError::Tool(ToolError::ExecutionFailed(format!(
                "LSP rename failed: {}",
                e
            )))
        })?;

        // Apply the workspace edit
        let mut files_changed = 0;
        if let Some(changes) = edit.changes {
            for (uri, edits) in changes {
                // Extract path from URI
                let path_str = uri.to_string();
                let path_str = path_str.strip_prefix("file://").unwrap_or(&path_str);
                let path = PathBuf::from(path_str);

                let content = std::fs::read_to_string(&path)
                    .map_err(SerenaError::Io)?;

                let new_content = apply_text_edits(&content, &edits);

                std::fs::write(&path, &new_content)
                    .map_err(SerenaError::Io)?;

                files_changed += 1;
            }
        }

        Ok(ToolResult::success(json!({
            "message": format!("Renamed '{}' to '{}' in {} files", target_name, params.new_name, files_changed),
            "files_changed": files_changed
        })))
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "symbol".to_string(),
            "rename".to_string(),
            "edit".to_string(),
            "lsp".to_string(),
        ]
    }
}

/// Apply text edits to content, handling reverse order application
fn apply_text_edits(content: &str, edits: &[lsp_types::TextEdit]) -> String {
    if edits.is_empty() {
        return content.to_string();
    }

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

    // Pre-compute line start byte offsets ONCE - O(n) total instead of O(n * edits)
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(content.match_indices('\n').map(|(i, _)| i + 1))
        .collect();

    let mut result = content.to_string();

    for edit in sorted_edits {
        let start_line = edit.range.start.line as usize;
        let end_line = edit.range.end.line as usize;

        if start_line >= line_starts.len() {
            continue;
        }

        // O(1) offset calculation using pre-computed line starts
        let start_offset = line_starts
            .get(start_line)
            .map(|&s| s + edit.range.start.character as usize)
            .unwrap_or(0);

        let end_offset = line_starts
            .get(end_line)
            .map(|&s| s + edit.range.end.character as usize)
            .unwrap_or(result.len());

        if start_offset <= result.len() && end_offset <= result.len() && start_offset <= end_offset
        {
            result.replace_range(start_offset..end_offset, &edit.new_text);
        }
    }

    result
}

// ============================================================================
// insert_after_symbol Tool
// ============================================================================

/// Tool for inserting content after a symbol
pub struct InsertAfterSymbolTool {
    project_root: PathBuf,
    lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
}

#[derive(Debug, Deserialize)]
struct InsertAfterSymbolParams {
    name_path: String,
    relative_path: String,
    content: String,
}

impl InsertAfterSymbolTool {
    /// Create a new InsertAfterSymbolTool
    pub fn new(
        project_root: impl Into<PathBuf>,
        lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
    ) -> Self {
        Self {
            project_root: project_root.into(),
            lsp_client,
        }
    }
}

#[async_trait]
impl Tool for InsertAfterSymbolTool {
    fn name(&self) -> &str {
        "insert_after_symbol"
    }

    fn description(&self) -> &str {
        "Insert content after a symbol (function, class, method, etc.). \
        The content will be inserted starting on the line after the symbol ends."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name_path": {
                    "type": "string",
                    "description": "Name path of the symbol to insert after"
                },
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file containing the symbol"
                },
                "content": {
                    "type": "string",
                    "description": "The content to insert after the symbol"
                }
            },
            "required": ["name_path", "relative_path", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: InsertAfterSymbolParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let file_path = self.project_root.join(&params.relative_path);
        let uri = format!("file://{}", file_path.display());

        debug!(
            "Inserting after symbol {} in {}",
            params.name_path, params.relative_path
        );

        let client = self.lsp_client.read().await;

        // Get document symbols to find the target
        let text_document = lsp_types::TextDocumentIdentifier {
            uri: uri.parse::<lsp_types::Uri>().map_err(|e| {
                SerenaError::InvalidParameter(format!("Invalid URI: {}", e))
            })?,
        };

        let symbols = client.document_symbols(text_document).await.map_err(|e| {
            SerenaError::Tool(ToolError::ExecutionFailed(format!(
                "LSP document symbols failed: {}",
                e
            )))
        })?;

        let target_name = params.name_path.split('/').next_back().unwrap_or(&params.name_path);

        // Find matching symbol to get its range
        let range = find_symbol_range(&symbols, target_name).ok_or_else(|| {
            SerenaError::NotFound(format!("Symbol not found: {}", params.name_path))
        })?;

        // Read the file
        let content = std::fs::read_to_string(&file_path)
            .map_err(SerenaError::Io)?;

        let lines: Vec<&str> = content.lines().collect();

        // Insert after the symbol's end line
        let insert_line = (range.end.line as usize + 1).min(lines.len());

        // Build new content
        let mut new_content = String::new();

        // Lines before and including the symbol
        for line in &lines[..insert_line] {
            new_content.push_str(line);
            new_content.push('\n');
        }

        // Add empty line if needed for separation
        if !params.content.starts_with('\n') {
            new_content.push('\n');
        }

        // New content
        new_content.push_str(&params.content);
        if !params.content.ends_with('\n') {
            new_content.push('\n');
        }

        // Lines after the insert point
        for line in &lines[insert_line..] {
            new_content.push_str(line);
            new_content.push('\n');
        }

        // Write back
        std::fs::write(&file_path, &new_content)
            .map_err(SerenaError::Io)?;

        Ok(ToolResult::success(json!({
            "message": format!("Inserted content after symbol '{}' at line {} in {}",
                params.name_path, insert_line + 1, params.relative_path)
        })))
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "symbol".to_string(),
            "edit".to_string(),
            "insert".to_string(),
            "lsp".to_string(),
        ]
    }
}

// ============================================================================
// insert_before_symbol Tool
// ============================================================================

/// Tool for inserting content before a symbol
pub struct InsertBeforeSymbolTool {
    project_root: PathBuf,
    lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
}

#[derive(Debug, Deserialize)]
struct InsertBeforeSymbolParams {
    name_path: String,
    relative_path: String,
    content: String,
}

impl InsertBeforeSymbolTool {
    /// Create a new InsertBeforeSymbolTool
    pub fn new(
        project_root: impl Into<PathBuf>,
        lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
    ) -> Self {
        Self {
            project_root: project_root.into(),
            lsp_client,
        }
    }
}

#[async_trait]
impl Tool for InsertBeforeSymbolTool {
    fn name(&self) -> &str {
        "insert_before_symbol"
    }

    fn description(&self) -> &str {
        "Insert content before a symbol (function, class, method, etc.). \
        The content will be inserted starting on the line before the symbol begins."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name_path": {
                    "type": "string",
                    "description": "Name path of the symbol to insert before"
                },
                "relative_path": {
                    "type": "string",
                    "description": "The relative path to the file containing the symbol"
                },
                "content": {
                    "type": "string",
                    "description": "The content to insert before the symbol"
                }
            },
            "required": ["name_path", "relative_path", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, SerenaError> {
        let params: InsertBeforeSymbolParams = serde_json::from_value(params)
            .map_err(|e| SerenaError::InvalidParameter(e.to_string()))?;

        let file_path = self.project_root.join(&params.relative_path);
        let uri = format!("file://{}", file_path.display());

        debug!(
            "Inserting before symbol {} in {}",
            params.name_path, params.relative_path
        );

        let client = self.lsp_client.read().await;

        // Get document symbols to find the target
        let text_document = lsp_types::TextDocumentIdentifier {
            uri: uri.parse::<lsp_types::Uri>().map_err(|e| {
                SerenaError::InvalidParameter(format!("Invalid URI: {}", e))
            })?,
        };

        let symbols = client.document_symbols(text_document).await.map_err(|e| {
            SerenaError::Tool(ToolError::ExecutionFailed(format!(
                "LSP document symbols failed: {}",
                e
            )))
        })?;

        let target_name = params.name_path.split('/').next_back().unwrap_or(&params.name_path);

        // Find matching symbol to get its range
        let range = find_symbol_range(&symbols, target_name).ok_or_else(|| {
            SerenaError::NotFound(format!("Symbol not found: {}", params.name_path))
        })?;

        // Read the file
        let content = std::fs::read_to_string(&file_path)
            .map_err(SerenaError::Io)?;

        let lines: Vec<&str> = content.lines().collect();

        // Insert at the symbol's start line
        let insert_line = range.start.line as usize;

        // Build new content
        let mut new_content = String::new();

        // Lines before the insert point
        for line in &lines[..insert_line] {
            new_content.push_str(line);
            new_content.push('\n');
        }

        // New content
        new_content.push_str(&params.content);
        if !params.content.ends_with('\n') {
            new_content.push('\n');
        }

        // Add empty line if needed for separation
        if !params.content.ends_with("\n\n") {
            new_content.push('\n');
        }

        // Lines from the symbol onwards
        for line in &lines[insert_line..] {
            new_content.push_str(line);
            new_content.push('\n');
        }

        // Write back
        std::fs::write(&file_path, &new_content)
            .map_err(SerenaError::Io)?;

        Ok(ToolResult::success(json!({
            "message": format!("Inserted content before symbol '{}' at line {} in {}",
                params.name_path, insert_line + 1, params.relative_path)
        })))
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "symbol".to_string(),
            "edit".to_string(),
            "insert".to_string(),
            "lsp".to_string(),
        ]
    }
}

// ============================================================================
// Symbol Tools Factory
// ============================================================================

/// Create all symbol operation tools
///
/// These tools require an LSP client for semantic code operations.
///
/// # Arguments
/// * `project_root` - The root path of the project
/// * `lsp_client` - Shared LSP client for language server communication
///
/// # Returns
/// Vector of all symbol tools wrapped in Arc for shared ownership
pub fn create_symbol_tools(
    project_root: impl Into<PathBuf> + Clone,
    lsp_client: Arc<RwLock<Box<dyn LanguageServer>>>,
) -> Vec<Arc<dyn Tool>> {
    let root: PathBuf = project_root.into();
    vec![
        Arc::new(GetSymbolsOverviewTool::new(root.clone(), lsp_client.clone())),
        Arc::new(FindSymbolTool::new(root.clone(), lsp_client.clone())),
        Arc::new(FindReferencingSymbolsTool::new(root.clone(), lsp_client.clone())),
        Arc::new(ReplaceSymbolBodyTool::new(root.clone(), lsp_client.clone())),
        Arc::new(RenameSymbolTool::new(root.clone(), lsp_client.clone())),
        Arc::new(InsertAfterSymbolTool::new(root.clone(), lsp_client.clone())),
        Arc::new(InsertBeforeSymbolTool::new(root, lsp_client)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_output() {
        let text = "Hello, World!".to_string();
        assert_eq!(truncate_output(text.clone(), -1), text);
        assert_eq!(truncate_output(text.clone(), 5), "Hello... (truncated)");
        assert_eq!(truncate_output(text.clone(), 100), text);
    }
}
