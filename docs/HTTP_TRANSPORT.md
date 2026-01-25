# HTTP Transport Implementation for Serena MCP Server

This document describes the HTTP and SSE transport implementation for the Serena MCP server.

## Overview

The Serena MCP server now supports three transport modes:

1. **stdio** - Standard input/output (default, for local integration)
2. **http** - HTTP JSON-RPC (for web-based clients)
3. **sse** - Server-Sent Events (for streaming responses)

## Architecture

### Components

The HTTP transport implementation consists of several layers:

```
serena (main binary)
  └─> serena-web (HTTP/SSE server)
      ├─> HttpTransport (request routing)
      ├─> SseTransport (event streaming)
      └─> serena-mcp
          ├─> HttpTransport (MCP adapter)
          └─> SerenaMcpServer (MCP protocol handler)
```

### File Structure

- **`crates/serena/src/app.rs`** - Main application with `run_http()` and `run_sse()` methods
- **`crates/serena-mcp/src/transport/http.rs`** - HTTP transport adapter for MCP
- **`crates/serena-web/src/http.rs`** - HTTP JSON-RPC handlers
- **`crates/serena-web/src/sse.rs`** - SSE streaming handlers
- **`crates/serena-web/src/server.rs`** - Axum web server configuration

## Usage

### Starting the HTTP Server

```bash
# Default port 3000
cargo run --release -- --transport http

# Custom port
cargo run --release -- --transport http --port 8080

# With project
cargo run --release -- --transport http --port 3000 --project /path/to/project
```

### Starting the SSE Server

```bash
# SSE uses the same server infrastructure
cargo run --release -- --transport sse --port 3000
```

## API Endpoints

### Health Check

```bash
GET http://localhost:3000/health
```

**Response:**
```json
{
  "status": "ok",
  "service": "serena-mcp",
  "version": "0.2.0"
}
```

### MCP JSON-RPC (Single Request)

```bash
POST http://localhost:3000/mcp
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {}
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {
        "listChanged": false
      }
    },
    "serverInfo": {
      "name": "serena-mcp",
      "version": "0.2.0"
    }
  }
}
```

### MCP Batch Requests

```bash
POST http://localhost:3000/mcp/batch
Content-Type: application/json

[
  {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "ping",
    "params": {}
  },
  {
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {}
  }
]
```

**Response:**
```json
[
  {
    "jsonrpc": "2.0",
    "id": 1,
    "result": {}
  },
  {
    "jsonrpc": "2.0",
    "id": 2,
    "result": {
      "tools": [...]
    }
  }
]
```

### SSE Event Stream

```bash
GET http://localhost:3000/mcp/events
```

Opens a long-lived SSE connection for streaming MCP responses.

## Supported MCP Methods

- **`initialize`** - Initialize the MCP server connection
- **`ping`** - Health check
- **`tools/list`** - List available tools
- **`tools/call`** - Execute a tool with parameters

## Testing

### Using the Test Script

```bash
# Install dependencies
uv pip install requests

# Start the server in one terminal
cargo run --release -- --transport http --port 3000

# Run tests in another terminal
uv run python test_http_transport.py
```

### Using curl

```bash
# Health check
curl http://localhost:3000/health

# Initialize
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {}
  }'

# List tools
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {}
  }'

# Batch request
curl -X POST http://localhost:3000/mcp/batch \
  -H "Content-Type: application/json" \
  -d '[
    {"jsonrpc": "2.0", "id": 1, "method": "ping", "params": {}},
    {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}
  ]'
```

## Configuration

The web server can be configured via `WebServerConfig`:

```rust
use serena_web::WebServerConfig;
use std::net::SocketAddr;

let config = WebServerConfig {
    bind_addr: SocketAddr::from(([0, 0, 0, 0], 3000)),
    enable_cors: true,                    // Allow cross-origin requests
    max_body_size: 10 * 1024 * 1024,     // 10MB request limit
};
```

### CORS

CORS is enabled by default with the following settings:
- **Origins:** All (`*`)
- **Methods:** GET, POST, OPTIONS
- **Headers:** Content-Type, Authorization
- **Max Age:** 3600 seconds

## Performance Considerations

### Batch Requests

Batch requests are processed **concurrently** using `futures::future::join_all`, providing better performance than sequential processing.

### Connection Pooling

The HTTP transport reuses connections via the underlying Axum/Hyper infrastructure.

### Async Processing

All request handlers are fully async, allowing the server to handle multiple concurrent requests efficiently.

## Error Handling

### JSON-RPC Error Codes

- **-32700** - Parse error (invalid JSON)
- **-32600** - Invalid request (e.g., empty batch)
- **-32601** - Method not found
- **-32602** - Invalid params
- **-32603** - Internal error

### Example Error Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32601,
    "message": "Method not found: unknown_method"
  }
}
```

## Implementation Details

### Request Flow

1. **Client** sends HTTP POST with JSON-RPC request
2. **Axum** router dispatches to `http_handler`
3. **HttpTransport** forwards to `SerenaMcpServer.handle_request()`
4. **SerenaMcpServer** processes the MCP method
5. **Response** flows back through the stack as JSON

### Async Handler Integration

The key innovation is making `SerenaMcpServer::handle_request()` public and async-compatible:

```rust
// In serena-mcp/src/server.rs
pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
    // Process MCP protocol methods
}

// In serena-mcp/src/transport/http.rs
pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
    self.mcp_server.handle_request(request).await
}

// In serena-web/src/http.rs
pub async fn http_handler(
    State(transport): State<Arc<HttpTransport>>,
    Json(request): Json<McpRequest>,
) -> Response {
    let response = transport.handle_request(request).await;
    Json(response).into_response()
}
```

## Security Considerations

### Network Binding

The default configuration binds to `0.0.0.0` (all interfaces). For production:

```bash
# Bind to localhost only
# (Currently requires code modification - future enhancement)
```

### Authentication

HTTP transport currently has **no authentication**. For production use, implement:

- Bearer token authentication
- API key validation
- Rate limiting
- Request size limits (already implemented: 10MB default)

### HTTPS

For production, use a reverse proxy (nginx, Caddy) to terminate TLS:

```nginx
server {
    listen 443 ssl;
    server_name mcp.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Future Enhancements

- [ ] WebSocket transport (bidirectional)
- [ ] Authentication middleware
- [ ] Rate limiting
- [ ] Request/response logging
- [ ] Metrics and observability
- [ ] Session management for SSE
- [ ] Graceful shutdown handling

## Troubleshooting

### Port Already in Use

```bash
# Error: Address already in use (os error 10048)
# Solution: Use a different port
cargo run --release -- --transport http --port 3001
```

### Connection Refused

```bash
# Ensure the server is running
cargo run --release -- --transport http

# Check firewall settings
# Windows: Allow inbound TCP on port 3000
```

### CORS Errors

If you encounter CORS errors in a web browser, ensure:
- Server is started with CORS enabled (default)
- Request includes proper `Origin` header
- Check browser console for specific CORS error

## Related Files

- `crates/serena/src/app.rs` - Main application logic
- `crates/serena-mcp/src/server.rs` - MCP protocol implementation
- `crates/serena-web/src/server.rs` - Web server configuration
- `test_http_transport.py` - Test suite

## License

Same as the Serena project (see main LICENSE file).
