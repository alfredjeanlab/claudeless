# Implementation Plan: MCP Tool Executor

## Overview

Integrate MCP tool execution into the existing tool execution pipeline. This involves creating `McpToolExecutor` implementing the `ToolExecutor` trait, a `CompositeExecutor` for MCP-first-then-builtin routing, and bridging async MCP calls to the synchronous executor trait using `block_on`.

## Project Structure

```
crates/cli/src/tools/
├── mod.rs               # Update exports
├── executor.rs          # Add create_executor_with_mcp()
├── mcp_executor.rs      # NEW - McpToolExecutor and CompositeExecutor
└── mcp_executor_tests.rs # NEW - Unit tests
```

## Dependencies

No new dependencies needed. Uses existing:
- `tokio` - Runtime for `block_on`
- `parking_lot` - `RwLock` for shared McpManager
- `serde_json` - Tool input handling

## Implementation Phases

### Phase 1: McpToolExecutor Structure

**Goal:** Create the `McpToolExecutor` that wraps `McpManager` and implements `ToolExecutor`.

**File:** `crates/cli/src/tools/mcp_executor.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! MCP tool executor for routing tool calls to MCP servers.

use std::sync::Arc;
use parking_lot::RwLock;

use crate::config::ToolCallSpec;
use crate::mcp::server::McpManager;
use super::executor::{ExecutionContext, ToolExecutor};
use super::result::ToolExecutionResult;

/// Executor that handles MCP tool calls.
pub struct McpToolExecutor {
    /// Shared MCP manager with server connections.
    manager: Arc<RwLock<McpManager>>,
}

impl McpToolExecutor {
    /// Create a new MCP tool executor.
    pub fn new(manager: Arc<RwLock<McpManager>>) -> Self {
        Self { manager }
    }

    /// Check if a tool is handled by MCP.
    pub fn has_tool(&self, name: &str) -> bool {
        self.manager.read().has_tool(name)
    }
}
```

**Key decisions:**
- Use `parking_lot::RwLock` (not `tokio::sync::RwLock`) for synchronous access in `has_tool()`
- Manager is shared via `Arc<RwLock<McpManager>>` for thread safety
- Simple delegation pattern - McpManager already has routing logic

---

### Phase 2: Async/Sync Bridge

**Goal:** Implement `ToolExecutor::execute()` bridging the async `McpManager::call_tool()` to sync.

**File:** `crates/cli/src/tools/mcp_executor.rs` (continued)

```rust
impl ToolExecutor for McpToolExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        _ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        // Check if we handle this tool
        let manager = self.manager.read();
        if !manager.has_tool(&call.tool) {
            return ToolExecutionResult::error(
                tool_use_id,
                format!("MCP tool not found: {}", call.tool),
            );
        }

        // Bridge async to sync using tokio's block_on
        // SAFETY: This assumes we're running inside a tokio runtime
        let handle = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "No tokio runtime available for MCP execution",
                );
            }
        };

        // Execute the async call
        let result = handle.block_on(async {
            manager.call_tool(&call.tool, call.input.clone()).await
        });

        // Convert McpToolResult to ToolExecutionResult
        match result {
            Ok(mcp_result) => {
                if mcp_result.success {
                    // Format content as string for tool result
                    let text = format_mcp_content(&mcp_result.content);
                    ToolExecutionResult::success(tool_use_id, text)
                } else {
                    ToolExecutionResult::error(
                        tool_use_id,
                        mcp_result.error.unwrap_or_else(|| "MCP tool execution failed".into()),
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

/// Format MCP content Value as string for tool result.
fn format_mcp_content(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
    }
}
```

**Key decisions:**
- Use `try_current()` to safely handle missing runtime case
- Hold `RwLock` read guard across the async block (parking_lot allows this)
- Format MCP JSON content to string for compatibility with existing result types

---

### Phase 3: CompositeExecutor

**Goal:** Create executor that tries MCP first, then falls back to builtin.

**File:** `crates/cli/src/tools/mcp_executor.rs` (continued)

