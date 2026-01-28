// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]
use super::*;
use crate::tools::builtin::{extract_file_path, extract_str};
use serde_json::json;
use tempfile::TempDir;

#[test]
fn test_extract_path() {
    let input1 = json!({ "file_path": "/tmp/test.txt" });
    assert_eq!(extract_file_path(&input1), Some("/tmp/test.txt"));

    let input2 = json!({ "path": "/tmp/test.txt" });
    assert_eq!(extract_file_path(&input2), Some("/tmp/test.txt"));
}

#[test]
fn test_extract_content() {
    let input = json!({ "content": "Hello, World!" });
    assert_eq!(extract_str(&input, "content"), Some("Hello, World!"));
}

#[test]
fn test_write_missing_path() {
    let executor = WriteExecutor::new();
    let call = ToolCallSpec {
        tool: "Write".to_string(),
        input: json!({ "content": "test" }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Missing"));
}

#[test]
fn test_write_missing_content() {
    let executor = WriteExecutor::new();
    let call = ToolCallSpec {
        tool: "Write".to_string(),
        input: json!({ "file_path": "/tmp/test.txt" }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Missing 'content'"));
}

#[test]
fn test_write_real_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    let executor = WriteExecutor::new();
    let call = ToolCallSpec {
        tool: "Write".to_string(),
        input: json!({
            "file_path": file_path.to_str().unwrap(),
            "content": "Hello, World!"
        }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(!result.is_error);
    assert!(result.text().unwrap().contains("Successfully wrote"));

    // Verify the file was written
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "Hello, World!");
}

#[test]
fn test_write_creates_parent_dirs() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("subdir/nested/test.txt");

    let executor = WriteExecutor::new();
    let call = ToolCallSpec {
        tool: "Write".to_string(),
        input: json!({
            "file_path": file_path.to_str().unwrap(),
            "content": "nested content"
        }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(!result.is_error);

    // Verify the file was written
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "nested content");
}

#[test]
fn test_tool_name() {
    assert_eq!(WriteExecutor::new().tool_name(), "Write");
}
