# Implementation Plan: McpServer and McpManager Integration

## Overview

Update `McpServer` and `McpManager` to use the real `McpClient` for spawning MCP server processes, discovering tools via `tools/list`, and routing tool calls. This connects the existing stub implementations to the working client/transport/protocol layers.

## Project Structure

```
crates/cli/src/mcp/
├── server.rs        # UPDATE - add client field, async spawn/call_tool/shutdown
├── server_tests.rs  # UPDATE - add integration tests with echo server
└── mod.rs           # No changes needed (exports already present)
```

## Dependencies

No new dependencies required. Uses existing:
- `tokio` (async runtime, `sync::Mutex`)
- `parking_lot` or std `Arc` for shared ownership
- Existing `McpClient`, `ToolInfo`, protocol types

## Implementation Phases

### Phase 1: Add Client Field to McpServer

**Goal:** Store a live `McpClient` connection in `McpServer`.

**Changes to `server.rs`:**

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use super::client::{ClientError, McpClient};

/// MCP server with optional live client connection.
pub struct McpServer {
    pub name: String,
    pub definition: McpServerDef,
    pub tools: Vec<McpToolDef>,
    pub status: McpServerStatus,
    /// Live client connection (None until spawned, or in mock mode).
    client: Option<Arc<Mutex<McpClient>>>,
}
```

**Note:** `McpServer` can no longer derive `Clone` since `McpClient` owns a child process. Update struct to not derive `Clone`, or make `client` field skipped for clone.

**Verification:**
- Compiles without errors
- Existing tests still pass

---

### Phase 2: Implement Async spawn() Method

**Goal:** Replace stub `spawn()` with real process spawning via `McpClient`.

**Changes to `McpServer`:**

```rust
impl McpServer {
    /// Spawn the MCP server process and initialize the connection.
    ///
    /// This spawns the actual MCP server process, initializes the MCP protocol,
    /// and discovers available tools via `tools/list`.
    pub async fn spawn(&mut self) -> Result<(), ClientError> {
        // Validate definition
        if self.definition.command.is_empty() {
            return Err(ClientError::Transport(
                TransportError::SpawnFailed("No command specified".into())
            ));
        }

        // Connect, initialize, and discover tools
        let client = McpClient::connect_and_initialize(&self.definition).await?;

        // Convert discovered tools to McpToolDef
        self.tools = client
            .tools()
            .iter()
            .map(|t| t.clone().into_tool_def(&self.name))
            .collect();

        self.client = Some(Arc::new(Mutex::new(client)));
        self.status = McpServerStatus::Running;
        Ok(())
    }

    /// Check if the server has an active client connection.
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Get the client (for internal use).
    pub(crate) fn client(&self) -> Option<&Arc<Mutex<McpClient>>> {
        self.client.as_ref()
    }
}
```

**Verification:**
- Unit test with mock (skip actual spawn)
- Integration test with echo MCP server

---

### Phase 3: Implement call_tool() on McpServer

**Goal:** Route tool calls through the live client connection.

**Changes to `McpServer`:**

```rust
impl McpServer {
    /// Execute a tool call on this server.
    ///
    /// Returns error if server is not connected or tool execution fails.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, ClientError> {
        let client = self.client.as_ref().ok_or(ClientError::NotInitialized)?;
        let guard = client.lock().await;
        let result = guard.call_tool(name, arguments).await?;
        Ok(result.into_tool_result())
    }

    /// Execute a tool call with custom timeout.
    pub async fn call_tool_with_timeout(
        &self,
        name: &str,
        arguments: serde_json::Value,
        timeout_ms: u64,
    ) -> Result<McpToolResult, ClientError> {
        let client = self.client.as_ref().ok_or(ClientError::NotInitialized)?;
        let guard = client.lock().await;
        let result = guard.call_tool_with_timeout(name, arguments, timeout_ms).await?;
        Ok(result.into_tool_result())
    }
}
```

**Verification:**
- Test `call_tool()` returns error when not connected
- Integration test with echo server returning expected result

---

### Phase 4: Implement shutdown() on McpServer

**Goal:** Graceful shutdown of the client connection.

**Changes to `McpServer`:**

```rust
impl McpServer {
    /// Shutdown the server connection gracefully.
    ///
    /// Takes ownership of the client and shuts it down. After this call,
    /// the server status changes to Disconnected and `call_tool` will fail.
    pub async fn shutdown(&mut self) -> Result<(), ClientError> {
        if let Some(client_arc) = self.client.take() {
            // Try to unwrap the Arc; if other references exist, we can't shutdown cleanly
            match Arc::try_unwrap(client_arc) {
                Ok(mutex) => {
                    let client = mutex.into_inner();
                    client.shutdown().await?;
                }
                Err(_arc) => {
                    // Other references exist; just drop our handle
                    // The process will be killed when all references are dropped
                }
            }
        }
        self.status = McpServerStatus::Disconnected;
        Ok(())
    }
}
```

**Verification:**
- Test shutdown changes status to Disconnected
- Test `call_tool` after shutdown returns error

---

### Phase 5: Update McpManager with initialize() and shutdown()

**Goal:** Add async `initialize()` to spawn all servers and `shutdown()` for cleanup.

**Changes to `McpManager`:**

```rust
impl McpManager {
    /// Initialize all servers by spawning their processes.
    ///
    /// Returns a list of (server_name, result) pairs. Servers that fail to
    /// initialize are marked as Failed but remain in the manager.
    pub async fn initialize(&mut self) -> Vec<(String, Result<(), ClientError>)> {
        let mut results = Vec::new();

        // Collect server names to avoid borrow issues
        let names: Vec<String> = self.servers.keys().cloned().collect();

        for name in names {
            let result = if let Some(server) = self.servers.get_mut(&name) {
                match server.spawn().await {
                    Ok(()) => {
                        // Register discovered tools in the mapping
                        for tool in &server.tools {
                            self.tool_server_map.insert(tool.name.clone(), name.clone());
                        }
                        Ok(())
                    }
                    Err(e) => {
                        server.status = McpServerStatus::Failed(e.to_string());
                        Err(e)
                    }
                }
            } else {
                continue;
            };
            results.push((name, result));
        }

        results
    }

