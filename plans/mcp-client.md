# Implementation Plan: MCP Client

## Overview

Build the MCP client that combines the transport layer (Phase 1) and protocol types (Phase 2) into a cohesive interface for managing the MCP server lifecycle. The client handles process spawning, protocol initialization, tool discovery, tool execution, and graceful shutdown.

This is Phase 3 of the larger MCP implementation (see `plans/mcp.md`).

## Project Structure

```
crates/cli/src/mcp/
├── mod.rs              # Update: add client module export
├── client.rs           # NEW: MCP client implementation
├── client_tests.rs     # NEW: Unit and integration tests
├── transport.rs        # Prerequisite: JSON-RPC transport (Phase 1)
├── protocol.rs         # Existing: MCP protocol types (Phase 2)
├── config.rs           # Existing: McpServerDef config
├── server.rs           # Existing: will use client in Phase 4
└── tools.rs            # Existing: McpToolResult
```

## Dependencies

No new dependencies. Uses existing:
- `tokio` - async runtime
- `serde_json` - JSON serialization
- `thiserror` - error handling (via transport.rs)
- Transport layer from Phase 1 (`transport.rs`)
- Protocol types from Phase 2 (`protocol.rs`)

**Prerequisite:** Phase 1 (transport.rs) must be implemented first.

## Implementation Phases

### Phase 1: Client Error Type

**Goal:** Define comprehensive error handling for client operations.

**File:** `crates/cli/src/mcp/client.rs`

```rust
use super::transport::TransportError;
use thiserror::Error;

/// Errors that can occur during MCP client operations.
#[derive(Debug, Error)]
pub enum ClientError {
    /// Transport-level error.
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    /// Failed to parse server response.
    #[error("invalid response: {0}")]
    InvalidResponse(String),

    /// Server returned unsupported protocol version.
    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(String),

    /// Client is not initialized.
    #[error("client not initialized")]
    NotInitialized,

    /// Client is already initialized.
    #[error("client already initialized")]
    AlreadyInitialized,

    /// Tool not found on server.
    #[error("tool not found: {0}")]
    ToolNotFound(String),

    /// Tool execution returned an error.
    #[error("tool error: {0}")]
    ToolError(String),
}
```

**Design notes:**
- Wraps `TransportError` for lower-level failures
- Provides specific errors for protocol-level issues
- Keeps errors actionable for callers

---

### Phase 2: Core Client Structure

**Goal:** Define the `McpClient` struct and its state.

```rust
use super::config::McpServerDef;
use super::protocol::{
    InitializeParams, InitializeResult, ServerInfo, ToolCallParams,
    ToolCallResult, ToolInfo, ToolsListResult, PROTOCOL_VERSION,
};
use super::transport::StdioTransport;

/// MCP client for communicating with a server.
///
/// Manages the full lifecycle of an MCP connection:
/// 1. Spawn server process via [`connect`](Self::connect)
/// 2. Initialize protocol via [`initialize`](Self::initialize)
/// 3. Discover tools via [`list_tools`](Self::list_tools)
/// 4. Execute tools via [`call_tool`](Self::call_tool)
/// 5. Clean shutdown via [`shutdown`](Self::shutdown)
pub struct McpClient {
    /// Underlying transport for JSON-RPC communication.
    transport: StdioTransport,

    /// Server definition used to create this client.
    definition: McpServerDef,

    /// Server info received during initialization.
    server_info: Option<ServerInfo>,

    /// Cached list of available tools.
    tools: Vec<ToolInfo>,

    /// Whether the client has completed initialization.
    initialized: bool,

    /// Default timeout for requests in milliseconds.
    timeout_ms: u64,
}
```

**Key fields:**
- `transport` - Owned transport for JSON-RPC
- `definition` - Retained for debugging/logging
- `server_info` - Populated after successful init
- `tools` - Cached tool list for quick lookup
- `initialized` - State flag to enforce correct ordering
- `timeout_ms` - Configurable from `McpServerDef.timeout`

---

### Phase 3: Connect and Initialize

**Goal:** Implement `connect()` and `initialize()` methods.

