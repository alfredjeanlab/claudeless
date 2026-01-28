# Implementation Plan: Real MCP Server Connections

## Overview

Update the claudeless simulator to support real MCP (Model Context Protocol) server connections. This involves spawning actual MCP server processes, communicating via JSON-RPC over stdio, dynamically discovering tools through the MCP `tools/list` method, and routing tool calls through the MCP protocol.

Currently, the MCP infrastructure is simulated with stubs. This plan transforms it into a working implementation that can spawn and communicate with real MCP servers.

## Project Structure

```
crates/cli/src/mcp/
├── mod.rs           # Module exports
├── config.rs        # ✓ Already implemented - MCP config parsing
├── config_tests.rs  # ✓ Already implemented
├── server.rs        # Needs update - spawn real processes
├── server_tests.rs  # Needs update - add integration tests
├── tools.rs         # ✓ Tool definitions (templates remain for testing)
├── tools_tests.rs   # ✓ Already implemented
├── transport.rs     # NEW - JSON-RPC stdio transport
├── transport_tests.rs
├── protocol.rs      # NEW - MCP protocol messages
├── protocol_tests.rs
├── client.rs        # NEW - MCP client for server communication
└── client_tests.rs

crates/cli/src/tools/
├── executor.rs      # Needs update - add MCP tool routing
├── builtin/mod.rs   # Needs update - delegate to MCP when appropriate
└── mcp_executor.rs  # NEW - MCP tool executor implementation
```

## Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# None needed - using existing dependencies:
# - tokio (async runtime, process spawning)
# - serde_json (JSON-RPC serialization)
# - thiserror (error handling)
# - parking_lot (synchronization)
```

The implementation uses only existing dependencies.

## Implementation Phases

### Phase 1: JSON-RPC Transport Layer

**Goal:** Create the low-level transport for communicating with MCP servers over stdio.

**Files:**
- `crates/cli/src/mcp/transport.rs` (new)
- `crates/cli/src/mcp/transport_tests.rs` (new)

**Key structures:**

```rust
/// JSON-RPC request message.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,  // "2.0"
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC response message.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// Stdio transport for JSON-RPC communication.
pub struct StdioTransport {
    child: tokio::process::Child,
    stdin: tokio::io::BufWriter<tokio::process::ChildStdin>,
    stdout: tokio::io::BufReader<tokio::process::ChildStdout>,
    next_id: AtomicU64,
}
```

**Implementation:**
1. `StdioTransport::spawn()` - Spawn child process with stdin/stdout pipes
2. `StdioTransport::send()` - Write JSON-RPC request (newline-delimited)
3. `StdioTransport::receive()` - Read JSON-RPC response
4. `StdioTransport::request()` - Send and await response with timeout
5. `StdioTransport::shutdown()` - Graceful process termination

**Verification:**
- Unit tests with mock stdin/stdout
- Test JSON serialization/deserialization
- Test timeout handling

---

### Phase 2: MCP Protocol Messages

**Goal:** Define MCP-specific protocol messages on top of JSON-RPC.

**Files:**
- `crates/cli/src/mcp/protocol.rs` (new)
- `crates/cli/src/mcp/protocol_tests.rs` (new)

**Key structures:**

```rust
/// MCP initialize request params.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

/// MCP tools/list response.
#[derive(Debug, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<ToolInfo>,
}

/// Tool information from MCP server.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// MCP tools/call request params.
#[derive(Debug, Serialize)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// MCP tools/call response.
#[derive(Debug, Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<ContentBlock>,
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentBlock {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { uri: String, text: Option<String> },
}
```

**Verification:**
- Test serialization matches MCP spec
- Test deserialization of sample server responses

---

### Phase 3: MCP Client

**Goal:** Create a client that manages the MCP protocol lifecycle.

**Files:**
- `crates/cli/src/mcp/client.rs` (new)
- `crates/cli/src/mcp/client_tests.rs` (new)

**Key structure:**

```rust
/// MCP client for communicating with a server.
pub struct McpClient {
    transport: StdioTransport,
    server_info: Option<ServerInfo>,
    tools: Vec<ToolInfo>,
    initialized: bool,
}

impl McpClient {
    /// Spawn server process and create client.
    pub async fn connect(def: &McpServerDef) -> Result<Self, McpError>;

