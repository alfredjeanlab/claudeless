# Implementation Plan: MCP Final Cleanup

## Overview

Final cleanup pass for MCP implementation: fix build errors, close test coverage gaps, add comprehensive integration tests with the echo server, review tech debt across all MCP modules, apply DRY improvements, ensure `make check` passes, and update LIMITATIONS.md to reflect new MCP capabilities.

## Project Structure

```
crates/cli/src/mcp/
├── client.rs           # MCP client lifecycle
├── client_tests.rs     # Client unit tests
├── transport.rs        # JSON-RPC stdio transport
├── transport_tests.rs  # Transport unit tests
├── protocol.rs         # MCP protocol types
├── protocol_tests.rs   # Protocol unit tests
├── server.rs           # McpServer and McpManager
├── server_tests.rs     # Server unit + integration tests
├── config.rs           # Configuration parsing
├── config_tests.rs     # Config unit tests
├── tools.rs            # Tool definitions
├── tools_tests.rs      # Tools unit tests
└── mod.rs              # Module exports

tests/fixtures/
└── echo_mcp_server.py  # Test MCP server (Python)

docs/LIMITATIONS.md     # Main limitations doc (needs update)
crates/cli/docs/LIMITATIONS.md  # Duplicate (sync with main)
```

## Dependencies

No new dependencies required.

## Implementation Phases

### Phase 1: Fix Build Error

**Goal:** Fix the compilation error preventing `make check` from running.

**File:** `crates/cli/src/mcp/client_tests.rs:36`

**Issue:** Test references `ClientError::ToolError` which doesn't exist in `ClientError` enum.

**Fix:** Remove the non-existent variant from the error display test:

```rust
// Before (line 32-39)
let errors = vec![
    ClientError::NotInitialized,
    ClientError::AlreadyInitialized,
    ClientError::ToolNotFound("missing".into()),
    ClientError::ToolError("failed".into()),  // REMOVE - doesn't exist
    ClientError::InvalidResponse("bad json".into()),
    ClientError::UnsupportedVersion("1.0".into()),
];

// After
let errors = vec![
    ClientError::NotInitialized,
    ClientError::AlreadyInitialized,
    ClientError::ToolNotFound("missing".into()),
    ClientError::InvalidResponse("bad json".into()),
    ClientError::UnsupportedVersion("1.0".into()),
];
```

**Note:** If `ToolError` should exist (for tool-level errors distinct from `ToolNotFound`), add it to `ClientError` instead. Based on the current implementation, tool errors are returned in `ToolCallResult.is_error` field, not as a `ClientError`.

**Verification:**
```bash
cargo build --all
cargo test -p claudeless mcp::client::tests::error_types
```

---

### Phase 2: Test Coverage Audit

**Goal:** Identify and document test coverage gaps across all MCP modules.

**Audit each module for missing test scenarios:**

#### client.rs / client_tests.rs
Current coverage is good. Potential additions:
- [ ] Test `call_tool` with very large arguments (JSON size limits)
- [ ] Test behavior when server responds with malformed JSON
- [ ] Test concurrent `list_tools` and `call_tool` calls

#### transport.rs / transport_tests.rs
Current coverage is comprehensive. Potential additions:
- [ ] Test `request` timeout with actual slow server (not just mock)
- [ ] Test behavior when server closes stdout mid-response

#### protocol.rs / protocol_tests.rs
Current coverage is good. Potential additions:
- [ ] Test edge cases: empty tool name, empty input schema
- [ ] Test ContentBlock with unknown type (forward compatibility)

#### server.rs / server_tests.rs
Current coverage includes integration tests. Potential additions:
- [ ] Test `McpManager::initialize()` with one failing, one succeeding server
- [ ] Test tool name collision across servers
- [ ] Test server timeout during spawn

#### config.rs / config_tests.rs
Current coverage is good. Potential additions:
- [ ] Test very long command/args strings
- [ ] Test invalid JSON5 syntax (error messages)

