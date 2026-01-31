# MCP Init Event Gaps Implementation Plan

## Overview

Fix three MCP init event gaps to match real Claude CLI output format:
1. Change `mcp_servers` from string array to object array with `name` and `status` fields
2. Include MCP tools in the `tools` array using qualified `mcp__server__tool` names
3. Add JSON-RPC debug logging when `--mcp-debug` is enabled

## Project Structure

Key files to modify:
```
crates/cli/src/
├── output_events.rs   # SystemInitEvent struct - mcp_servers type change
├── output.rs          # write_real_response_with_mcp signature changes
├── main.rs            # Combine builtin + MCP tools, extract server statuses
└── mcp/
    ├── transport.rs   # Add debug logging for JSON-RPC messages
    └── server.rs      # (read-only) McpServerStatus enum reference
```

Test files:
```
crates/cli/src/
└── output_tests.rs    # Unskip test_mcp_servers_format_matches_real_claude
```

## Dependencies

No new dependencies required. Uses existing:
- `serde` for serialization
- `serde_json` for JSON handling

## Implementation Phases

### Phase 1: mcp_servers Object Format

**Goal:** Change `mcp_servers` from `Vec<String>` to `Vec<McpServerInfo>` with name/status.

**Files:** `output_events.rs`, `output.rs`

1. Add `McpServerInfo` struct in `output_events.rs`:
```rust
/// MCP server info for init event (matches real Claude CLI format)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct McpServerInfo {
    pub name: String,
    pub status: String,  // "connected", "failed", "disconnected"
}
```

2. Update `SystemInitEvent` struct:
```rust
pub struct SystemInitEvent {
    // ... existing fields ...
    pub mcp_servers: Vec<McpServerInfo>,  // Changed from Vec<String>
}
```

3. Update `SystemInitEvent::new()` and `with_mcp_servers()` to accept `Vec<McpServerInfo>`

4. Update `output.rs` function signatures:
   - `write_real_response_with_mcp()`
   - `write_real_stream_json()`

**Verification:** Run `cargo test output_tests::test_mcp_servers_format_matches_real_claude`

### Phase 2: MCP Tools in Init Event

**Goal:** Include MCP tools in the `tools` array alongside builtin tools.

**Files:** `main.rs` (around line 184)

1. Add helper function to extract qualified tool names from MCP manager:
```rust
fn get_mcp_tool_names(mcp_manager: &Option<Arc<RwLock<McpManager>>>) -> Vec<String> {
    match mcp_manager {
        Some(manager) => {
            let guard = manager.read();
            guard.all_tools()
                .iter()
                .map(|tool| tool.qualified_name())
                .collect()
        }
        None => vec![],
    }
}
```

2. Add helper to extract server info:
```rust
fn get_mcp_server_info(mcp_manager: &Option<Arc<RwLock<McpManager>>>) -> Vec<McpServerInfo> {
    match mcp_manager {
        Some(manager) => {
            let guard = manager.read();
            guard.servers()
                .iter()
                .filter(|s| s.status == McpServerStatus::Running)
                .map(|s| McpServerInfo {
                    name: s.name.clone(),
                    status: "connected".to_string(),
                })
                .collect()
        }
        None => vec![],
    }
}
```

3. Modify main.rs output section (line ~184):
```rust
// Combine builtin tools with MCP tools
let mut tools: Vec<String> = cli.allowed_tools.clone();
let mcp_tools = get_mcp_tool_names(&mcp_manager);
tools.extend(mcp_tools);

// Get MCP server info
let mcp_servers = get_mcp_server_info(&mcp_manager);

// Use write_real_response_with_mcp
writer.write_real_response_with_mcp(&response, &session_ctx.session_id.to_string(), tools, mcp_servers)?;
```

**Verification:**
- Run `cargo test output_tests::test_init_event_tools_includes_builtin_and_mcp`
- Manual test: `claudeless --mcp-config mcp.json -p "list tools" -o stream-json | jq '.tools'`

### Phase 3: JSON-RPC Debug Logging

