# Priority Unwrap Fixes - Top 20 Highest Risk

## Instructions

Apply these fixes in priority order. Each fix includes:
- File location and line number
- Current code
- Recommended fix
- Risk justification

---

## Fix #1: Server Default Bind Address (HIGH RISK)

**File:** `crates/serena-web/src/server.rs:33`

**Current Code:**
```rust
impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:3000".parse().unwrap(),
            enable_cors: true,
            max_body_size: 10 * 1024 * 1024, // 10MB
        }
    }
}
```

**Fixed Code:**
```rust
impl Default for WebServerConfig {
    fn default() -> Self {
        const DEFAULT_BIND: &str = "127.0.0.1:3000";
        Self {
            bind_addr: DEFAULT_BIND
                .parse()
                .expect("DEFAULT_BIND constant must be valid SocketAddr"),
            enable_cors: true,
            max_body_size: 10 * 1024 * 1024, // 10MB
        }
    }
}
```

**Why:**
- Production code path executed at server initialization
- Hard-coded value is safe, but expect() provides better error message
- Using const makes the invariant explicit

**Impact:** Server initialization panic → Clear compile-time constant validation

---

## Fix #2: Test Server Bind Address (MEDIUM RISK)

**File:** `crates/serena-web/src/server.rs:169`

**Current Code:**
```rust
let config = WebServerConfig {
    bind_addr: "0.0.0.0:8080".parse().unwrap(),
    enable_cors: false,
    max_body_size: 5 * 1024 * 1024,
};
```

**Fixed Code:**
```rust
let config = WebServerConfig {
    bind_addr: "0.0.0.0:8080"
        .parse()
        .expect("Test bind address should be valid SocketAddr"),
    enable_cors: false,
    max_body_size: 5 * 1024 * 1024,
};
```

**Why:**
- Test code, but demonstrates production pattern
- Better error message helps debug test failures
- Documents expected invariant

**Impact:** Clearer test failure messages

---

## Fix #3: SSE Transport Send (HIGH RISK if used in production)

**File:** `crates/serena-web/src/sse.rs:112`

**Current Code:**
```rust
#[tokio::test]
async fn test_sse_transport_send() {
    let (transport, mut stream) = SseTransport::new();

    let response = McpResponse::success(Some(1), serde_json::json!({"test": "data"}));
    transport.send(response.clone()).unwrap();

    // Receive the response
    let received = stream.rx.recv().await;
    assert!(received.is_some());
    let received = received.unwrap();
    assert_eq!(received.id, Some(1));
}
```

**Fixed Code:**
```rust
#[tokio::test]
async fn test_sse_transport_send() {
    let (transport, mut stream) = SseTransport::new();

    let response = McpResponse::success(Some(1), serde_json::json!({"test": "data"}));
    transport
        .send(response.clone())
        .expect("Should send response in test");

    // Receive the response
    let received = stream.rx.recv().await;
    assert!(
        received.is_some(),
        "Should receive response from channel"
    );
    let received = received.expect("Received value should exist");
    assert_eq!(received.id, Some(1));
}
```

**Why:**
- Channel send can fail if receiver dropped
- Test code, but pattern may be used in production
- Better error messages for test debugging

**Impact:** Better test diagnostics, documents channel semantics

---

## Fix #4-6: LSP Client JSON Serialization (MEDIUM RISK)

**File:** `crates/serena-lsp/src/client.rs:357,366,373`

**Current Code:**
```rust
#[test]
fn test_jsonrpc_request_serialization() {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: serde_json::json!({}),
        id: 1,
    };

    let serialized = serde_json::to_string(&req).unwrap();
    assert!(serialized.contains("\"method\":\"initialize\""));
    assert!(serialized.contains("\"id\":1"));
}

#[test]
fn test_jsonrpc_response_deserialization() {
    // Test with non-null result
    let json = r#"{"jsonrpc":"2.0","id":1,"result":{"success":true}}"#;
    let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.id, Some(1));
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());

    // Test with null result
    let json_null = r#"{"jsonrpc":"2.0","id":2,"result":null}"#;
    let resp_null: JsonRpcResponse = serde_json::from_str(json_null).unwrap();
    assert_eq!(resp_null.id, Some(2));
    assert!(resp_null.result.is_none() || resp_null.result == Some(Value::Null));
    assert!(resp_null.error.is_none());
}
```

