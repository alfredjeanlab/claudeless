// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::tools::builtin::{extract_bool, extract_file_path, extract_str};
use serde_json::json;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_extract_fields() {
    let input = json!({
        "file_path": "/tmp/test.txt",
        "old_string": "old",
        "new_string": "new",
        "replace_all": true
    });

    assert_eq!(extract_file_path(&input), Some("/tmp/test.txt"));
    assert_eq!(extract_str(&input, "old_string"), Some("old"));
    assert_eq!(extract_str(&input, "new_string"), Some("new"));
    assert!(extract_bool(&input, "replace_all", false));
}

#[test]
fn test_edit_missing_fields() {
    let executor = EditExecutor::new();
    let ctx = BuiltinContext::default();

    // Missing path
    let call = ToolCallSpec {
        tool: "Edit".to_string(),
        input: json!({ "old_string": "a", "new_string": "b" }),
        result: None,
    };
    let result = executor.execute(&call, "toolu_123", &ctx);
    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Missing"));
}

#[test]
fn test_edit_file_not_found() {
    let executor = EditExecutor::new();
    let call = ToolCallSpec {
        tool: "Edit".to_string(),
        input: json!({
            "file_path": "/nonexistent/file.txt",
            "old_string": "old",
            "new_string": "new"
        }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Failed to read"));
}

#[test]
fn test_edit_string_not_found() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "Hello, World!").unwrap();

    let executor = EditExecutor::new();
    let call = ToolCallSpec {
        tool: "Edit".to_string(),
        input: json!({
            "file_path": temp.path().to_str().unwrap(),
            "old_string": "nonexistent",
            "new_string": "new"
        }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("not found"));
}

#[test]
fn test_edit_successful() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "Hello, World!").unwrap();

    let executor = EditExecutor::new();
    let call = ToolCallSpec {
        tool: "Edit".to_string(),
        input: json!({
            "file_path": temp.path().to_str().unwrap(),
            "old_string": "World",
            "new_string": "Rust"
        }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(!result.is_error);
    assert!(result.text().unwrap().contains("Successfully edited"));

    // Verify the change
    let content = fs::read_to_string(temp.path()).unwrap();
    assert!(content.contains("Hello, Rust!"));
}

#[test]
fn test_tool_name() {
    assert_eq!(EditExecutor::new().tool_name(), "Edit");
}