```rust
impl McpClient {
    /// Spawn a server process and create a client.
    ///
    /// This only spawns the process. Call [`initialize`](Self::initialize)
    /// to complete the MCP handshake.
    pub async fn connect(def: &McpServerDef) -> Result<Self, ClientError> {
        let transport = StdioTransport::spawn(def).await?;

        Ok(Self {
            transport,
            definition: def.clone(),
            server_info: None,
            tools: Vec::new(),
            initialized: false,
            timeout_ms: def.timeout.unwrap_or(30_000),
        })
    }

    /// Initialize the MCP protocol.
    ///
    /// Sends the `initialize` request and waits for server response.
    /// Must be called before `list_tools` or `call_tool`.
    pub async fn initialize(&mut self) -> Result<&ServerInfo, ClientError> {
        if self.initialized {
            return Err(ClientError::AlreadyInitialized);
        }

        let params = InitializeParams::default();
        let params_json = serde_json::to_value(&params)
            .map_err(|e| ClientError::InvalidResponse(e.to_string()))?;

        let result = self.transport
            .request("initialize", Some(params_json), self.timeout_ms)
            .await?;

        let init_result: InitializeResult = serde_json::from_value(result)
            .map_err(|e| ClientError::InvalidResponse(e.to_string()))?;

        // Verify protocol version compatibility
        if init_result.protocol_version != PROTOCOL_VERSION {
            return Err(ClientError::UnsupportedVersion(
                init_result.protocol_version.clone(),
            ));
        }

        self.server_info = Some(init_result.server_info);
        self.initialized = true;

        // Send initialized notification (no response expected)
        self.send_initialized_notification().await?;

        Ok(self.server_info.as_ref().unwrap())
    }

    /// Send the `initialized` notification after successful init.
    async fn send_initialized_notification(&self) -> Result<(), ClientError> {
        use super::transport::JsonRpcNotification;

        let notification = JsonRpcNotification::new("notifications/initialized", None);
        self.transport.send_notification(&notification).await?;
        Ok(())
    }
}
```

**Protocol flow:**
1. Client sends `initialize` request with `InitializeParams`
2. Server responds with `InitializeResult` containing server info and capabilities
3. Client sends `notifications/initialized` notification
4. Connection is now ready for tool operations

---

### Phase 4: Tool Discovery

**Goal:** Implement `list_tools()` method.

```rust
impl McpClient {
    /// Discover available tools from the server.
    ///
    /// Calls the `tools/list` method and caches the results.
    /// Can be called multiple times to refresh the tool list.
    pub async fn list_tools(&mut self) -> Result<&[ToolInfo], ClientError> {
        if !self.initialized {
            return Err(ClientError::NotInitialized);
        }

        let result = self.transport
            .request("tools/list", None, self.timeout_ms)
            .await?;

        let tools_result: ToolsListResult = serde_json::from_value(result)
            .map_err(|e| ClientError::InvalidResponse(e.to_string()))?;

        self.tools = tools_result.tools;
        Ok(&self.tools)
    }

    /// Get the cached tool list without making a request.
    pub fn tools(&self) -> &[ToolInfo] {
        &self.tools
    }

    /// Check if a tool is available by name.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name == name)
    }

    /// Get tool info by name.
    pub fn get_tool(&self, name: &str) -> Option<&ToolInfo> {
        self.tools.iter().find(|t| t.name == name)
    }
}
```

**Design notes:**
- Caches tool list for quick local lookups
- Allows refresh via repeated `list_tools()` calls
- Provides convenience methods for tool queries

---

### Phase 5: Tool Execution

**Goal:** Implement `call_tool()` method.

```rust
impl McpClient {
    /// Execute a tool call.
    ///
    /// Sends the `tools/call` method with the given name and arguments.
    /// Returns the raw `ToolCallResult` for caller to handle.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolCallResult, ClientError> {
        if !self.initialized {
            return Err(ClientError::NotInitialized);
        }

        // Optionally validate tool exists (may be skipped if tool list changed)
        // if !self.has_tool(name) {
        //     return Err(ClientError::ToolNotFound(name.to_string()));
        // }

        let params = ToolCallParams {
            name: name.to_string(),
            arguments: Some(arguments),
        };

        let params_json = serde_json::to_value(&params)
            .map_err(|e| ClientError::InvalidResponse(e.to_string()))?;

        let result = self.transport
            .request("tools/call", Some(params_json), self.timeout_ms)
            .await?;

        let tool_result: ToolCallResult = serde_json::from_value(result)
            .map_err(|e| ClientError::InvalidResponse(e.to_string()))?;

        Ok(tool_result)
    }

    /// Execute a tool call with a custom timeout.
    pub async fn call_tool_with_timeout(
        &self,
        name: &str,
        arguments: serde_json::Value,
        timeout_ms: u64,
    ) -> Result<ToolCallResult, ClientError> {
        if !self.initialized {
            return Err(ClientError::NotInitialized);
        }

        let params = ToolCallParams {
            name: name.to_string(),
            arguments: Some(arguments),
        };

        let params_json = serde_json::to_value(&params)
            .map_err(|e| ClientError::InvalidResponse(e.to_string()))?;

        let result = self.transport
            .request("tools/call", Some(params_json), timeout_ms)
            .await?;

        let tool_result: ToolCallResult = serde_json::from_value(result)
            .map_err(|e| ClientError::InvalidResponse(e.to_string()))?;

        Ok(tool_result)
    }
}
```

