// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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

// =============================================================================
// Real Claude CLI Compatibility Tests
// =============================================================================

/// Real Claude CLI provides qualified tool names in the init event.
///
/// Observed from `claude --mcp-config ... --output-format stream-json --verbose`:
/// ```json
/// {"tools": ["Read", "Write", ..., "mcp__filesystem__read_file", ...]}
/// ```
#[test]
fn test_filesystem_tools_qualified_names() {
    let tools = McpToolTemplates::filesystem_tools("filesystem");
    let qualified_names: Vec<String> = tools.iter().map(|t| t.qualified_name()).collect();

    assert!(qualified_names.contains(&"mcp__filesystem__read_file".to_string()));
    assert!(qualified_names.contains(&"mcp__filesystem__write_file".to_string()));
    assert!(qualified_names.contains(&"mcp__filesystem__list_directory".to_string()));
}

/// Real Claude CLI MCP tool result format.
///
/// Observed tool result content from MCP tools:
/// ```json
/// {"content":"{\"content\":\"hello world\\n\"}","structuredContent":{"content":"hello world\n"}}
/// ```
///
/// The MCP server returns JSON content, and Claude CLI wraps it with both
/// `content` (as string) and `structuredContent` (as object).
#[test]
fn test_tool_result_json_format() {
    let result = McpToolResult::success(serde_json::json!({"content": "hello world\n"}));

    let json = serde_json::to_string(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Result should have content field
    assert!(parsed["content"].is_object());
    assert_eq!(parsed["content"]["content"], "hello world\n");
    assert!(parsed["success"].as_bool().unwrap());
}

/// Real Claude CLI tool error format shows access denied clearly.
///
/// Observed:
/// ```json
/// {"type":"tool_result","content":"Access denied - path outside allowed directories: /tmp/...","is_error":true}
/// ```
#[test]
fn test_tool_result_error_format() {
    let result = McpToolResult::failure(
        "Access denied - path outside allowed directories: /etc/passwd not in /tmp/mcp-test",
    );

    let json = serde_json::to_string(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(!parsed["success"].as_bool().unwrap());
    assert!(parsed["error"].as_str().unwrap().contains("Access denied"));
}
