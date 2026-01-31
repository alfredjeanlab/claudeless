// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool execution module for tool calls.
//!
//! This module provides tool execution capabilities with three modes:
//! - **Disabled** - No tool execution
//! - **Mock** - Return pre-configured results from scenario config
//! - **Live** - Execute built-in tools directly (Bash, Read, Write, etc.)
//!
//! # Example
//!
//! ```
//! use claudeless::tools::{ToolExecutor, MockExecutor, ExecutionContext};
//! use claudeless::config::ToolCallSpec;
//!
//! let executor = MockExecutor::new();
//! let call = ToolCallSpec {
//!     tool: "Bash".to_string(),
//!     input: serde_json::json!({ "command": "ls" }),
//!     result: Some("file1.txt\nfile2.txt".to_string()),
//! };
//! let ctx = ExecutionContext::default();
//! let result = executor.execute(&call, "toolu_123", &ctx);
//! assert!(!result.is_error);
//! ```

pub mod builtin;
pub mod executor;
pub mod mcp_executor;
pub mod result;
pub mod tool_name;

pub use executor::{
    create_executor, create_executor_with_mcp, create_executor_with_mcp_and_permissions,
    create_executor_with_permissions, ExecutionContext, MockExecutor, PermissionCheckingExecutor,
    ToolExecutor,
};
pub use mcp_executor::{CompositeExecutor, McpToolExecutor};
pub use result::{ToolExecutionResult, ToolResultContent};
pub use tool_name::ToolName;