**Design notes:**
- Returns raw `ToolCallResult` to let callers handle content blocks
- Tool validation is optional (server may have different tool list)
- Supports custom timeout for long-running tools

---

### Phase 6: Shutdown and Accessors

**Goal:** Implement `shutdown()` and accessor methods.

```rust
impl McpClient {
    /// Gracefully shut down the client.
    ///
    /// Closes the transport and terminates the server process.
    /// After shutdown, the client cannot be used again.
    pub async fn shutdown(self) -> Result<(), ClientError> {
        self.transport.shutdown().await?;
        Ok(())
    }

    /// Check if the client is initialized and ready for tool operations.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Check if the server process is still running.
    pub async fn is_running(&self) -> bool {
        self.transport.is_running().await
    }

    /// Get the server info (available after initialization).
    pub fn server_info(&self) -> Option<&ServerInfo> {
        self.server_info.as_ref()
    }

    /// Get the server definition used to create this client.
    pub fn definition(&self) -> &McpServerDef {
        &self.definition
    }
}
```

**Shutdown behavior:**
1. `shutdown()` consumes `self` to prevent further use
2. Delegates to transport for graceful process termination
3. Returns error if transport shutdown fails

---

### Phase 7: Convenience Constructor

**Goal:** Add a `connect_and_initialize()` helper for common use case.

```rust
impl McpClient {
    /// Connect to a server and initialize in one step.
    ///
    /// Convenience method that combines [`connect`](Self::connect),
    /// [`initialize`](Self::initialize), and [`list_tools`](Self::list_tools).
    ///
    /// Returns an initialized client with tools already discovered.
    pub async fn connect_and_initialize(def: &McpServerDef) -> Result<Self, ClientError> {
        let mut client = Self::connect(def).await?;
        client.initialize().await?;
        client.list_tools().await?;
        Ok(client)
    }
}
```

**Usage pattern:**
```rust
// Quick setup
let client = McpClient::connect_and_initialize(&server_def).await?;

// Or step-by-step for more control
let mut client = McpClient::connect(&server_def).await?;
let server_info = client.initialize().await?;
println!("Connected to {}", server_info.name);
let tools = client.list_tools().await?;
println!("Found {} tools", tools.len());
```

---

### Phase 8: Module Integration

**Goal:** Export client types and integrate with mod.rs.

**File:** `crates/cli/src/mcp/mod.rs` (update)

```rust
pub mod client;

// Existing modules...
pub mod config;
pub mod protocol;
pub mod server;
pub mod tools;
pub mod transport;  // Added in Phase 1

// Re-exports
pub use client::{ClientError, McpClient};
// ... existing re-exports ...
```

---

## Key Implementation Details

### State Machine

The client follows a strict state progression:

```
                      ┌──────────────┐
                      │   Created    │
                      │ (not init)   │
                      └──────┬───────┘
                             │ connect()
                      ┌──────▼───────┐
                      │   Spawned    │
                      │ (not init)   │
                      └──────┬───────┘
                             │ initialize()
                      ┌──────▼───────┐
                      │ Initialized  │
                      │ (ready)      │
                      └──────┬───────┘
                             │ list_tools(), call_tool()
                      ┌──────▼───────┐
                      │   Active     │
                      │              │
                      └──────┬───────┘
                             │ shutdown()
                      ┌──────▼───────┐
                      │   Shutdown   │
                      └──────────────┘
```

Methods enforce state requirements:
- `initialize()` - Requires not initialized
- `list_tools()`, `call_tool()` - Require initialized
- `shutdown()` - Consumes client (no further use)

### Error Recovery

The client does not attempt automatic reconnection:
- Transport errors bubble up as `ClientError::Transport`
- Callers should create a new client if reconnection is needed
- Server process crashes are detected via transport errors

### Timeout Handling

Timeouts flow through the layers:
```
McpServerDef.timeout
     │
     ▼
McpClient.timeout_ms
     │
     ▼
StdioTransport.request(timeout_ms)
     │
     ▼
tokio::time::timeout()
```

Per-call override via `call_tool_with_timeout()` for long-running tools.

### Thread Safety

The `McpClient` is `!Sync` due to internal mutability in transport.
For concurrent access, wrap in `Arc<Mutex<McpClient>>`:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

let client = Arc::new(Mutex::new(
    McpClient::connect_and_initialize(&def).await?
));

// From different tasks:
let result = client.lock().await.call_tool("name", args).await?;
```

## Verification Plan

### Unit Tests

**File:** `crates/cli/src/mcp/client_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use serde_json::json;

