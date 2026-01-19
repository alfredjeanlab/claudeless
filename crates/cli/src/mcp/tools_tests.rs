#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_filesystem_tools() {
    let tools = McpToolTemplates::filesystem_tools("fs");
    assert_eq!(tools.len(), 3);

    let names: Vec<_> = tools.iter().map(|t| &t.name).collect();
    assert!(names.contains(&&"read_file".to_string()));
    assert!(names.contains(&&"write_file".to_string()));
    assert!(names.contains(&&"list_directory".to_string()));

    for tool in &tools {
        assert_eq!(tool.server_name, "fs");
    }
}

#[test]
fn test_github_tools() {
    let tools = McpToolTemplates::github_tools("github");
    assert_eq!(tools.len(), 2);

    let names: Vec<_> = tools.iter().map(|t| &t.name).collect();
    assert!(names.contains(&&"create_issue".to_string()));
    assert!(names.contains(&&"create_pull_request".to_string()));
}

#[test]
fn test_database_tools() {
    let tools = McpToolTemplates::database_tools("db");
    assert_eq!(tools.len(), 2);

    let names: Vec<_> = tools.iter().map(|t| &t.name).collect();
    assert!(names.contains(&&"query".to_string()));
    assert!(names.contains(&&"list_tables".to_string()));
}

#[test]
fn test_tool_call_serialization() {
    let call = McpToolCall {
        name: "read_file".into(),
        arguments: serde_json::json!({"path": "/tmp/test.txt"}),
        server: Some("fs".into()),
    };

    let json = serde_json::to_string(&call).unwrap();
    assert!(json.contains("read_file"));
    assert!(json.contains("/tmp/test.txt"));
}

#[test]
fn test_tool_result_success() {
    let result = McpToolResult::success(serde_json::json!("file contents"));
    assert!(result.success);
    assert!(result.error.is_none());
}

#[test]
fn test_tool_result_failure() {
    let result = McpToolResult::failure("file not found");
    assert!(!result.success);
    assert_eq!(result.error, Some("file not found".into()));
}
