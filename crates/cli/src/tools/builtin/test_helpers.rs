// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared test helpers for builtin tool executor tests.

use super::*;
use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

/// Create a tool call spec with the given tool name and input.
pub fn tool_call(tool: &str, input: serde_json::Value) -> ToolCallSpec {
    ToolCallSpec {
        tool: tool.to_string(),
        input,
        result: None,
    }
}

/// Execute a builtin tool executor with default context.
pub fn execute<E: BuiltinToolExecutor + Default>(input: serde_json::Value) -> ToolExecutionResult {
    let executor = E::default();
    let call = tool_call(executor.tool_name().as_str(), input);
    executor.execute(&call, "test_id", &BuiltinContext::default())
}
