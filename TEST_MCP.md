# MCP Exploratory Testing Guide

Manual testing guide for validating claudeless MCP server support using the filesystem MCP server.

## Setup

### 1. Install the filesystem MCP server

```bash
npm install -g @modelcontextprotocol/server-filesystem
```

### 2. Create a test directory

```bash
mkdir -p /tmp/mcp-test
echo "hello world" > /tmp/mcp-test/sample.txt
```

### 3. Create MCP config

Create `mcp-test-config.json`:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp/mcp-test"]
    }
  }
}
```

## Test Scenarios

### T1: Server Initialization

**Goal:** Verify claudeless spawns the MCP server and discovers tools.

```bash
claudeless --mcp-config mcp-test-config.json --mcp-debug -p "list your available tools"
```

**Verify:**
- [x] Server starts without errors
- [x] Tools are discovered (read_file, write_file, list_directory, etc.)
- [x] Tool names and descriptions appear in output

**Automated tests:** `tests/mcp_config.rs::test_mcp_config_flag_accepted`, `test_mcp_debug_flag`

---

### T2: Read File

**Goal:** Verify tool calls work end-to-end.

```bash
claudeless --mcp-config mcp-test-config.json -p "read the file sample.txt"
```

**Verify:**
- [ ] Claude calls the read_file tool
- [ ] Content "hello world" appears in response
- [ ] Tool call shows in output (if using stream-json or verbose mode)

**Automated tests:** `tests/mcp_scenarios.rs::mcp_read_scenario` (mock mode only)

---

### T3: Write File

**Goal:** Verify write operations work.

```bash
claudeless --mcp-config mcp-test-config.json -p "create a file called test-output.txt with the content 'written by mcp'"
```

**Verify:**
- [ ] Tool call executes
- [ ] File exists: `cat /tmp/mcp-test/test-output.txt`
- [ ] Content matches

**Automated tests:** `tests/mcp_scenarios.rs::mcp_write_scenario` (mock mode only)

---

### T4: List Directory

**Goal:** Verify directory listing.

```bash
claudeless --mcp-config mcp-test-config.json -p "list all files in the directory"
```

**Verify:**
- [ ] Shows sample.txt and test-output.txt
- [ ] Format is reasonable

**Automated tests:** `tests/mcp_scenarios.rs::mcp_list_scenario` (mock mode only)

---

### T5: Multi-tool Conversation

**Goal:** Verify stateful conversation with multiple tool calls.

```bash
claudeless --mcp-config mcp-test-config.json
```

Then interactively:
```
> list the files
> read sample.txt
> append " - modified" to sample.txt
> read it again to confirm
```

**Verify:**
- [ ] Each tool call works in sequence
- [ ] State persists between calls
- [ ] Claude correctly interprets results

**Automated tests:** None (requires interactive mode)

---

### T6: Error Handling

**Goal:** Verify errors are handled gracefully.

```bash
claudeless --mcp-config mcp-test-config.json -p "read /etc/passwd"
```

**Verify:**
- [ ] Error is returned (path outside allowed directory)
- [ ] Error message is clear
- [ ] claudeless doesn't crash

**Automated tests:** None (requires live MCP server error response)

---

### T7: Strict Mode

**Goal:** Verify --strict-mcp-config flag behavior.

```bash
# Should fail immediately
claudeless --mcp-config '{"mcpServers":{"bad":{"command":"nonexistent"}}}' \
  --strict-mcp-config -p "hello"
```

**Verify:**
- [x] Exits with error before conversation starts
- [x] Error message identifies which server failed

**Automated tests:** `tests/mcp_config.rs::test_strict_mcp_config_flag`

---

### T8: Debug Mode

**Goal:** Verify --mcp-debug shows useful information.

```bash
claudeless --mcp-config mcp-test-config.json --mcp-debug -p "read sample.txt"
```

**Verify:**
- [x] Shows server spawn information
- [x] Shows tool discovery
- [ ] Shows JSON-RPC messages (not implemented)

**Automated tests:** `tests/mcp_config.rs::test_mcp_debug_flag`

---

## Using Scenarios for Scripted MCP Testing

Claudeless scenarios (see [docs/SCENARIOS.md](docs/SCENARIOS.md)) can be used to create deterministic, repeatable MCP tests without requiring live MCP servers.

### Simulating MCP Tool Calls

Use the `tool_calls` field in responses to simulate MCP tool invocations:

```toml
# mcp-read-scenario.toml
name = "mcp-read-test"

