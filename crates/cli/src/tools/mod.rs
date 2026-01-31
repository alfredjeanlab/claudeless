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
//! ```ignore
//! use claudeless::tools::{create_executor_with_mcp, ExecutionContext};
//!
//! let executor = create_executor_with_mcp(mode, mcp_manager);
//! let ctx = ExecutionContext::default();
//! let result = executor.execute(&call, "toolu_123", &ctx);
//! ```

pub(crate) mod builtin;
pub(crate) mod executor;
pub(crate) mod mcp_executor;
pub(crate) mod result;
pub(crate) mod tool_name;

pub use executor::{create_executor_with_mcp, ExecutionContext, ToolExecutor};
pub use result::{ToolExecutionResult, ToolResultContent};
