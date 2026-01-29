# Implementation Plan: MCP Main Integration

## Overview

Wire MCP server management into `main.rs` to enable real MCP server connections at startup. This involves initializing servers on startup, handling `--strict-mcp-config` errors, outputting debug information with `--mcp-debug`, creating an MCP-aware tool executor, and ensuring graceful shutdown on exit.

## Project Structure

```
crates/cli/src/
├── main.rs              # UPDATE - wire MCP initialization and shutdown
├── tools/
│   ├── mod.rs           # UPDATE - export MCP executor
│   ├── executor.rs      # UPDATE - add create_executor_with_mcp
│   └── mcp_executor.rs  # NEW - MCP tool executor
└── mcp/
    └── (existing modules - no changes needed)
```

## Dependencies

No new dependencies. Uses existing:
- `tokio` - async runtime
- `parking_lot` - `RwLock` for shared state

## Implementation Phases

### Phase 1: MCP Tool Executor

**Goal:** Create an executor that routes tool calls to MCP servers.

**Files:**
- `crates/cli/src/tools/mcp_executor.rs` (new)
- `crates/cli/src/tools/mcp_executor_tests.rs` (new)
- `crates/cli/src/tools/mod.rs` (update exports)

**Implementation:**

```rust
// crates/cli/src/tools/mcp_executor.rs

use std::sync::Arc;
use parking_lot::RwLock;

use crate::config::ToolCallSpec;
use crate::mcp::McpManager;

use super::executor::{ExecutionContext, ToolExecutor};
use super::result::ToolExecutionResult;

/// Executor that routes tool calls to MCP servers.
pub struct McpExecutor {
    manager: Arc<RwLock<McpManager>>,
}

impl McpExecutor {
    pub fn new(manager: Arc<RwLock<McpManager>>) -> Self {
        Self { manager }
    }

    /// Check if this executor can handle a tool.
    pub fn has_tool(&self, name: &str) -> bool {
        self.manager.read().has_tool(name)
    }
}

impl ToolExecutor for McpExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        _ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        // Use tokio runtime to execute async call
        let manager = self.manager.read();
        let result = tokio::runtime::Handle::current().block_on(
            manager.call_tool(&call.tool, call.input.clone())
        );

        match result {
            Ok(mcp_result) => {
                if mcp_result.success {
                    ToolExecutionResult::success(tool_use_id, &mcp_result.output)
                } else {
                    ToolExecutionResult::error(
                        tool_use_id,
                        mcp_result.error.unwrap_or_else(|| "MCP tool error".to_string()),
                    )
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

**Verification:**
- Unit tests for `McpExecutor::has_tool`
- Unit tests for tool execution routing

---

### Phase 2: Composite Executor

**Goal:** Create a composite executor that tries MCP first, then falls back to builtin.

**Files:**
- `crates/cli/src/tools/executor.rs` (update)

**Implementation:**

```rust
// Add to executor.rs

/// Executor that tries MCP first, then falls back to builtin.
pub struct CompositeExecutor {
    mcp: Option<McpExecutor>,
    builtin: Box<dyn ToolExecutor>,
}

impl CompositeExecutor {
    pub fn new(mcp: Option<McpExecutor>, builtin: Box<dyn ToolExecutor>) -> Self {
        Self { mcp, builtin }
    }
}

impl ToolExecutor for CompositeExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        id: &str,
        ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        // Check MCP first (user-configured tools take precedence)
        if let Some(ref mcp) = self.mcp {
            if mcp.has_tool(&call.tool) {
                return mcp.execute(call, id, ctx);
            }
        }
        // Fall back to builtin
        self.builtin.execute(call, id, ctx)
    }

    fn name(&self) -> &'static str {
        "composite"
    }
}

