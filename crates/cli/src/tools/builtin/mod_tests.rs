// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use serde_json::json;

#[test]
fn test_builtin_executor_with_mock_result() {
    let executor = BuiltinExecutor::new();
    let call = ToolCallSpec {
        tool: "Bash".to_string(),
        input: json!({ "command": "ls" }),
        result: Some("mock output".to_string()),
    };
    let ctx = ExecutionContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(!result.is_error);
    assert_eq!(result.text(), Some("mock output"));
}

#[test]
fn test_builtin_executor_unknown_tool() {
    let executor = BuiltinExecutor::new();
    let call = ToolCallSpec {
        tool: "UnknownTool".to_string(),
        input: json!({}),
        result: None,
    };
    let ctx = ExecutionContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Unknown built-in tool"));
}
