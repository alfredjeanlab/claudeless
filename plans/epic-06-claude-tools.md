# Epic 6: Simulated Tool Execution

## Goal

Add tool execution capabilities to the scenario engine with three levels:
1. **Mock Results** - Return pre-configured results from scenario config
2. **Simulated Built-in Tools** - Configurable sandboxed execution of Bash, Read, Write, etc.
3. **Real MCP Server Execution** - Spawn actual MCP servers for integration testing

## Current State

- `ToolCallSpec` has `tool`, `input`, and `result` fields (result is unused)
- Tool calls are serialized to output but never executed
- `McpManager` loads config but never spawns servers
- Permission system exists but isn't wired to tool execution

---

## Phase 1: Tool Execution Foundation + Mock Results

Wire up the existing `result` field and create the execution engine abstraction.

**Files to modify:**
- `src/config.rs` - Add `ToolExecutionConfig` to `ScenarioConfig`
- `src/tools/mod.rs` (NEW) - Tool execution module exports
- `src/tools/executor.rs` (NEW) - `ToolExecutionEngine` trait + mock impl
- `src/tools/result.rs` (NEW) - `ToolExecutionResult` type
- `src/output.rs` - Add `write_tool_result()` for stream-json
- `src/main.rs` - Wire execution after tool_use output
- `src/cli.rs` - Add `--tool-execution-mode` flag

**Config structure:**
```toml
[tool_execution]
mode = "mock"  # mock | simulated | real_mcp | disabled

[[responses]]
pattern = { type = "contains", text = "list files" }
[responses.response]
text = "Here are the files:"
[[responses.response.tool_calls]]
tool = "Bash"
input = { command = "ls" }
result = "file1.txt\nfile2.txt"  # ← Already exists, just wire it up
```

**Execution engine trait:**
```rust
pub trait ToolExecutor: Send + Sync {
    fn execute(&self, call: &ToolCallSpec, ctx: &ExecutionContext) -> ToolExecutionResult;
}

pub struct MockExecutor;  // Returns call.result or error if missing
```

---

## Phase 2: Permission Integration

Wire permission checking into tool execution flow.

**Files to modify:**
- `src/tools/executor.rs` - Add permission check before execution
- `src/main.rs` - Create `PermissionChecker` from CLI args

**Flow:**
```
ToolCall → PermissionChecker.check(tool, action)
  → Allowed: Execute
  → Denied: Return error result
  → NeedsPrompt: Check bypass mode → execute or deny
```

---

## Phase 3: Simulated Built-in Tools

Implement sandboxed executors for Claude's built-in tools. Configurable per-scenario.

**New files:**
```
src/tools/builtin/
  mod.rs          - BuiltinToolExecutor trait, registry
  bash.rs         - Sandboxed or mock command execution
  read.rs         - File read within sandbox
  write.rs        - File write within sandbox
  edit.rs         - Search/replace in files
  glob.rs         - Pattern matching
  grep.rs         - Content search
```

**Config options:**
```toml
[tool_execution]
mode = "simulated"
sandbox_root = "/tmp/claudeless-sandbox"
allow_real_bash = false  # true = actually run commands
```

**Sandbox design:**
- All paths resolved relative to `sandbox_root`
- Path traversal prevented (`..` normalized, no escaping)
- Bash: allowlist of safe commands OR opt-in real execution

---

## Phase 4: Real MCP Server Execution

Implement full MCP protocol (JSON-RPC 2.0 over stdio) to spawn and communicate with real MCP servers.

**New files:**
```
src/tools/mcp/
  mod.rs          - Module exports
  protocol.rs     - JSON-RPC message types
  transport.rs    - stdio process spawn + message framing
  client.rs       - MCP client lifecycle
```

**MCP Protocol (per spec 2025-11-25):**

1. **Transport**: Newline-delimited JSON over stdio
   - Client spawns server as subprocess
   - Read JSON-RPC from stdout, write to stdin
   - Messages MUST NOT contain embedded newlines

2. **Lifecycle**:
   ```
   initialize → tools/list → tools/call* → (shutdown)
   ```