    /// Initialize the MCP protocol.
    pub async fn initialize(&mut self) -> Result<(), McpError>;

    /// Discover available tools.
    pub async fn list_tools(&mut self) -> Result<Vec<ToolInfo>, McpError>;

    /// Execute a tool call.
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolCallResult, McpError>;

    /// Graceful shutdown.
    pub async fn shutdown(&mut self) -> Result<(), McpError>;
}
```

**Protocol flow:**
1. Spawn process with `StdioTransport`
2. Send `initialize` request, receive `initialized` notification
3. Call `tools/list` to discover available tools
4. Ready for `tools/call` requests

**Verification:**
- Integration test with a simple echo MCP server script
- Test initialization handshake
- Test tool discovery
- Test tool execution

---

### Phase 4: Update McpServer and McpManager

**Goal:** Update existing types to use the new client.

**Files:**
- `crates/cli/src/mcp/server.rs` (update)
- `crates/cli/src/mcp/server_tests.rs` (update)
- `crates/cli/src/mcp/mod.rs` (update exports)

**Changes to `McpServer`:**

```rust
pub struct McpServer {
    pub name: String,
    pub definition: McpServerDef,
    pub tools: Vec<McpToolDef>,
    pub status: McpServerStatus,
    /// Live client connection (None in mock mode).
    client: Option<Arc<Mutex<McpClient>>>,
}

impl McpServer {
    /// Spawn the real MCP server process.
    pub async fn spawn(&mut self) -> Result<(), McpError> {
        // Validate definition
        if self.definition.command.is_empty() {
            return Err(McpError::InvalidConfig("No command specified"));
        }

        // Connect and initialize
        let mut client = McpClient::connect(&self.definition).await?;
        client.initialize().await?;

        // Discover tools
        let tools = client.list_tools().await?;
        self.tools = tools.into_iter()
            .map(|t| McpToolDef::from_tool_info(t, &self.name))
            .collect();

        self.client = Some(Arc::new(Mutex::new(client)));
        self.status = McpServerStatus::Running;
        Ok(())
    }

    /// Execute a tool call on this server.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        let client = self.client.as_ref()
            .ok_or(McpError::NotConnected)?;
        let mut guard = client.lock().await;
        guard.call_tool(name, arguments).await
            .map(McpToolResult::from)
    }
}
```

**Changes to `McpManager`:**

```rust
impl McpManager {
    /// Initialize all servers (spawn processes, discover tools).
    pub async fn initialize(&mut self) -> Vec<(String, Result<(), McpError>)> {
        let mut results = Vec::new();
        for (name, server) in &mut self.servers {
            let result = server.spawn().await;
            if result.is_ok() {
                // Update tool map
                for tool in &server.tools {
                    self.tool_server_map.insert(tool.name.clone(), name.clone());
                }
            }
            results.push((name.clone(), result));
        }
        results
    }

    /// Execute a tool call, routing to the appropriate server.
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        let server_name = self.tool_server_map.get(tool_name)
            .ok_or(McpError::ToolNotFound(tool_name.to_string()))?;
        let server = self.servers.get(server_name)
            .ok_or(McpError::ServerNotFound(server_name.clone()))?;
        server.call_tool(tool_name, arguments).await
    }

    /// Shutdown all servers gracefully.
    pub async fn shutdown(&mut self) {
        for server in self.servers.values_mut() {
            if let Some(client) = server.client.take() {
                let _ = client.lock().await.shutdown().await;
            }
        }
    }
}
```

**Verification:**
- Test `McpServer::spawn()` with real MCP server
- Test tool discovery populates `tools` field
- Test `call_tool` routes correctly

---

### Phase 5: MCP Tool Executor

**Goal:** Integrate MCP tools into the tool execution pipeline.

**Files:**
- `crates/cli/src/tools/mcp_executor.rs` (new)
- `crates/cli/src/tools/executor.rs` (update)
- `crates/cli/src/tools/mod.rs` (update exports)

**New executor:**

```rust
/// Executor that handles MCP tool calls.
pub struct McpToolExecutor {
    manager: Arc<RwLock<McpManager>>,
}

impl McpToolExecutor {
    pub fn new(manager: Arc<RwLock<McpManager>>) -> Self {
        Self { manager }
    }
}