```rust
use super::builtin::BuiltinExecutor;

/// Executor that routes to MCP first, then falls back to builtin.
pub struct CompositeExecutor {
    /// Optional MCP executor (None if no MCP servers configured).
    mcp: Option<McpToolExecutor>,
    /// Builtin tool executor as fallback.
    builtin: BuiltinExecutor,
}

impl CompositeExecutor {
    /// Create a new composite executor.
    pub fn new(mcp: Option<McpToolExecutor>, builtin: BuiltinExecutor) -> Self {
        Self { mcp, builtin }
    }

    /// Create with just builtin (no MCP).
    pub fn builtin_only(builtin: BuiltinExecutor) -> Self {
        Self { mcp: None, builtin }
    }
}

impl ToolExecutor for CompositeExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        // Check MCP first - user-configured tools take precedence
        if let Some(ref mcp) = self.mcp {
            if mcp.has_tool(&call.tool) {
                return mcp.execute(call, tool_use_id, ctx);
            }
        }

        // Fall back to builtin
        self.builtin.execute(call, tool_use_id, ctx)
    }

    fn name(&self) -> &'static str {
        "composite"
    }
}
```

**Key decisions:**
- MCP tools take precedence over builtins (allows user override)
- Optional MCP support - works with or without MCP servers
- Simple two-tier routing; no complex chain needed

---

### Phase 4: Factory Functions

**Goal:** Add `create_executor_with_mcp()` factory and update module exports.

**File:** `crates/cli/src/tools/executor.rs` (add)

```rust
use std::sync::Arc;
use parking_lot::RwLock;
use crate::mcp::server::McpManager;
use super::mcp_executor::{CompositeExecutor, McpToolExecutor};

/// Create an executor with MCP support.
///
/// If `mcp_manager` is provided, MCP tools are checked first before falling
/// back to builtin tools. This allows MCP servers to override builtin behavior.
pub fn create_executor_with_mcp(
    mode: ToolExecutionMode,
    mcp_manager: Option<Arc<RwLock<McpManager>>>,
) -> Box<dyn ToolExecutor> {
    match mode {
        ToolExecutionMode::Disabled => Box::new(DisabledExecutor::new()),
        ToolExecutionMode::Mock => Box::new(MockExecutor::new()),
        ToolExecutionMode::Live => {
            let builtin = super::builtin::BuiltinExecutor::new();
            let mcp = mcp_manager.map(McpToolExecutor::new);
            Box::new(CompositeExecutor::new(mcp, builtin))
        }
    }
}

/// Create an executor with MCP and permission checking.
pub fn create_executor_with_mcp_and_permissions(
    mode: ToolExecutionMode,
    mcp_manager: Option<Arc<RwLock<McpManager>>>,
    checker: PermissionChecker,
) -> Box<dyn ToolExecutor> {
    let inner = create_executor_with_mcp(mode, mcp_manager);
    Box::new(PermissionCheckingExecutor::new(inner, checker))
}
```

**File:** `crates/cli/src/tools/mod.rs` (update exports)

```rust
pub mod mcp_executor;

pub use mcp_executor::{CompositeExecutor, McpToolExecutor};
pub use executor::{
    create_executor, create_executor_with_mcp, create_executor_with_mcp_and_permissions,
    // ... existing exports
};
```

---

### Phase 5: Unit Tests

**Goal:** Test executor behavior with mocked MCP manager.

**File:** `crates/cli/src/tools/mcp_executor_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::config::ToolCallSpec;
use crate::mcp::config::McpToolDef;
use crate::mcp::server::{McpManager, McpServer, McpServerDef, McpServerStatus};
use std::sync::Arc;
use parking_lot::RwLock;

fn mock_manager_with_tool(tool_name: &str) -> Arc<RwLock<McpManager>> {
    let mut manager = McpManager::new();

    let mut server = McpServer::from_def("test-server", McpServerDef::default());
    server.status = McpServerStatus::Running;
    server.register_tool(McpToolDef {
        name: tool_name.to_string(),
        description: "Test tool".to_string(),
        input_schema: serde_json::json!({"type": "object"}),
        server_name: "test-server".to_string(),
    });

    manager.add_server(server);
    manager.register_tool("test-server", McpToolDef {
        name: tool_name.to_string(),
        description: "Test tool".to_string(),
        input_schema: serde_json::json!({"type": "object"}),
        server_name: "test-server".to_string(),
    });

    Arc::new(RwLock::new(manager))
}

#[test]
fn has_tool_returns_true_for_registered_tool() {
    let manager = mock_manager_with_tool("my_tool");
    let executor = McpToolExecutor::new(manager);

    assert!(executor.has_tool("my_tool"));
    assert!(!executor.has_tool("other_tool"));
}

#[test]
fn composite_routes_to_builtin_for_unknown_mcp_tool() {
    let manager = mock_manager_with_tool("mcp_tool");
    let mcp = McpToolExecutor::new(manager);
    let builtin = BuiltinExecutor::new();
    let composite = CompositeExecutor::new(Some(mcp), builtin);

    let call = ToolCallSpec {
        tool: "Read".to_string(),
        input: serde_json::json!({"file_path": "/tmp/test"}),
        result: Some("test content".to_string()),
    };

    let result = composite.execute(&call, "test-id", &ExecutionContext::default());

    // Should fall through to builtin (which uses mock result)
    assert!(!result.is_error);
    assert_eq!(result.text(), Some("test content"));
}

#[test]
fn composite_routes_mcp_tool_to_mcp() {
    let manager = mock_manager_with_tool("custom_tool");
    let mcp = McpToolExecutor::new(manager);
    let builtin = BuiltinExecutor::new();
    let composite = CompositeExecutor::new(Some(mcp), builtin);

    // MCP tool should be recognized
    assert!(composite.mcp.as_ref().unwrap().has_tool("custom_tool"));
}

#[test]
fn composite_without_mcp_works() {
    let builtin = BuiltinExecutor::new();
    let composite = CompositeExecutor::builtin_only(builtin);

    let call = ToolCallSpec {
        tool: "Read".to_string(),
        input: serde_json::json!({"file_path": "/tmp/test"}),
        result: Some("test content".to_string()),
    };

    let result = composite.execute(&call, "test-id", &ExecutionContext::default());
    assert!(!result.is_error);
}
```

