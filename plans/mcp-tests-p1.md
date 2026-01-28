# Implementation Plan: MCP Tests Phases 1-3

## Overview

Add comprehensive test coverage for the MCP implementation phases 1-3: transport layer, protocol layer, and client layer. This includes unit tests for JSON-RPC serialization, timeout handling, error propagation, protocol message serde, and client protocol flow. Also create a reusable Python echo MCP server fixture for integration testing.

**Current State:**
- `transport.rs` - Implemented with tests in `transport_tests.rs`
- `protocol.rs` - Implemented with tests in `protocol_tests.rs`
- `client.rs` - **NOT YET IMPLEMENTED** (planned in `plans/mcp-client.md`)

## Project Structure

```
crates/cli/src/mcp/
├── mod.rs              # Update: add client module export
├── transport.rs        # Existing: JSON-RPC transport
├── transport_tests.rs  # Existing: enhance with additional coverage
├── protocol.rs         # Existing: MCP protocol types
├── protocol_tests.rs   # Existing: enhance with edge cases
├── client.rs           # NEW: MCP client implementation
└── client_tests.rs     # NEW: client unit and integration tests

tests/
└── fixtures/
    └── echo_mcp_server.py  # NEW: reusable test fixture
```

## Dependencies

No new dependencies. Uses existing:
- `tokio` - async runtime, tests
- `serde_json` - JSON serialization
- `thiserror` - error handling
- Python 3 - for echo MCP server fixture

## Implementation Phases

### Phase 1: Create Test Fixtures Directory and Echo Server

**Goal:** Create a reusable Python MCP server for integration testing.

**Files:**
- `tests/fixtures/echo_mcp_server.py` (new)

**Implementation:**

```python
#!/usr/bin/env python3
"""Minimal MCP server for integration testing.

Implements the MCP protocol for testing:
- initialize: Returns server info and capabilities
- notifications/initialized: Acknowledges initialization
- tools/list: Returns one echo tool
- tools/call: Echoes back the input arguments

Usage:
    python3 echo_mcp_server.py

The server reads JSON-RPC requests from stdin (one per line)
and writes JSON-RPC responses to stdout.
"""
import json
import sys

def main():
    for line in sys.stdin:
        try:
            req = json.loads(line.strip())
            method = req.get("method", "")
            req_id = req.get("id")
            params = req.get("params", {})

            if method == "initialize":
                result = {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {"listChanged": False}},
                    "serverInfo": {"name": "echo-test", "version": "1.0.0"}
                }
            elif method == "notifications/initialized":
                # Notifications don't get responses
                continue
            elif method == "tools/list":
                result = {
                    "tools": [
                        {
                            "name": "echo",
                            "description": "Echo back input arguments",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "message": {"type": "string"}
                                }
                            }
                        },
                        {
                            "name": "fail",
                            "description": "Always returns an error",
                            "inputSchema": {"type": "object"}
                        }
                    ]
                }
            elif method == "tools/call":
                tool_name = params.get("name", "")
                arguments = params.get("arguments", {})

                if tool_name == "echo":
                    result = {
                        "content": [{"type": "text", "text": json.dumps(arguments)}],
                        "isError": False
                    }
                elif tool_name == "fail":
                    result = {
                        "content": [{"type": "text", "text": "Intentional failure"}],
                        "isError": True
                    }
                else:
                    # Unknown tool
                    resp = {
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "error": {"code": -32601, "message": f"Tool not found: {tool_name}"}
                    }
                    print(json.dumps(resp), flush=True)
                    continue
            else:
                # Unknown method
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32601, "message": f"Method not found: {method}"}
                }
                print(json.dumps(resp), flush=True)
                continue

            if req_id is not None:
                resp = {"jsonrpc": "2.0", "id": req_id, "result": result}
                print(json.dumps(resp), flush=True)

        except json.JSONDecodeError as e:
            if "id" in locals():
                err = {"jsonrpc": "2.0", "id": req_id, "error": {"code": -32700, "message": f"Parse error: {e}"}}
                print(json.dumps(err), flush=True)
        except Exception as e:
            if "req_id" in locals() and req_id is not None:
                err = {"jsonrpc": "2.0", "id": req_id, "error": {"code": -32603, "message": str(e)}}
                print(json.dumps(err), flush=True)

if __name__ == "__main__":
    main()
```

**Verification:**
```bash
# Manual test
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | python3 tests/fixtures/echo_mcp_server.py
```