    /// Execute a tool call, routing to the appropriate server.
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, ClientError> {
        let server_name = self.tool_server_map.get(tool_name)
            .ok_or_else(|| ClientError::ToolNotFound(tool_name.to_string()))?;
        let server = self.servers.get(server_name)
            .ok_or_else(|| ClientError::ToolNotFound(format!("server '{}' not found", server_name)))?;
        server.call_tool(tool_name, arguments).await
    }

    /// Shutdown all server connections gracefully.
    pub async fn shutdown(&mut self) {
        for server in self.servers.values_mut() {
            let _ = server.shutdown().await;
        }
    }
}
```

**Note:** Update `from_config()` to NOT auto-start servers. Initialization should be explicit:

```rust
/// Initialize from config (does not spawn servers).
///
/// Call [`initialize()`](Self::initialize) to spawn server processes.
pub fn from_config(config: &McpConfig) -> Self {
    let mut manager = Self::new();

    for (name, def) in &config.mcp_servers {
        let server = McpServer::from_def(name, def.clone());
        // Don't auto-start; let caller call initialize()
        manager.servers.insert(name.clone(), server);
    }

    manager
}
```

**Verification:**
- Test `initialize()` spawns all servers
- Test `initialize()` handles partial failures
- Test `call_tool()` routes correctly
- Test `shutdown()` disconnects all servers

---

### Phase 6: Add Integration Tests

**Goal:** Verify end-to-end functionality with a real MCP server.

**Test file:** `server_tests.rs`

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::path::PathBuf;

    fn echo_server_def() -> McpServerDef {
        // Use the test echo server script
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/echo_mcp_server.py");
        McpServerDef {
            command: "python3".into(),
            args: vec![script.to_string_lossy().to_string()],
            timeout_ms: 5000,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_server_spawn_and_tool_discovery() {
        let mut server = McpServer::from_def("echo", echo_server_def());
        assert_eq!(server.status, McpServerStatus::Uninitialized);

        server.spawn().await.expect("spawn failed");

        assert_eq!(server.status, McpServerStatus::Running);
        assert!(server.is_connected());
        assert!(!server.tools.is_empty());
        assert!(server.tools.iter().any(|t| t.name == "echo"));

        server.shutdown().await.expect("shutdown failed");
        assert_eq!(server.status, McpServerStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_server_call_tool() {
        let mut server = McpServer::from_def("echo", echo_server_def());
        server.spawn().await.expect("spawn failed");

        let result = server
            .call_tool("echo", serde_json::json!({"message": "hello"}))
            .await
            .expect("call_tool failed");

        assert!(result.success);
        server.shutdown().await.ok();
    }

    #[tokio::test]
    async fn test_manager_initialize_and_call() {
        let config = McpConfig::parse(&format!(
            r#"{{"mcpServers": {{"echo": {{"command": "python3", "args": ["{}"]}}}}}}"#,
            echo_server_def().args[0]
        )).unwrap();

        let mut manager = McpManager::from_config(&config);
        assert_eq!(manager.running_server_count(), 0);

        let results = manager.initialize().await;
        assert!(results.iter().all(|(_, r)| r.is_ok()));
        assert_eq!(manager.running_server_count(), 1);
        assert!(manager.has_tool("echo"));

        let result = manager
            .call_tool("echo", serde_json::json!({"msg": "test"}))
            .await
            .expect("call failed");
        assert!(result.success);

        manager.shutdown().await;
    }
}
```

**Verification:**
- All integration tests pass
- `make check` passes

## Key Implementation Details

### Async/Sync Considerations

- `McpServer::spawn()`, `call_tool()`, and `shutdown()` are all `async`
- `McpManager::initialize()`, `call_tool()`, and `shutdown()` are all `async`
- Callers must be in an async context (tokio runtime)

### Error Handling

Reuse `ClientError` from the client module for consistency:
- `ClientError::NotInitialized` - server not spawned
- `ClientError::ToolNotFound` - tool not registered
- `ClientError::Transport(...)` - spawn/communication failures

### Clone Behavior

`McpServer` cannot implement `Clone` after this change because `McpClient` owns a child process. Options:
1. Remove `#[derive(Clone)]` from `McpServer`
2. Make `client` field `#[skip]` for clone (always `None` in cloned copies)

Recommendation: Remove `Clone` derive. Servers with live connections shouldn't be cloned.

### Tool Registration Timing

Tools are registered in `tool_server_map` during `initialize()`, not `from_config()`. This ensures only actually-discovered tools from running servers are registered.

## Verification Plan

1. **Unit tests** (no external processes):
   - `McpServer::from_def()` creates uninitialized server
   - `call_tool()` returns error when not connected
   - Status transitions work correctly

2. **Integration tests** (with echo server):
   - `spawn()` connects and discovers tools
   - `call_tool()` routes and returns results
   - `shutdown()` cleans up properly
   - `McpManager::initialize()` spawns all servers
   - `McpManager::call_tool()` routes correctly

3. **Full verification**:
   ```bash
   make check
   ```
