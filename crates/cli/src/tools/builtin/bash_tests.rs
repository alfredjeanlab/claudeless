// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]
use super::*;
use crate::tools::builtin::extract_str;
use serde_json::json;

#[test]
fn test_extract_command() {
    let input = json!({ "command": "ls -la" });
    assert_eq!(extract_str(&input, "command"), Some("ls -la"));

    let empty = json!({});
    assert_eq!(extract_str(&empty, "command"), None);
}

#[test]
fn test_bash_missing_command() {
    let executor = BashExecutor::new();
    let call = ToolCallSpec {
        tool: "Bash".to_string(),
        input: json!({}),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Missing 'command'"));
}

#[test]
#[cfg(unix)]
fn test_bash_real_execution() {
    let executor = BashExecutor::new();
    let call = ToolCallSpec {
        tool: "Bash".to_string(),
        input: json!({ "command": "echo hello" }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(!result.is_error);
    assert_eq!(result.text(), Some("hello"));
}

#[test]
fn test_tool_name() {
    assert_eq!(BashExecutor::new().tool_name(), "Bash");
}
