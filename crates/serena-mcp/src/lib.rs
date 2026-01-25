pub mod protocol;
pub mod server;
pub mod transport;

pub use protocol::{McpError, McpRequest, McpResponse, ToolInfo};
pub use server::SerenaMcpServer;
pub use transport::http::HttpTransport;
pub use transport::stdio::StdioTransport;