[[responses]]
pattern = { type = "contains", text = "read" }

[responses.response]
text = "Here's the file content:"

[[responses.response.tool_calls]]
tool = "mcp__filesystem__read_file"
input = { path = "/tmp/mcp-test/sample.txt" }
result = "hello world"
```

Run with:

```bash
claudeless --scenario mcp-read-scenario.toml -p "read sample.txt"
```

### Tool Execution Modes

The `tool_execution` section controls how tools are executed:

```toml
[tool_execution]
mode = "mock"  # or "live" or "disabled"

[tool_execution.tools."mcp__filesystem__read_file"]
auto_approve = true
result = "canned file contents"

[tool_execution.tools."mcp__filesystem__write_file"]
auto_approve = false
error = "Permission denied"
```

| Mode | Description |
|------|-------------|
| `disabled` | No tool execution (default) - returns canned response only |
| `mock` | Return pre-configured results from scenario |
| `live` | Execute tools directly (requires `--tool-mode live`) |

### Multi-Turn MCP Scenarios

Use `turns` for multi-step MCP interactions:

```toml
[[responses]]
pattern = { type = "contains", text = "list and read" }
response = "I'll list the files first."

[[responses.response.tool_calls]]
tool = "mcp__filesystem__list_directory"
input = { path = "/tmp/mcp-test" }
result = "[FILE] sample.txt\n[FILE] other.txt"

turns = [
    { expect = { type = "any" }, response = "Now reading sample.txt..." }
]
```

### Combining Scenarios with Live MCP

For integration testing, combine scenarios with live MCP servers:

```bash
claudeless \
  --scenario mcp-integration.toml \
  --mcp-config mcp-test-config.json \
  --tool-mode live \
  -p "read sample.txt"
```

This allows scenarios to control response text while MCP servers handle actual tool execution.

---

## Test Coverage Summary

### Automated Test Coverage

| Area | File | Coverage |
|------|------|----------|
| Config loading | `tests/mcp_config.rs` | File, inline JSON, multiple configs, JSON5 |
| Config errors | `tests/mcp_config.rs` | Nonexistent file, invalid JSON, empty config |
| Strict mode | `tests/mcp_config.rs` | Fail-fast on bad server |
| Debug flag | `tests/mcp_config.rs` | Flag acceptance |
| Scenario init | `tests/mcp_scenarios.rs` | Tool listing with `mcp__` prefix |
| Scenario read | `tests/mcp_scenarios.rs` | Mock mode, response text |
| Scenario write | `tests/mcp_scenarios.rs` | Mock mode, response text |
| Scenario list | `tests/mcp_scenarios.rs` | Mock mode, response text |
| Tool modes | `tests/mcp_scenarios.rs` | disabled, mock, live routing |
| Qualified names | `src/tools/mcp_executor_tests.rs` | Parsing, routing |
| MCP config parsing | `src/mcp/config_tests.rs` | JSON parsing, merging, env vars, timeouts |
| MCP protocol | `src/mcp/protocol_tests.rs` | JSON-RPC serialization |
| MCP client | `src/mcp/client_tests.rs` | Client lifecycle |
| MCP server | `src/mcp/server_tests.rs` | Server management |
| MCP tools | `src/mcp/tools_tests.rs` | Tool definitions |

### Not Covered by Automated Tests

| Area | Reason |
|------|--------|
| Live MCP tool execution | Requires running MCP server |
| MCP error responses | Requires live server to trigger errors |
| Multi-turn interactive | Requires interactive mode testing |
| JSON-RPC debug output | Not implemented |
| `mcp_servers` in init event | Known gap in implementation |
| `tools` array with MCP tools in init | Known gap in implementation |

---

## Cleanup

```bash
rm -rf /tmp/mcp-test
rm mcp-test-config.json
```