**Fixed Code:**
```rust
#[test]
fn test_jsonrpc_request_serialization() {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: serde_json::json!({}),
        id: 1,
    };

    let serialized = serde_json::to_string(&req)
        .expect("JsonRpcRequest should serialize to JSON");
    assert!(serialized.contains("\"method\":\"initialize\""));
    assert!(serialized.contains("\"id\":1"));
}

#[test]
fn test_jsonrpc_response_deserialization() {
    // Test with non-null result
    let json = r#"{"jsonrpc":"2.0","id":1,"result":{"success":true}}"#;
    let resp: JsonRpcResponse = serde_json::from_str(json)
        .expect("Valid JSON should deserialize to JsonRpcResponse");
    assert_eq!(resp.id, Some(1));
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());

    // Test with null result
    let json_null = r#"{"jsonrpc":"2.0","id":2,"result":null}"#;
    let resp_null: JsonRpcResponse = serde_json::from_str(json_null)
        .expect("Valid JSON with null result should deserialize");
    assert_eq!(resp_null.id, Some(2));
    assert!(resp_null.result.is_none() || resp_null.result == Some(Value::Null));
    assert!(resp_null.error.is_none());
}
```

**Why:**
- JSON operations can fail on malformed data
- Better error messages for test debugging
- Documents expected serialization invariants

**Impact:** Clearer test failure diagnostics

---

## Fix #7: MCP HTTP Transport Error Field (MEDIUM RISK)

**File:** `crates/serena-mcp/src/transport/http.rs:106`

**Current Code:**
```rust
let response = transport.handle_request(request).await;

assert_eq!(response.id, Some(1));
assert!(response.result.is_none());
assert!(response.error.is_some());

let error = response.error.unwrap();
assert_eq!(error.code, -32601);
```

**Fixed Code:**
```rust
let response = transport.handle_request(request).await;

assert_eq!(response.id, Some(1));
assert!(response.result.is_none());
assert!(response.error.is_some());

let error = response
    .error
    .expect("Response should contain error for unknown method");
assert_eq!(error.code, -32601);
```

**Why:**
- Test assumes error field is Some
- Better error message if assumption violated
- Documents test expectation

**Impact:** Better test failure diagnostics

---

## Fix #8-9: MCP STDIO Transport (MEDIUM RISK)

**File:** `crates/serena-mcp/src/transport/stdio.rs:113,123`

**Current Code:**
```rust
#[test]
fn test_mcp_request_deserialization() {
    let json = r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#;
    let request: McpRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.method, "initialize");
    assert_eq!(request.id, Some(1));
}

#[test]
fn test_mcp_response_serialization() {
    let response = McpResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(1),
        result: Some(serde_json::json!({"status": "ok"})),
        error: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"result\""));
}
```

**Fixed Code:**
```rust
#[test]
fn test_mcp_request_deserialization() {
    let json = r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#;
    let request: McpRequest = serde_json::from_str(json)
        .expect("Valid MCP JSON should deserialize to McpRequest");
    assert_eq!(request.method, "initialize");
    assert_eq!(request.id, Some(1));
}

#[test]
fn test_mcp_response_serialization() {
    let response = McpResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(1),
        result: Some(serde_json::json!({"status": "ok"})),
        error: None,
    };

    let json = serde_json::to_string(&response)
        .expect("McpResponse should serialize to JSON");
    assert!(json.contains("\"result\""));
}
```

**Why:**
- JSON operations documented with expectations
- Better debugging when deserialization fails
- Clear test intent

**Impact:** Improved test diagnostics

---

## Fixes #10-20: Test Setup Unwraps (LOW-MEDIUM RISK)

These are lower priority but still worth fixing for better test diagnostics.

