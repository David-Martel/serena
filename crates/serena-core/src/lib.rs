pub mod error;
pub mod prompts;
pub mod registry;
pub mod traits;
pub mod types;

// Test utilities (available in tests and with test-utils feature)
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

// Re-export error types
pub use error::{ConfigError, LspError, SerenaError, ToolError};

// Re-export core types
pub use types::{Location, Position, Range, SymbolInfo, SymbolKind, ToolResult, ToolStatus};

// Re-export traits
pub use traits::{LanguageServer, MemoryStorage, Tool};

// Re-export registry
pub use registry::{ToolRegistry, ToolRegistryBuilder};

// Re-export prompts
pub use prompts::PromptFactory;