3. **Message formats:**
   ```rust
   // Initialize request
   { "jsonrpc": "2.0", "id": 1, "method": "initialize",
     "params": { "protocolVersion": "2025-11-25", "capabilities": {...}, "clientInfo": {...} }}

   // tools/list request
   { "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {} }

   // tools/call request
   { "jsonrpc": "2.0", "id": 3, "method": "tools/call",
     "params": { "name": "read_file", "arguments": { "path": "/tmp/test.txt" }}}

   // tools/call response
   { "jsonrpc": "2.0", "id": 3,
     "result": { "content": [{"type": "text", "text": "file contents"}], "isError": false }}
   ```

**Modify:**
- `src/mcp/server.rs` - Add `spawn()` method to `McpServer`
- `src/mcp/mod.rs` - Re-export client types

**Usage:**
```bash
claudeless --mcp-config fs.json --tool-execution-mode real_mcp -p "read /tmp/test"
```

---

## Phase 5: Output Stream Integration

Add `tool_result` content blocks to stream-json output, matching Claude's format.

**Stream sequence:**
```json
{"type":"content_block_start","index":0,"content_block":{"type":"text"}}
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"I'll read that file."}}
{"type":"content_block_stop","index":0}
{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_xxx","name":"read_file"}}
{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/tmp/test\"}"}}
{"type":"content_block_stop","index":1}
{"type":"tool_result","tool_use_id":"toolu_xxx","is_error":false,"content":[{"type":"text","text":"file contents"}]}
```

**Files to modify:**
- `src/output.rs` - Add `ToolResultBlock` struct and `write_tool_result()` method

---

## CLI Flags

```
--tool-execution-mode <MODE>    Override scenario's execution mode
                                [mock, simulated, real_mcp, disabled]
--sandbox-root <PATH>           Root directory for sandboxed execution
--allow-real-bash               Enable real Bash execution (requires simulated mode)
```

---

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Scenario + CLI config | Scenarios set defaults, CLI overrides for flexibility |
| `disabled` as default | Backward compatibility with existing tests |
| Sequential tool execution | Matches Claude's behavior, simpler to implement |
| Per-scenario bash mode | `allow_real_bash` can be true/false per scenario |
| Full MCP protocol | Required for proper server integration testing |

---

## Files Summary

| File | Change |
|------|--------|
| `src/config.rs` | Add `ToolExecutionConfig`, `ToolExecutionMode` |
| `src/cli.rs` | Add `--tool-execution-mode`, `--sandbox-root`, `--allow-real-bash` |
| `src/tools/mod.rs` | NEW - Module root, exports |
| `src/tools/executor.rs` | NEW - `ToolExecutor` trait, `MockExecutor` |
| `src/tools/result.rs` | NEW - `ToolExecutionResult` type |
| `src/tools/builtin/mod.rs` | NEW - Built-in executor registry |
| `src/tools/builtin/bash.rs` | NEW - Bash executor (mock + sandboxed) |
| `src/tools/builtin/read.rs` | NEW - Read file executor |
| `src/tools/builtin/write.rs` | NEW - Write file executor |
| `src/tools/mcp/mod.rs` | NEW - MCP client module |
| `src/tools/mcp/protocol.rs` | NEW - JSON-RPC types |
| `src/tools/mcp/transport.rs` | NEW - stdio spawn + framing |
| `src/tools/mcp/client.rs` | NEW - MCP client lifecycle |
| `src/mcp/server.rs` | Add `spawn()` to `McpServer` |
| `src/output.rs` | Add `ToolResultBlock`, `write_tool_result()` |
| `src/main.rs` | Wire execution engine into response flow |

---

## Verification

1. **Unit tests**: Each executor mode (mock, simulated, real_mcp)
2. **Mock mode**: Returns configured `result` values
3. **Simulated mode**: Executes within sandbox boundaries
4. **Permission integration**: Modes block/allow appropriately
5. **Real MCP**: Spawn filesystem server, call read_file
6. **Output format**: `tool_result` blocks appear in stream-json
7. **`make check`** passes