---

### Phase 2: Enhance Transport Tests

**Goal:** Add additional test coverage for edge cases in the transport layer.

**File:** `crates/cli/src/mcp/transport_tests.rs` (update)

**Tests to add:**

```rust
mod json_rpc_edge_cases {
    use super::*;

    #[test]
    fn response_with_null_result() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":null}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let result = resp.into_result().unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn request_with_complex_params() {
        let params = serde_json::json!({
            "nested": {"array": [1, 2, 3], "bool": true},
            "unicode": "こんにちは"
        });
        let req = JsonRpcRequest::new(99, "complex", Some(params.clone()));
        let json = serde_json::to_string(&req).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["params"]["nested"]["array"][0], 1);
        assert_eq!(parsed["params"]["unicode"], "こんにちは");
    }

    #[test]
    fn error_with_numeric_data() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"Custom","data":42}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.data.unwrap(), 42);
    }
}

mod transport_error_coverage {
    use super::*;

    #[test]
    fn all_error_variants_display() {
        let errors = vec![
            TransportError::Spawn("test".into()),
            TransportError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "file")),
            TransportError::Serialize("json".into()),
            TransportError::Deserialize("parse".into()),
            TransportError::ProcessExited,
            TransportError::Timeout(1000),
            TransportError::IdMismatch { request: 1, response: 2 },
            TransportError::JsonRpc(JsonRpcError { code: -32600, message: "test".into(), data: None }),
            TransportError::Shutdown,
        ];

        for err in errors {
            let msg = format!("{}", err);
            assert!(!msg.is_empty(), "Error should have display: {:?}", err);
        }
    }
}

mod concurrent_operations {
    use super::*;

    #[tokio::test]
    async fn sequential_requests_maintain_order() {
        // Verifies ID generation is sequential
        let def = McpServerDef {
            command: "python3".to_string(),
            args: vec![
                "-c".to_string(),
                r#"
import sys, json
for line in sys.stdin:
    req = json.loads(line)
    print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"id": req["id"]}}), flush=True)
"#.to_string(),
            ],
            ..Default::default()
        };

        let transport = StdioTransport::spawn(&def).await.unwrap();

        for expected_id in 1..=5 {
            let result = transport.request("test", None, 5000).await.unwrap();
            assert_eq!(result["id"], expected_id);
        }

        transport.shutdown().await.unwrap();
    }
}
```

**Coverage additions:**
- Null result handling
- Complex/nested params
- Unicode in messages
- Error data variants
- All error Display implementations
- Sequential request ordering

---

### Phase 3: Enhance Protocol Tests

**Goal:** Add edge case coverage for protocol message handling.

**File:** `crates/cli/src/mcp/protocol_tests.rs` (update)

**Tests to add:**

