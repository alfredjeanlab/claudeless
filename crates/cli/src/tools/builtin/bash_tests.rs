// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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
    // Output should include exit code for log extraction
    let text = result.text().unwrap();
    assert!(text.contains("hello"));
    assert!(text.contains("Exit code: 0"));
}

#[test]
#[cfg(unix)]
fn test_bash_exit_code_format() {
    let executor = BashExecutor::new();
    let call = ToolCallSpec {
        tool: "Bash".to_string(),
        input: json!({ "command": "exit 42" }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    let text = result.text().unwrap();
    assert!(text.contains("Exit code: 42"));
}

#[test]
#[cfg(unix)]
fn test_bash_failed_command_has_exit_code() {
    let executor = BashExecutor::new();
    let call = ToolCallSpec {
        tool: "Bash".to_string(),
        input: json!({ "command": "ls /nonexistent_path_abc123" }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    let text = result.text().unwrap();
    // Should contain some error message and exit code
    assert!(text.contains("Exit code:"));
    // Exit code should be non-zero
    assert!(!text.contains("Exit code: 0"));
}

#[test]
fn test_tool_name() {
    assert_eq!(
        BashExecutor::new().tool_name(),
        crate::tools::ToolName::Bash
    );
}
