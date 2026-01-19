#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use serde_json::json;

#[test]
fn test_mcp_executor_with_mock_result() {
    let executor = McpExecutor::new();
    let call = ToolCallSpec {
        tool: "read_file".to_string(),
        input: json!({ "path": "/tmp/test.txt" }),
        result: Some("file contents".to_string()),
    };
    let ctx = ExecutionContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(!result.is_error);
    assert_eq!(result.text(), Some("file contents"));
}

#[test]
fn test_mcp_executor_no_manager() {
    let executor = McpExecutor::new();
    let call = ToolCallSpec {
        tool: "read_file".to_string(),
        input: json!({ "path": "/tmp/test.txt" }),
        result: None,
    };
    let ctx = ExecutionContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("No MCP server found"));
}

#[test]
fn test_executor_name() {
    assert_eq!(McpExecutor::new().name(), "mcp");
}
