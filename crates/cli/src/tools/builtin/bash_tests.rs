#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use serde_json::json;

#[test]
fn test_extract_command() {
    let input = json!({ "command": "ls -la" });
    assert_eq!(BashExecutor::extract_command(&input), Some("ls -la"));

    let empty = json!({});
    assert_eq!(BashExecutor::extract_command(&empty), None);
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