```rust
mod initialization_edge_cases {
    use super::*;

    #[test]
    fn deserialize_initialize_result_with_extra_fields() {
        // Servers may include additional fields we don't know about
        let json = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}, "unknownCap": true},
            "serverInfo": {"name": "test", "extraField": "ignored"},
            "extraTopLevel": 123
        });

        let result: InitializeResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.protocol_version, "2024-11-05");
        assert_eq!(result.server_info.name, "test");
    }

    #[test]
    fn client_capabilities_empty_by_default() {
        let caps = ClientCapabilities::default();
        let json = serde_json::to_value(&caps).unwrap();
        // Should be empty object, not null
        assert!(json.is_object());
    }
}

mod tools_list_edge_cases {
    use super::*;

    #[test]
    fn deserialize_empty_tools_list() {
        let json = json!({"tools": []});
        let result: ToolsListResult = serde_json::from_value(json).unwrap();
        assert!(result.tools.is_empty());
    }

    #[test]
    fn deserialize_tool_with_complex_schema() {
        let json = json!({
            "tools": [{
                "name": "complex",
                "description": "Complex tool",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "required_field": {"type": "string"},
                        "optional_array": {
                            "type": "array",
                            "items": {"type": "number"}
                        }
                    },
                    "required": ["required_field"]
                }
            }]
        });

        let result: ToolsListResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.tools[0].name, "complex");
        assert!(result.tools[0].input_schema["required"].is_array());
    }
}

mod tool_call_edge_cases {
    use super::*;

    #[test]
    fn tool_call_result_empty_content() {
        let result = ToolCallResult {
            content: vec![],
            is_error: false,
        };
        let mcp_result = result.into_tool_result();
        assert!(mcp_result.success);
        assert!(mcp_result.content.is_array());
    }

    #[test]
    fn deserialize_tool_result_with_mixed_content() {
        let json = json!({
            "content": [
                {"type": "text", "text": "Output line 1"},
                {"type": "image", "data": "base64data", "mimeType": "image/png"},
                {"type": "text", "text": "Output line 2"}
            ],
            "isError": false
        });

        let result: ToolCallResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.content.len(), 3);

        // Extract text content
        let text: Vec<_> = result.content.iter()
            .filter_map(|c| c.as_text())
            .collect();
        assert_eq!(text, vec!["Output line 1", "Output line 2"]);
    }

    #[test]
    fn tool_call_params_empty_arguments() {
        let params = ToolCallParams {
            name: "no_args".into(),
            arguments: Some(json!({})),
        };
        let json = serde_json::to_value(&params).unwrap();
        assert!(json["arguments"].is_object());
        assert!(json["arguments"].as_object().unwrap().is_empty());
    }
}

mod content_block_edge_cases {
    use super::*;

    #[test]
    fn resource_with_all_fields() {
        let json = json!({
            "type": "resource",
            "uri": "file:///test",
            "text": "content",
            "mimeType": "application/json"
        });
        let block: ContentBlock = serde_json::from_value(json).unwrap();
        match block {
            ContentBlock::Resource { uri, text, mime_type } => {
                assert_eq!(uri, "file:///test");
                assert_eq!(text, Some("content".into()));
                assert_eq!(mime_type, Some("application/json".into()));
            }
            _ => panic!("Expected Resource"),
        }
    }

    #[test]
    fn text_block_with_empty_string() {
        let block = ContentBlock::Text { text: "".into() };
        assert_eq!(block.as_text(), Some(""));
    }

    #[test]
    fn image_block_preserves_base64() {
        let data = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let block = ContentBlock::Image {
            data: data.into(),
            mime_type: "image/png".into(),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["data"], data);
    }
}
```

**Coverage additions:**
- Extra/unknown fields in server responses (forward compatibility)
- Empty capabilities handling
- Empty tool lists
- Complex JSON schemas
- Mixed content block types
- Empty arguments objects
- Resource blocks with all optional fields

---

### Phase 4: Implement MCP Client

**Goal:** Create the `McpClient` implementation per `plans/mcp-client.md`.

**File:** `crates/cli/src/mcp/client.rs` (new)

See `plans/mcp-client.md` for full implementation details. Key components:

```rust
use super::config::McpServerDef;
use super::protocol::{
    InitializeParams, InitializeResult, ServerInfo, ToolCallParams,
    ToolCallResult, ToolInfo, ToolsListResult, PROTOCOL_VERSION,
};
use super::transport::{JsonRpcNotification, StdioTransport, TransportError};
use thiserror::Error;

/// Errors that can occur during MCP client operations.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("invalid response: {0}")]
    InvalidResponse(String),

    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(String),

    #[error("client not initialized")]
    NotInitialized,

    #[error("client already initialized")]
    AlreadyInitialized,

    #[error("tool not found: {0}")]
    ToolNotFound(String),

    #[error("tool error: {0}")]
    ToolError(String),
}

/// MCP client for communicating with a server.
pub struct McpClient {
    transport: StdioTransport,
    definition: McpServerDef,
    server_info: Option<ServerInfo>,
    tools: Vec<ToolInfo>,
    initialized: bool,
    timeout_ms: u64,
}

impl McpClient {
    pub async fn connect(def: &McpServerDef) -> Result<Self, ClientError>;
    pub async fn initialize(&mut self) -> Result<&ServerInfo, ClientError>;
    pub async fn list_tools(&mut self) -> Result<&[ToolInfo], ClientError>;
    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> Result<ToolCallResult, ClientError>;
    pub async fn shutdown(self) -> Result<(), ClientError>;
    pub async fn connect_and_initialize(def: &McpServerDef) -> Result<Self, ClientError>;

    // Accessors
    pub fn is_initialized(&self) -> bool;
    pub async fn is_running(&self) -> bool;
    pub fn server_info(&self) -> Option<&ServerInfo>;
    pub fn tools(&self) -> &[ToolInfo];
    pub fn has_tool(&self, name: &str) -> bool;
}
```

---

### Phase 5: Implement Client Tests