/// Create an executor with optional MCP support.
pub fn create_executor_with_mcp(
    mode: ToolExecutionMode,
    mcp_manager: Option<Arc<RwLock<McpManager>>>,
) -> Box<dyn ToolExecutor> {
    match mode {
        ToolExecutionMode::Disabled => Box::new(DisabledExecutor::new()),
        ToolExecutionMode::Mock => Box::new(MockExecutor::new()),
        ToolExecutionMode::Live => {
            let builtin = Box::new(super::builtin::BuiltinExecutor::new());
            let mcp = mcp_manager.map(McpExecutor::new);
            Box::new(CompositeExecutor::new(mcp, builtin))
        }
    }
}
```

**Verification:**
- Test MCP tool routing
- Test fallback to builtin tools
- Test with no MCP manager (None)

---

### Phase 3: Main Integration - Initialization

**Goal:** Initialize MCP servers on startup with proper error handling.

**Files:**
- `crates/cli/src/main.rs` (update)

**Changes to `load_mcp_configs`:**

```rust
/// Load and initialize MCP servers from CLI flags.
async fn load_mcp_configs(cli: &Cli) -> Result<Option<Arc<RwLock<McpManager>>>, Box<dyn std::error::Error>> {
    if cli.mcp_config.is_empty() {
        return Ok(None);
    }

    // Load config files
    let mut configs = Vec::new();
    for config_input in &cli.mcp_config {
        match load_mcp_config(config_input) {
            Ok(config) => configs.push(config),
            Err(e) => {
                eprintln!("Error loading MCP config: {}", e);
                std::process::exit(1);
            }
        }
    }

    let merged = McpConfig::merge(configs);
    let mut manager = McpManager::from_config(&merged);

    if cli.mcp_debug {
        eprintln!(
            "MCP: Loading {} server(s): {:?}",
            manager.server_count(),
            manager.server_names()
        );
    }

    // Initialize servers (spawn processes, discover tools)
    let results = manager.initialize().await;

    // Handle initialization results
    for (name, result) in &results {
        match result {
            Ok(()) => {
                if cli.mcp_debug {
                    let server = manager.get_server(name).unwrap();
                    eprintln!(
                        "MCP: Server '{}' started with {} tool(s): {:?}",
                        name,
                        server.tools.len(),
                        server.tool_names()
                    );
                }
            }
            Err(e) => {
                if cli.strict_mcp_config {
                    eprintln!("MCP error: Server '{}' failed to start: {}", name, e);
                    std::process::exit(1);
                } else if cli.mcp_debug {
                    eprintln!("MCP warning: Server '{}' failed to start: {}", name, e);
                }
            }
        }
    }

    // Check if any servers are running
    if manager.running_server_count() == 0 && cli.mcp_debug {
        eprintln!("MCP: No servers running");
    }

    Ok(Some(Arc::new(RwLock::new(manager))))
}
```

**Verification:**
- Test successful initialization output
- Test `--strict-mcp-config` exits on failure
- Test `--mcp-debug` shows warnings on failure

---

### Phase 4: Main Integration - Tool Execution

**Goal:** Wire MCP manager into the tool execution pipeline.

**Files:**
- `crates/cli/src/main.rs` (update)

**Changes to main():**

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // ... existing permission bypass validation ...

    // Load and initialize MCP servers
    let mcp_manager = load_mcp_configs(&cli).await?;

    // ... rest of main ...

    // When creating executor (around line 239-244):
    let executor: Box<dyn ToolExecutor> = match execution_mode {
        ToolExecutionMode::Live => {
            // Create composite executor with MCP support
            let builtin = BuiltinExecutor::new()
                .with_state_writer(Arc::clone(&state_writer));
            let mcp = mcp_manager.as_ref().map(|m| McpExecutor::new(Arc::clone(m)));
            Box::new(CompositeExecutor::new(mcp, Box::new(builtin)))
        }
        _ => create_executor(execution_mode),
    };

    // ... tool execution loop ...

    // Cleanup on exit
    if let Some(mgr) = mcp_manager {
        mgr.write().shutdown().await;
    }

    Ok(())
}
```

**Verification:**
- Test tool calls route to MCP servers
- Test builtin tools still work
- Test MCP tools override builtin if same name

---

### Phase 5: Graceful Shutdown

**Goal:** Ensure MCP servers are shut down gracefully on exit.

**Files:**
- `crates/cli/src/main.rs` (update)

**Implementation:**

Add shutdown handling at all exit points:

1. Normal completion - already handled in Phase 4
2. Error paths - add cleanup before `std::process::exit`
3. TUI mode - add shutdown to `run_tui_mode`

```rust
/// Run in TUI mode with MCP support
fn run_tui_mode(
    cli: &Cli,
    allow_bypass_permissions: bool,
    mcp_manager: Option<Arc<RwLock<McpManager>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // ... existing TUI setup ...

    let exit_reason = app.run()?;

    // Shutdown MCP servers before exiting
    if let Some(mgr) = mcp_manager {
        // Use runtime to run async shutdown
        tokio::runtime::Handle::current().block_on(async {
            mgr.write().shutdown().await;
        });
    }

    // ... existing exit handling ...
}
```

For early exits (errors), use a helper:

```rust
fn exit_with_mcp_cleanup(
    mcp_manager: Option<Arc<RwLock<McpManager>>>,
    code: i32,
) -> ! {
    if let Some(mgr) = mcp_manager {
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            handle.block_on(async {
                mgr.write().shutdown().await;
            });
        }
    }
    std::process::exit(code);
}
```

**Verification:**
- Test normal exit shuts down servers
- Test error exit shuts down servers
- Test TUI exit shuts down servers

---

### Phase 6: Integration Tests

**Goal:** End-to-end testing of MCP integration.

**Files:**
- `tests/mcp_integration.rs` (new)

**Test scenarios:**

1. **Basic MCP tool execution**
   - Start claudeless with echo MCP server
   - Execute MCP tool call
   - Verify output

2. **Strict mode failure**
   - Use `--strict-mcp-config` with invalid command
   - Verify exit code 1

3. **Debug mode output**
   - Use `--mcp-debug` with valid server
   - Verify startup messages on stderr

4. **Graceful shutdown**
   - Start server, execute tool, exit
   - Verify no orphan processes

```rust
#[tokio::test]
async fn test_mcp_tool_execution() {
    let output = Command::new(env!("CARGO_BIN_EXE_claudeless"))
        .args([
            "--mcp-config", r#"{"mcpServers":{"echo":{"command":"python3","args":["tests/fixtures/echo_mcp_server.py"]}}}"#,
            "--tool-mode", "live",
            "--print",
            "--scenario", "tests/fixtures/mcp_tool_call.toml",
            "test prompt",
        ])
        .output()
        .await
        .unwrap();

    assert!(output.status.success());
}

#[tokio::test]
async fn test_strict_mcp_config_failure() {
    let output = Command::new(env!("CARGO_BIN_EXE_claudeless"))
        .args([
            "--mcp-config", r#"{"mcpServers":{"bad":{"command":"nonexistent"}}}"#,
            "--strict-mcp-config",
            "--print",
            "test",
        ])
        .output()
        .await
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to start"));
}
```

**Verification:**
- All integration tests pass
- `make check` passes

## Key Implementation Details

### Async/Sync Bridge

The `ToolExecutor` trait is synchronous, but MCP communication is async. Use `tokio::runtime::Handle::current().block_on()`:

```rust
fn execute(&self, call: &ToolCallSpec, ...) -> ToolExecutionResult {
    tokio::runtime::Handle::current().block_on(async {
        self.manager.read().call_tool(&call.tool, call.input.clone()).await
    })
}
```

### Tool Name Collision

MCP tools may share names with built-in tools. Resolution order:
1. MCP servers (user's configured tools take precedence)
2. Built-in tools (fallback)

This allows users to override built-in behavior via MCP if desired.

### Error Handling Strategy

- **Config load errors**: Exit immediately (invalid JSON/file)
- **Server spawn errors with `--strict-mcp-config`**: Exit with error
- **Server spawn errors without strict**: Log warning if `--mcp-debug`, continue
- **Tool execution errors**: Return error result to caller

### Shutdown Sequence

1. Send shutdown notification to each MCP client
2. Wait for graceful termination (timeout)
3. Force kill if process doesn't exit
4. This happens at all exit points (normal, error, TUI)

## Verification Plan

### Unit Tests

1. **McpExecutor** (`mcp_executor_tests.rs`)
   - `has_tool` returns correct result
   - Tool execution routes correctly
   - Error handling for failed calls

2. **CompositeExecutor** (`executor_tests.rs`)
   - MCP tools found and executed
   - Fallback to builtin when MCP doesn't have tool
   - Works with None MCP manager

### Integration Tests

1. **End-to-end flow** (`tests/mcp_integration.rs`)
   - Start with MCP server, execute tool
   - Verify output format

2. **Error handling**
   - `--strict-mcp-config` exits on failure
   - `--mcp-debug` shows appropriate output

3. **Graceful shutdown**
   - No orphan processes after exit

### Manual Testing

```bash
# Test with echo MCP server
claudeless --mcp-config '{"mcpServers":{"echo":{"command":"python3","args":["echo_server.py"]}}}' \
    --mcp-debug --tool-mode live --print -p "test"

# Test strict mode failure
claudeless --mcp-config '{"mcpServers":{"bad":{"command":"nonexistent"}}}' \
    --strict-mcp-config --print -p "test"
# Should exit with error

# Test debug output
claudeless --mcp-config '...' --mcp-debug --print -p "test"
# Should show server startup on stderr
```

### Final Verification

```bash
make check
```