#### tools.rs / tools_tests.rs
Current coverage is basic. Potential additions:
- [ ] Test `McpToolResult` edge cases
- [ ] Test template generation with edge case inputs

**Verification:**
```bash
cargo test -p claudeless mcp
```

---

### Phase 3: Add Missing Tests

**Goal:** Implement the most valuable missing tests identified in Phase 2.

#### 3a: Add `McpManager` partial failure test

**File:** `crates/cli/src/mcp/server_tests.rs`

```rust
#[tokio::test]
async fn test_manager_initialize_partial_failure() {
    // Create config with one valid server (echo) and one invalid (bad command)
    let script = echo_server_path();
    let config = McpConfig::parse(&format!(
        r#"{{
            "mcpServers": {{
                "echo": {{"command": "python3", "args": ["{}"]}},
                "bad": {{"command": "nonexistent_xyz_123", "args": []}}
            }}
        }}"#,
        script
    ))
    .unwrap();

    let mut manager = McpManager::from_config(&config);
    let results = manager.initialize().await;

    // One should succeed, one should fail
    assert_eq!(results.len(), 2);
    let successes: Vec<_> = results.iter().filter(|(_, r)| r.is_ok()).collect();
    let failures: Vec<_> = results.iter().filter(|(_, r)| r.is_err()).collect();

    assert_eq!(successes.len(), 1);
    assert_eq!(failures.len(), 1);

    // Echo tools should be available
    assert!(manager.has_tool("echo"));

    // Failed server should be marked as Failed
    let bad_server = manager.get_server("bad").unwrap();
    assert!(matches!(bad_server.status, McpServerStatus::Failed(_)));

    manager.shutdown().await;
}
```

#### 3b: Add malformed response test

**File:** `crates/cli/src/mcp/transport_tests.rs`

```rust
#[tokio::test]
async fn test_malformed_json_response() {
    // Create a server that returns invalid JSON
    let script = r#"
import sys
for line in sys.stdin:
    # Return malformed JSON
    print('{"jsonrpc": "2.0", "id": 1, "result": INVALID}', flush=True)
    break
"#;
    // Write script to temp file and test
    // (Implementation similar to existing tests)
}
```

**Verification:**
```bash
cargo test -p claudeless mcp::server::tests::integration_tests::test_manager_initialize_partial_failure
```

---

### Phase 4: Tech Debt Review

**Goal:** Review and document remaining tech debt in MCP modules.

#### 4a: Audit for DRY violations

**Check for repeated patterns:**

1. **Serialization helpers** - Already extracted in `mcp-cleanup-1`:
   - `serialize_params()` and `deserialize_response()` in `client.rs`

2. **Shutdown check** - Already extracted:
   - `require_not_shutdown()` in `transport.rs`

3. **State guards** - Already extracted:
   - `require_initialized()` in `client.rs`

4. **Potential new DRY opportunities:**
   - `echo_server_def()` helper duplicated in `client_tests.rs` and `server_tests.rs`
   - Extract to shared test utilities

**Action:** Create shared test fixture helper:

```rust
// crates/cli/src/mcp/test_fixtures.rs (or in mod.rs under #[cfg(test)])
#[cfg(test)]
pub(crate) fn echo_server_def() -> McpServerDef {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let script = format!("{}/../../tests/fixtures/echo_mcp_server.py", manifest_dir);
    McpServerDef {
        command: "python3".into(),
        args: vec![script],
        timeout_ms: 5000,
        ..Default::default()
    }
}
```

#### 4b: Review error handling

All error types use `thiserror` consistently. Check for:
- [ ] Unused error variants (clippy dead_code)
- [ ] Error messages that could be more descriptive
- [ ] Proper `#[from]` usage for error conversion

#### 4c: Review documentation

Check for:
- [ ] Public types without doc comments
- [ ] Examples that may be outdated
- [ ] Module-level documentation

**Verification:**
```bash
cargo doc --no-deps -p claudeless
# Check for warnings in output
```

