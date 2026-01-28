// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn test_success_result() {
    let result = ToolExecutionResult::success("toolu_123", "file contents");
    assert!(!result.is_error);
    assert_eq!(result.tool_use_id, "toolu_123");
    assert_eq!(result.text(), Some("file contents"));
}

#[test]
fn test_error_result() {
    let result = ToolExecutionResult::error("toolu_456", "file not found");
    assert!(result.is_error);
    assert_eq!(result.text(), Some("file not found"));
}

#[test]
fn test_no_mock_result() {
    let result = ToolExecutionResult::no_mock_result("toolu_789", "Bash");
    assert!(result.is_error);
    assert!(result.text().unwrap().contains("No mock result"));
    assert!(result.text().unwrap().contains("Bash"));
}

#[test]
fn test_disabled_result() {
    let result = ToolExecutionResult::disabled("toolu_abc");
    assert!(result.is_error);
    assert!(result.text().unwrap().contains("disabled"));
}

#[test]
fn test_permission_denied() {
    let result = ToolExecutionResult::permission_denied("toolu_def", "DontAsk mode");
    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Permission denied"));
    assert!(result.text().unwrap().contains("DontAsk mode"));
}

#[test]
fn test_serialization() {
    let result = ToolExecutionResult::success("toolu_123", "output");
    let json = serde_json::to_string(&result).unwrap();
    let parsed: ToolExecutionResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.tool_use_id, "toolu_123");
    assert!(!parsed.is_error);
}