### Pattern: TempDir Creation

**Files:** Multiple test files across crates

**Current Pattern:**
```rust
let temp_dir = TempDir::new().unwrap();
```

**Fixed Pattern:**
```rust
let temp_dir = TempDir::new()
    .expect("Should create temporary directory for test");
```

**Why:** Better error messages when test setup fails

---

### Pattern: File Writing in Tests

**Current Pattern:**
```rust
std::fs::write(&config_path, yaml_content).unwrap();
fs::write(&file_path, "content").await.unwrap();
```

**Fixed Pattern:**
```rust
std::fs::write(&config_path, yaml_content)
    .expect("Should write test configuration file");
fs::write(&file_path, "content")
    .await
    .expect("Should write test file");
```

**Why:** Clearer diagnostics for test setup failures

---

### Pattern: Tool Execution in Tests

**Current Pattern:**
```rust
let result = tool.execute(params).await.unwrap();
let data = result.data.unwrap();
```

**Fixed Pattern:**
```rust
let result = tool
    .execute(params)
    .await
    .expect("Tool execution should succeed in test");
let data = result
    .data
    .expect("Result should contain data");
```

**Why:** Better context for test failures

---

### Pattern: JSON Deserialization in Tests

**Current Pattern:**
```rust
let output: OutputType = serde_json::from_value(data).unwrap();
```

**Fixed Pattern:**
```rust
let output: OutputType = serde_json::from_value(data)
    .expect("Data should deserialize to OutputType");
```

**Why:** Documents expected type conversions

---

## Automated Fix Script

Create `scripts/fix-unwraps.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

# Fix high-priority unwraps in production code
echo "Fixing high-priority unwraps..."

# Fix #1: Server default bind address
sed -i 's/bind_addr: "127.0.0.1:3000".parse().unwrap(),/bind_addr: "127.0.0.1:3000".parse().expect("Default bind address must be valid"),/' \
    crates/serena-web/src/server.rs

# Fix test unwraps with expect() - requires manual review for messages
find crates -name "*.rs" -type f -exec sed -i \
    's/\.unwrap()/\.expect("TODO: Add expect message")/' {} +

echo "Done! Review changes and add appropriate expect() messages."
echo "Run: cargo test --workspace to verify fixes."
```

---

## Verification Commands

After applying fixes:

```bash
# 1. Check compilation
cargo check --workspace

# 2. Run all tests
cargo test --workspace

# 3. Search for remaining unwraps
rg "\.unwrap\(\)" crates/ --type rust

# 4. Check for new panics
cargo clippy --workspace -- -W clippy::unwrap_used

# 5. Run specific crate tests
cargo test -p serena-web
cargo test -p serena-lsp
cargo test -p serena-mcp
```

---

## Summary of Priority Fixes

| Fix # | File | Line | Risk | Estimated Time |
|-------|------|------|------|----------------|
| 1 | serena-web/server.rs | 33 | HIGH | 5 min |
| 2 | serena-web/server.rs | 169 | MED | 2 min |
| 3 | serena-web/sse.rs | 112,117 | HIGH | 5 min |
| 4-6 | serena-lsp/client.rs | 357,366,373 | MED | 5 min |
| 7 | serena-mcp/http.rs | 106 | MED | 2 min |
| 8-9 | serena-mcp/stdio.rs | 113,123 | MED | 3 min |
| 10-20 | Various test files | Multiple | LOW | 30 min |

**Total Estimated Time:** 1 hour for priority fixes, 2-3 hours for all test improvements

---

## Prevention Strategy

Add to each crate's `lib.rs`:

```rust
// Deny unwrap in production code
#![deny(clippy::unwrap_used)]

// Allow unwrap in tests
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
}
```

Add to CI pipeline (`.github/workflows/ci.yml`):

```yaml
- name: Check for unwrap in production code
  run: |
    # Exclude test modules
    rg "\.unwrap\(\)" crates/ --type rust \
      --glob '!**/tests/**' \
      --glob '!**/test*.rs' \
      && exit 1 || exit 0
```