impl ToolExecutor for McpToolExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        _ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        // Block on async call (executor trait is sync)
        let manager = self.manager.read();
        if !manager.has_tool(&call.tool) {
            return ToolExecutionResult::tool_not_found(tool_use_id, &call.tool);
        }

        // Use tokio runtime to execute async call
        let result = tokio::runtime::Handle::current()
            .block_on(manager.call_tool(&call.tool, call.input.clone()));

        match result {
            Ok(mcp_result) => {
                if mcp_result.success {
                    ToolExecutionResult::success(tool_use_id, &mcp_result.to_string())
                } else {
                    ToolExecutionResult::error(tool_use_id, mcp_result.error.unwrap_or_default())
                }
            }
            Err(e) => ToolExecutionResult::error(tool_use_id, e.to_string()),
        }
    }

    fn name(&self) -> &'static str {
        "mcp"
    }
}
```

**Composite executor:**

```rust
/// Executor that tries MCP first, then falls back to builtin.
pub struct CompositeExecutor {
    mcp: Option<McpToolExecutor>,
    builtin: BuiltinExecutor,
}

impl ToolExecutor for CompositeExecutor {
    fn execute(&self, call: &ToolCallSpec, id: &str, ctx: &ExecutionContext) -> ToolExecutionResult {
        // Check MCP first
        if let Some(ref mcp) = self.mcp {
            if mcp.manager.read().has_tool(&call.tool) {
                return mcp.execute(call, id, ctx);
            }
        }
        // Fall back to builtin
        self.builtin.execute(call, id, ctx)
    }
}
```

**Update `create_executor`:**

```rust
pub fn create_executor_with_mcp(
    mode: ToolExecutionMode,
    mcp_manager: Option<Arc<RwLock<McpManager>>>,
) -> Box<dyn ToolExecutor> {
    match mode {
        ToolExecutionMode::Disabled => Box::new(DisabledExecutor::new()),
        ToolExecutionMode::Mock => Box::new(MockExecutor::new()),
        ToolExecutionMode::Live => {
            let builtin = BuiltinExecutor::new();
            let mcp = mcp_manager.map(McpToolExecutor::new);
            Box::new(CompositeExecutor { mcp, builtin })
        }
    }
}
```

**Verification:**
- Test MCP tool routing
- Test fallback to builtin tools
- Test error handling for failed MCP calls

---

### Phase 6: Main Integration and CLI

**Goal:** Wire everything together in main.rs and update output.

**Files:**
- `crates/cli/src/main.rs` (update)
- `crates/cli/src/output_events.rs` (verify MCP output)

**Changes to `main.rs`:**

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... existing CLI parsing ...

    // Load and initialize MCP servers
    let mcp_manager = if !cli.mcp_config.is_empty() {
        let config = load_mcp_configs(&cli)?;
        let mut manager = McpManager::from_config(&config);

        // Initialize servers (spawn processes, discover tools)
        let results = manager.initialize().await;

        // Handle strict mode
        if cli.strict_mcp_config {
            for (name, result) in &results {
                if let Err(e) = result {
                    eprintln!("MCP server '{}' failed to start: {}", name, e);
                    std::process::exit(1);
                }
            }
        } else {
            // Log failures but continue
            for (name, result) in &results {
                if let Err(e) = result {
                    if cli.mcp_debug {
                        eprintln!("Warning: MCP server '{}' failed: {}", name, e);
                    }
                }
            }
        }

        Some(Arc::new(RwLock::new(manager)))
    } else {
        None
    };

    // Create executor with MCP support
    let executor = create_executor_with_mcp(
        scenario.tool_execution_mode(),
        mcp_manager.clone(),
    );

    // ... rest of main ...

    // Cleanup on exit
    if let Some(mgr) = mcp_manager {
        mgr.write().shutdown().await;
    }
}
```

**Verification:**
- End-to-end test with real MCP server
- Test `--strict-mcp-config` flag
- Test `--mcp-debug` flag
- Test tool call output formatting

---

