use async_trait::async_trait;
use lsp_types::{
    GotoDefinitionResponse, InitializeParams, Location, RenameParams, ServerCapabilities,
    TextDocumentIdentifier, TextDocumentPositionParams, WorkspaceEdit,
};

use crate::{LspError, SymbolInfo};

/// Trait for language server implementations
#[async_trait]
pub trait LanguageServer: Send + Sync {
    /// Initialize the language server
    async fn initialize(
        &mut self,
        params: InitializeParams,
    ) -> Result<ServerCapabilities, LspError>;

    /// Shutdown the language server
    async fn shutdown(&mut self) -> Result<(), LspError>;

    /// Get document symbols for a file
    async fn document_symbols(
        &self,
        document: TextDocumentIdentifier,
    ) -> Result<Vec<SymbolInfo>, LspError>;

    /// Find all references to a symbol at the given position
    async fn find_references(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Vec<Location>, LspError>;

    /// Rename a symbol
    async fn rename(&self, params: RenameParams) -> Result<WorkspaceEdit, LspError>;

    /// Go to definition of a symbol at the given position
    async fn goto_definition(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<GotoDefinitionResponse, LspError>;

    /// Notify the server that a document was opened
    async fn did_open(
        &self,
        uri: String,
        language_id: String,
        text: String,
    ) -> Result<(), LspError>;

    /// Notify the server that a document was closed
    async fn did_close(&self, uri: String) -> Result<(), LspError>;

    /// Notify the server that a document was changed
    async fn did_change(&self, uri: String, text: String) -> Result<(), LspError>;

    /// Get the language ID this server handles
    fn language_id(&self) -> &str;

    /// Check if the server is initialized
    fn is_initialized(&self) -> bool;

    /// Check if the server is running
    fn is_running(&self) -> bool;

    /// Restart the language server
    async fn restart(&mut self) -> Result<(), LspError> {
        self.shutdown().await?;
        // Re-initialization should be done by the caller with appropriate params
        Ok(())
    }
}