**Goal:** Log JSON-RPC request/response messages when `--mcp-debug` is enabled.

**Files:** `transport.rs`, `main.rs`, `server.rs`

1. Add debug flag to `StdioTransport`:
```rust
pub struct StdioTransport {
    // ... existing fields ...
    debug: bool,
}

impl StdioTransport {
    pub fn new(child: Child, debug: bool) -> Self {
        // ... use debug flag ...
    }
}
```

2. Add debug logging in `send()`:
```rust
pub async fn send(&self, request: &JsonRpcRequest) -> Result<(), TransportError> {
    if self.debug {
        eprintln!("MCP JSON-RPC -> {}: {}",
            request.method,
            serde_json::to_string(request).unwrap_or_default()
        );
    }
    self.write_message(request).await
}
```

3. Add debug logging in `receive()`:
```rust
pub async fn receive(&self) -> Result<JsonRpcResponse, TransportError> {
    // ... existing code ...
    let response = serde_json::from_str(&line)?;
    if self.debug {
        eprintln!("MCP JSON-RPC <- {}", line.trim());
    }
    Ok(response)
}
```

4. Thread debug flag through:
   - `McpClient::new()` accepts debug flag
   - `McpServer::spawn()` passes debug flag from CLI
   - `McpManager::initialize()` receives debug flag

**Verification:**
- Run `claudeless --mcp-config mcp.json --mcp-debug -p "read file"`
- Should see JSON-RPC messages in stderr

### Phase 4: Testing and Cleanup

**Goal:** Enable ignored tests and verify all changes work together.

1. Remove `#[ignore]` from `test_mcp_servers_format_matches_real_claude`

2. Update the test to use the new signature:
```rust
#[test]
fn test_mcp_servers_format_matches_real_claude() {
    let expected_format = serde_json::json!([
        {"name": "filesystem", "status": "connected"}
    ]);

    let mcp_servers = vec![McpServerInfo {
        name: "filesystem".to_string(),
        status: "connected".to_string(),
    }];

    let init = SystemInitEvent::with_mcp_servers("session-123", vec![], mcp_servers);
    // ... rest of test ...
}
```

3. Run full test suite: `make check`

4. Update TEST_MCP.md to mark gaps as resolved

**Verification:** `make check` passes

## Key Implementation Details

### Status Mapping

Map `McpServerStatus` enum to string for JSON output:
- `McpServerStatus::Running` -> `"connected"`
- `McpServerStatus::Failed(_)` -> `"failed"`
- `McpServerStatus::Disconnected` -> `"disconnected"`
- `McpServerStatus::Uninitialized` -> (not included in output)

### Tool Ordering

The tools array should maintain ordering:
1. Builtin tools first (from `cli.allowed_tools`)
2. MCP tools after (in server discovery order)

### Thread Safety

MCP manager is wrapped in `Arc<RwLock<>>`. Access with `.read()` for the helpers since we only need read access.

## Verification Plan

1. **Unit tests:**
   - `cargo test output_tests::test_mcp_servers_format_matches_real_claude`
   - `cargo test output_tests::test_init_event_tools_includes_builtin_and_mcp`

2. **Integration tests:**
   - `cargo test mcp_config` - existing MCP tests still pass

3. **Manual verification:**
```bash
# Verify mcp_servers format
claudeless --mcp-config mcp.json -p "hello" -o stream-json | head -1 | jq '.mcp_servers'
# Expected: [{"name": "filesystem", "status": "connected"}]

# Verify tools array includes MCP tools
claudeless --mcp-config mcp.json -p "hello" -o stream-json | head -1 | jq '.tools'
# Expected: ["Read", "Write", ..., "mcp__filesystem__read_file", ...]

# Verify JSON-RPC debug output
claudeless --mcp-config mcp.json --mcp-debug -p "read sample.txt" 2>&1 | grep "JSON-RPC"
# Expected: MCP JSON-RPC -> initialize: {...}
```

4. **Full check:**
```bash
make check
```
