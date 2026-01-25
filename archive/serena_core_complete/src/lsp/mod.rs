//! LSP client functionality
//!
//! **DEPRECATED**: Use `serena-lsp` crate instead.
//! - `LspClient` → `serena_lsp::LspClient`
//! - `ResourceManager` → `serena_lsp::ResourceManager`

pub mod client;

pub use client::LspClient;

// NOTE: resources.rs migrated to serena-lsp::ResourceManager
// Use `serena_lsp::ResourceManager` instead