**Goal:** Create comprehensive tests for the MCP client.

**File:** `crates/cli/src/mcp/client_tests.rs` (new)

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::mcp::config::McpServerDef;
use serde_json::json;

/// Helper to get the path to the echo server fixture
fn echo_server_path() -> String {
    // Resolve relative to workspace root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/../../tests/fixtures/echo_mcp_server.py", manifest_dir)
}

fn echo_server_def() -> McpServerDef {
    McpServerDef {
        command: "python3".into(),
        args: vec![echo_server_path()],
        timeout_ms: 5000,
        ..Default::default()
    }
}

mod error_types {
    use super::*;

    #[test]
    fn client_errors_display_correctly() {
        let errors = vec![
            ClientError::NotInitialized,
            ClientError::AlreadyInitialized,
            ClientError::ToolNotFound("missing".into()),
            ClientError::ToolError("failed".into()),
            ClientError::InvalidResponse("bad json".into()),
            ClientError::UnsupportedVersion("1.0".into()),
        ];

        for err in errors {
            let msg = format!("{}", err);
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn transport_error_converts_to_client_error() {
        let transport_err = TransportError::Shutdown;
        let client_err: ClientError = transport_err.into();
        assert!(matches!(client_err, ClientError::Transport(_)));
    }
}

mod connect {
    use super::*;

    #[tokio::test]
    async fn connects_to_echo_server() {
        let client = McpClient::connect(&echo_server_def()).await.unwrap();
        assert!(!client.is_initialized());
        assert!(client.is_running().await);
        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn connect_fails_for_missing_command() {
        let def = McpServerDef {
            command: "nonexistent_command_xyz".into(),
            ..Default::default()
        };
        let result = McpClient::connect(&def).await;
        assert!(matches!(result, Err(ClientError::Transport(_))));
    }
}

mod initialize {
    use super::*;

    #[tokio::test]
    async fn initializes_successfully() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();

        let server_info = client.initialize().await.unwrap();
        assert_eq!(server_info.name, "echo-test");
        assert!(client.is_initialized());

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fails_double_initialization() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();

        let result = client.initialize().await;
        assert!(matches!(result, Err(ClientError::AlreadyInitialized)));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn server_info_available_after_init() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        assert!(client.server_info().is_none());

        client.initialize().await.unwrap();

        let info = client.server_info().unwrap();
        assert_eq!(info.name, "echo-test");
        assert_eq!(info.version.as_deref(), Some("1.0.0"));

        client.shutdown().await.unwrap();
    }
}

mod list_tools {
    use super::*;

    #[tokio::test]
    async fn lists_tools_after_init() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();

        let tools = client.list_tools().await.unwrap();
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "echo"));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fails_before_initialization() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();

        let result = client.list_tools().await;
        assert!(matches!(result, Err(ClientError::NotInitialized)));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn has_tool_checks_cached_list() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();
        client.list_tools().await.unwrap();

        assert!(client.has_tool("echo"));
        assert!(client.has_tool("fail"));
        assert!(!client.has_tool("nonexistent"));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn tools_accessor_returns_cached() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        assert!(client.tools().is_empty());

        client.initialize().await.unwrap();
        client.list_tools().await.unwrap();

        assert!(!client.tools().is_empty());

        client.shutdown().await.unwrap();
    }
}

mod call_tool {
    use super::*;

    #[tokio::test]
    async fn calls_echo_tool() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();

        let result = client.call_tool("echo", json!({"message": "hello"})).await.unwrap();

        assert!(!result.is_error);
        assert!(!result.content.is_empty());

        // Verify echo returned our input
        let text = result.content.iter()
            .filter_map(|c| c.as_text())
            .next()
            .unwrap();
        assert!(text.contains("hello"));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn handles_tool_error() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();

        let result = client.call_tool("fail", json!({})).await.unwrap();
        assert!(result.is_error);

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fails_before_initialization() {
        let client = McpClient::connect(&echo_server_def()).await.unwrap();

        let result = client.call_tool("echo", json!({})).await;
        assert!(matches!(result, Err(ClientError::NotInitialized)));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn call_with_empty_arguments() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();

        let result = client.call_tool("echo", json!({})).await.unwrap();
        assert!(!result.is_error);

        client.shutdown().await.unwrap();
    }
}

mod connect_and_initialize {
    use super::*;

