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

/// Assert that a tool result is an error containing the expected text.
pub fn assert_tool_error_contains(result: &ToolExecutionResult, expected: &str) {
    assert!(
        result.is_error,
        "Expected error but got success: {:?}",
        result
    );
    let text = result.text().expect("Error result should have text");
    assert!(
        text.contains(expected),
        "Expected '{}' in: {}",
        expected,
        text
    );
}

/// Assert that a tool result is a success containing the expected text.
pub fn assert_tool_success_contains(result: &ToolExecutionResult, expected: &str) {
    assert!(
        !result.is_error,
        "Expected success but got error: {:?}",
        result
    );
    let text = result.text().expect("Success result should have text");
    assert!(
        text.contains(expected),
        "Expected '{}' in: {}",
        expected,
        text
    );
}

/// Execute a builtin tool by name with the given input.
pub fn execute_tool(tool: &str, input: serde_json::Value) -> ToolExecutionResult {
    let executor = BuiltinExecutor::new();
    let call = tool_call(tool, input);
    executor.execute(
        &call,
        "test_id",
        &crate::tools::executor::ExecutionContext::default(),
    )
}

#[cfg(test)]
#[path = "test_helpers_tests.rs"]
mod tests;