mod error_handling {
    use super::*;

    #[test]
    fn client_error_displays_correctly() {
        let err = ClientError::NotInitialized;
        assert!(err.to_string().contains("not initialized"));

        let err = ClientError::ToolNotFound("test".into());
        assert!(err.to_string().contains("test"));
    }
}
```

### Integration Tests with Echo Server

Create a simple MCP echo server for testing:

**File:** `tests/fixtures/echo_mcp_server.py`

```python
#!/usr/bin/env python3
"""Minimal MCP server for integration testing."""
import json
import sys

def main():
    for line in sys.stdin:
        try:
            req = json.loads(line.strip())
            method = req.get("method", "")
            req_id = req.get("id")

            if method == "initialize":
                result = {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "echo", "version": "1.0.0"}
                }
            elif method == "notifications/initialized":
                continue  # No response for notifications
            elif method == "tools/list":
                result = {
                    "tools": [
                        {
                            "name": "echo",
                            "description": "Echo back input",
                            "inputSchema": {"type": "object"}
                        }
                    ]
                }
            elif method == "tools/call":
                params = req.get("params", {})
                result = {
                    "content": [{"type": "text", "text": json.dumps(params)}],
                    "isError": False
                }
            else:
                result = {}

            if req_id is not None:
                resp = {"jsonrpc": "2.0", "id": req_id, "result": result}
                print(json.dumps(resp), flush=True)
        except Exception as e:
            if req.get("id"):
                err = {
                    "jsonrpc": "2.0",
                    "id": req.get("id"),
                    "error": {"code": -32600, "message": str(e)}
                }
                print(json.dumps(err), flush=True)

if __name__ == "__main__":
    main()
```

### Integration Test Cases

```rust
mod integration {
    use super::*;
    use crate::mcp::config::McpServerDef;

    fn echo_server_def() -> McpServerDef {
        McpServerDef {
            command: "python3".into(),
            args: vec!["tests/fixtures/echo_mcp_server.py".into()],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn connects_and_initializes() {
        let def = echo_server_def();
        let mut client = McpClient::connect(&def).await.unwrap();

        let server_info = client.initialize().await.unwrap();
        assert_eq!(server_info.name, "echo");
        assert!(client.is_initialized());

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn discovers_tools() {
        let def = echo_server_def();
        let mut client = McpClient::connect_and_initialize(&def).await.unwrap();

        assert!(client.has_tool("echo"));
        assert!(!client.has_tool("nonexistent"));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn calls_tool() {
        let def = echo_server_def();
        let mut client = McpClient::connect_and_initialize(&def).await.unwrap();

        let result = client.call_tool("echo", json!({"msg": "hello"})).await.unwrap();
        assert!(!result.is_error);
        assert!(!result.content.is_empty());

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn connect_and_initialize_convenience() {
        let def = echo_server_def();
        let client = McpClient::connect_and_initialize(&def).await.unwrap();

        assert!(client.is_initialized());
        assert!(!client.tools().is_empty());

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fails_without_initialization() {
        let def = echo_server_def();
        let client = McpClient::connect(&def).await.unwrap();

        let result = client.call_tool("echo", json!({})).await;
        assert!(matches!(result, Err(ClientError::NotInitialized)));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fails_double_initialization() {
        let def = echo_server_def();
        let mut client = McpClient::connect(&def).await.unwrap();
        client.initialize().await.unwrap();

        let result = client.initialize().await;
        assert!(matches!(result, Err(ClientError::AlreadyInitialized)));

        client.shutdown().await.unwrap();
    }
}
```

### Test with Invalid Server

```rust
mod error_cases {
    use super::*;

    #[tokio::test]
    async fn connect_fails_for_missing_command() {
        let def = McpServerDef {
            command: "nonexistent_command_12345".into(),
            ..Default::default()
        };

        let result = McpClient::connect(&def).await;
        assert!(matches!(result, Err(ClientError::Transport(_))));
    }

    #[tokio::test]
    async fn handles_server_crash() {
        let def = McpServerDef {
            command: "sh".into(),
            args: vec!["-c".into(), "exit 1".into()],
            ..Default::default()
        };

        let mut client = McpClient::connect(&def).await.unwrap();

        // Server exited, so initialize should fail
        let result = client.initialize().await;
        assert!(result.is_err());
    }
}
```

### Final Verification

```bash
# Run client tests
cargo test -p claudeless mcp::client

# Run with output
cargo test -p claudeless mcp::client -- --nocapture

# Full validation
make check
```

**Expected results:**
- All unit tests pass
- Integration tests with echo server pass
- Error cases handled correctly
- No clippy warnings