    #[tokio::test]
    async fn convenience_method_works() {
        let client = McpClient::connect_and_initialize(&echo_server_def()).await.unwrap();

        assert!(client.is_initialized());
        assert!(!client.tools().is_empty());
        assert!(client.has_tool("echo"));

        client.shutdown().await.unwrap();
    }
}

mod shutdown {
    use super::*;

    #[tokio::test]
    async fn shutdown_terminates_process() {
        let client = McpClient::connect(&echo_server_def()).await.unwrap();
        assert!(client.is_running().await);

        client.shutdown().await.unwrap();
        // Client is consumed, can't check is_running
    }
}

mod protocol_flow {
    use super::*;

    #[tokio::test]
    async fn full_lifecycle() {
        // Complete protocol flow test
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();

        // Step 1: Initialize
        let server_info = client.initialize().await.unwrap();
        assert_eq!(server_info.name, "echo-test");

        // Step 2: Discover tools
        let tools = client.list_tools().await.unwrap();
        assert!(tools.len() >= 2);

        // Step 3: Call tool
        let result = client.call_tool("echo", json!({"test": true})).await.unwrap();
        assert!(!result.is_error);

        // Step 4: Call another tool
        let result = client.call_tool("fail", json!({})).await.unwrap();
        assert!(result.is_error);

        // Step 5: Shutdown
        client.shutdown().await.unwrap();
    }
}

mod error_recovery {
    use super::*;

    #[tokio::test]
    async fn handles_server_crash() {
        let def = McpServerDef {
            command: "sh".into(),
            args: vec!["-c".into(), "exit 1".into()],
            timeout_ms: 1000,
            ..Default::default()
        };

        let mut client = McpClient::connect(&def).await.unwrap();

        // Give process time to exit
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Initialize should fail since process exited
        let result = client.initialize().await;
        assert!(result.is_err());
    }
}
```

---

### Phase 6: Module Integration

**Goal:** Export client module and update tests path.

**File:** `crates/cli/src/mcp/mod.rs` (update)

```rust
// Add client module
pub mod client;
#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;

// Add to re-exports
pub use client::{ClientError, McpClient};
```

---

## Key Implementation Details

### Test Fixture Discovery

Tests need to find the Python fixture relative to the workspace:

```rust
fn echo_server_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/../../tests/fixtures/echo_mcp_server.py", manifest_dir)
}
```

### Error Propagation Pattern

Client errors wrap transport errors for clean layering:

```rust
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),
    // ...
}
```

### Protocol State Machine

The client enforces correct ordering:
- `initialize()` - Requires `!initialized`
- `list_tools()`, `call_tool()` - Require `initialized`
- `shutdown()` - Consumes self

### Test Isolation

Each test creates its own client and shuts it down, ensuring no shared state between tests.

## Verification Plan

### Unit Test Execution

```bash
# Run transport tests
cargo test -p claudeless mcp::transport

# Run protocol tests
cargo test -p claudeless mcp::protocol

# Run client tests
cargo test -p claudeless mcp::client

# Run all MCP tests
cargo test -p claudeless mcp::

# Run with output
cargo test -p claudeless mcp:: -- --nocapture
```

### Test Coverage Checklist

**Transport Layer:**
- [x] JSON-RPC request serialization (existing)
- [x] JSON-RPC response deserialization (existing)
- [x] JSON-RPC error handling (existing)
- [x] Timeout behavior (existing)
- [x] Process spawn/shutdown (existing)
- [ ] Null result handling (new)
- [ ] Complex params (new)
- [ ] All error Display impls (new)

**Protocol Layer:**
- [x] Initialize params serialization (existing)
- [x] Initialize result deserialization (existing)
- [x] Tools list deserialization (existing)
- [x] Content block variants (existing)
- [x] Tool call params/result (existing)
- [ ] Extra/unknown fields (new)
- [ ] Empty values (new)
- [ ] Complex schemas (new)

**Client Layer:**
- [ ] Connect success/failure
- [ ] Initialize success/double-init
- [ ] List tools success/before-init
- [ ] Call tool success/error/before-init
- [ ] Shutdown behavior
- [ ] Full protocol flow
- [ ] Server crash handling

### Integration Verification

```bash
# Manual echo server test
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | python3 tests/fixtures/echo_mcp_server.py

# Full test suite
make check
```

### Final Validation

```bash
make check
```

This validates:
- All unit tests pass
- All integration tests pass
- Linting (clippy) passes
- Formatting is correct
- No compile warnings
