// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool execution module for simulated tool calls.
//!
//! This module provides tool execution capabilities with three levels:
//! - **Mock Results** - Return pre-configured results from scenario config
//! - **Simulated Built-in Tools** - Sandboxed execution of Bash, Read, Write, etc.
//! - **Real MCP Server Execution** - Spawn actual MCP servers
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
pub mod mcp;
pub mod result;

pub use executor::{
    create_executor, create_executor_with_permissions, ExecutionContext, MockExecutor,
    PermissionCheckingExecutor, ToolExecutor,
};
pub use result::{ToolExecutionResult, ToolResultContent};