---

### Phase 5: Update LIMITATIONS.md

**Goal:** Update documentation to reflect implemented MCP capabilities.

**Files:**
- `docs/LIMITATIONS.md` (main)
- `crates/cli/docs/LIMITATIONS.md` (duplicate - keep in sync)

**Current MCP section (outdated):**
```markdown
## MCP

| Feature | Status |
|---------|--------|
| Actual server execution | Not implemented (returns stub) |
| Dynamic tool discovery | Not implemented (manual registration) |
| Server health checks | Not implemented (always "running") |
```

**Updated MCP section:**
```markdown
## MCP

| Feature | Status |
|---------|--------|
| Server spawning | ✓ Implemented via `McpClient::connect()` |
| Protocol initialization | ✓ Implemented (`initialize` + `notifications/initialized`) |
| Dynamic tool discovery | ✓ Implemented via `tools/list` |
| Tool execution | ✓ Implemented via `tools/call` |
| Graceful shutdown | ✓ Implemented with timeout and force-kill |
| Multi-server management | ✓ Implemented via `McpManager` |
| Tool routing | ✓ Implemented (tool → server mapping) |
| Resources protocol | Not implemented (tools only) |
| Prompts protocol | Not implemented (tools only) |
| Server health checks | Basic (process exit detection) |

### MCP Configuration

Supported config formats:
- JSON (`.json`)
- JSON5 (`.json5`) with comments

Supported flags:
- `--mcp-config <path>` - Load MCP config file
- `--strict-mcp-config` - Fail on invalid config
- `--mcp-debug` - Enable MCP debug output
```

**Also update CLI Flags section:**
```markdown
### Partial → Full

| Flag | Status |
|------|--------|
| `--mcp-config` | ✓ Full: config parsing + server execution |
```

**Verification:**
- Review updated docs for accuracy
- Ensure both LIMITATIONS.md files are in sync

---

### Phase 6: Final Verification

**Goal:** Ensure all changes pass `make check` and code is production-ready.

**Run full check:**
```bash
make check
```

This runs:
1. `make lint` (shellcheck)
2. `cargo fmt --all -- --check`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `quench check --fix`
5. `cargo test --all`
6. `cargo build --all`
7. `cargo publish --dry-run`
8. `cargo audit`
9. `cargo deny check`

**Fix any issues that arise:**
- Clippy warnings
- Formatting issues
- Test failures
- Audit findings

**Final review checklist:**
- [ ] All MCP tests pass
- [ ] No clippy warnings
- [ ] No dead code warnings
- [ ] Documentation updated
- [ ] LIMITATIONS.md accurate
- [ ] `make check` passes completely

## Key Implementation Details

### Test Fixture Organization

The echo server (`tests/fixtures/echo_mcp_server.py`) provides:
- `initialize` → Server info with protocol version `2024-11-05`
- `tools/list` → Two tools: `echo` (returns input) and `fail` (always errors)
- `tools/call` → Executes tools with proper JSON-RPC responses

### Error Flow

```
TransportError → ClientError → (user-facing)
     ↓
Spawn/IO/Timeout → Transport wrapper → Display message
```

### State Transitions

```
McpServer:    Uninitialized → Running → Disconnected
                                ↓
                              Failed
```

### Thread Safety

- `Arc<Mutex<McpClient>>` for shared server access
- `AtomicBool` for shutdown flags
- `AtomicU64` for request ID generation

## Verification Plan

### After Each Phase

```bash
cargo test -p claudeless mcp
```

### Final Verification

```bash
# Full project check
make check

# Verify documentation builds
cargo doc --no-deps

# Verify echo server works standalone
python3 tests/fixtures/echo_mcp_server.py < /dev/null
```

### Regression Checklist

- [ ] All existing tests pass
- [ ] Build error fixed
- [ ] New tests added and passing
- [ ] No new clippy warnings
- [ ] LIMITATIONS.md updated in both locations
- [ ] `make check` passes completely