---

### Phase 6: Integration Verification

**Goal:** Verify the complete execution flow works end-to-end.

This phase is manual verification using the existing echo MCP server fixture (if available from prior phases):

```bash
# Build and test
cargo build --all
cargo test --all

# Full verification
make check
```

## Key Implementation Details

### Async/Sync Bridge Pattern

The `ToolExecutor` trait is synchronous, but MCP communication is async. We bridge using tokio's runtime handle:

```rust
let handle = tokio::runtime::Handle::try_current()?;
let result = handle.block_on(async { ... });
```

**Important considerations:**
- `try_current()` is safer than `current()` - returns Result instead of panicking
- This works because the main CLI runs in a tokio runtime
- The `parking_lot::RwLock` can be held across await points (unlike std RwLock)

### Tool Precedence

MCP tools take precedence over builtin tools. This allows users to:
1. Override default behavior via MCP servers
2. Provide custom implementations of standard tools
3. Test with mock MCP servers

The routing is: **MCP → Builtin → Error**

### State Writer Integration

The `BuiltinExecutor` has an optional `StateWriter` for TodoWrite/ExitPlanMode. The `CompositeExecutor` passes through to builtin, preserving this functionality. If MCP and builtin both need state writers, extend as needed.

### Error Mapping

MCP errors are converted to `ToolExecutionResult::error()`:
- `ClientError::*` → error message
- `McpToolResult` with `success: false` → use `error` field
- Missing runtime → explicit error message

## Verification Plan

### Unit Tests (`mcp_executor_tests.rs`)

1. **McpToolExecutor**
   - `has_tool()` returns correct values
   - Executor name is "mcp"

2. **CompositeExecutor**
   - Routes MCP tools to MCP executor
   - Falls back to builtin for non-MCP tools
   - Works without MCP (None case)
   - Works with MCP (Some case)

3. **Factory functions**
   - `create_executor_with_mcp()` returns CompositeExecutor for Live mode
   - Returns DisabledExecutor/MockExecutor for other modes
   - Works with and without MCP manager

### Integration Tests

Test with echo MCP server from `tests/fixtures/echo_mcp_server.py`:

```rust
#[tokio::test]
async fn mcp_tool_executor_executes_real_tool() {
    let def = McpServerDef {
        command: "python3".to_string(),
        args: vec!["tests/fixtures/echo_mcp_server.py".to_string()],
        ..Default::default()
    };

    let mut manager = McpManager::new();
    let mut server = McpServer::from_def("echo", def);
    server.spawn().await.unwrap();
    manager.add_server(server);

    // Register discovered tools
    for tool in manager.get_server("echo").unwrap().tools.iter() {
        manager.register_tool("echo", tool.clone());
    }

    let manager = Arc::new(RwLock::new(manager));
    let executor = McpToolExecutor::new(manager);

    let call = ToolCallSpec {
        tool: "echo".to_string(),
        input: serde_json::json!({"msg": "hello"}),
        result: None,
    };

    let result = executor.execute(&call, "test-id", &ExecutionContext::default());
    assert!(!result.is_error);
}
```

### Final Verification

```bash
# Run all checks
make check

# Specific tests
cargo test --package claudeless mcp_executor
cargo test --package claudeless --test mcp_integration
```
