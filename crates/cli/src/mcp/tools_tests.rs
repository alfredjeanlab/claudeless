// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

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