## Key Implementation Details

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Failed to spawn MCP server: {0}")]
    SpawnError(String),

    #[error("MCP server process exited unexpectedly")]
    ProcessExited,

    #[error("JSON-RPC error {code}: {message}")]
    JsonRpcError { code: i64, message: String },

    #[error("MCP protocol error: {0}")]
    ProtocolError(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Server not found: {0}")]
    ServerNotFound(String),

    #[error("Not connected to server")]
    NotConnected,

    #[error("Request timeout after {0}ms")]
    Timeout(u64),

    #[error("Invalid config: {0}")]
    InvalidConfig(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Async/Sync Bridge

The `ToolExecutor` trait is synchronous, but MCP communication is async. Use `tokio::runtime::Handle::current().block_on()` to bridge:

```rust
fn execute(&self, ...) -> ToolExecutionResult {
    tokio::runtime::Handle::current().block_on(async {
        self.manager.read().call_tool(&call.tool, call.input.clone()).await
    })
}
```

### Process Lifecycle

MCP servers are spawned as child processes:
1. Inherit environment variables from `McpServerDef.env`
2. Set working directory from `McpServerDef.cwd`
3. Redirect stdin/stdout for JSON-RPC
4. stderr goes to parent's stderr (for debugging)
5. Graceful shutdown via MCP `shutdown` notification
6. SIGKILL if process doesn't exit within timeout

### Tool Name Collision

MCP tools may have the same name as built-in tools. Resolution order:
1. Check MCP servers first (user's configured tools take precedence)
2. Fall back to built-in tools

This allows users to override built-in behavior via MCP if desired.

## Verification Plan

### Unit Tests

1. **Transport layer** (`transport_tests.rs`)
   - JSON-RPC serialization/deserialization
   - Timeout handling
   - Error response parsing

2. **Protocol layer** (`protocol_tests.rs`)
   - MCP message serialization
   - Response parsing
   - Content block handling

3. **Client** (`client_tests.rs`)
   - Protocol flow (initialize → list_tools → call_tool → shutdown)
   - Error handling for each step

4. **Server/Manager** (`server_tests.rs`)
   - Tool discovery integration
   - Multi-server management
   - Tool routing

5. **Executor** (`mcp_executor_tests.rs`)
   - Tool routing to correct server
   - Fallback to builtin tools
   - Error result formatting

### Integration Tests

Create a simple echo MCP server for testing:

```python
#!/usr/bin/env python3
# tests/fixtures/echo_mcp_server.py
import json
import sys

def main():
    while True:
        line = sys.stdin.readline()
        if not line:
            break
        req = json.loads(line)

        if req["method"] == "initialize":
            resp = {"jsonrpc": "2.0", "id": req["id"], "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "serverInfo": {"name": "echo", "version": "1.0"}
            }}
        elif req["method"] == "tools/list":
            resp = {"jsonrpc": "2.0", "id": req["id"], "result": {
                "tools": [{"name": "echo", "description": "Echo input",
                          "inputSchema": {"type": "object"}}]
            }}
        elif req["method"] == "tools/call":
            resp = {"jsonrpc": "2.0", "id": req["id"], "result": {
                "content": [{"type": "text", "text": json.dumps(req["params"])}]
            }}
        else:
            resp = {"jsonrpc": "2.0", "id": req["id"], "result": {}}

        print(json.dumps(resp), flush=True)

if __name__ == "__main__":
    main()
```

Integration test:

```rust
#[tokio::test]
async fn test_mcp_end_to_end() {
    let def = McpServerDef {
        command: "python3".to_string(),
        args: vec!["tests/fixtures/echo_mcp_server.py".to_string()],
        ..Default::default()
    };

    let mut server = McpServer::from_def("echo", def);
    server.spawn().await.unwrap();

    assert!(server.is_running());
    assert_eq!(server.tools.len(), 1);
    assert_eq!(server.tools[0].name, "echo");

    let result = server.call_tool("echo", json!({"msg": "hello"})).await.unwrap();
    assert!(result.success);
}
```

### CLI Tests

```bash
# Test with echo MCP server
claudeless --mcp-config '{"mcpServers":{"echo":{"command":"python3","args":["echo_server.py"]}}}' \
    --print -p "test" --scenario test.toml

# Test strict mode with failing server
claudeless --mcp-config '{"mcpServers":{"bad":{"command":"nonexistent"}}}' \
    --strict-mcp-config --print -p "test"
# Should exit with error

# Test debug mode
claudeless --mcp-config '...' --mcp-debug --print -p "test"
# Should show MCP initialization messages
```

### Final Verification

Run full test suite:

```bash
make check
```

This validates:
- All unit tests pass
- All integration tests pass
- Linting (clippy) passes
- Formatting is correct
- No compile warnings
