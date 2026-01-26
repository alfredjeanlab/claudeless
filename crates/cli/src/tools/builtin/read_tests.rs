// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use serde_json::json;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_extract_path() {
    let input1 = json!({ "file_path": "/tmp/test.txt" });
    assert_eq!(ReadExecutor::extract_path(&input1), Some("/tmp/test.txt"));

    let input2 = json!({ "path": "/tmp/test.txt" });
    assert_eq!(ReadExecutor::extract_path(&input2), Some("/tmp/test.txt"));

    let empty = json!({});
    assert_eq!(ReadExecutor::extract_path(&empty), None);
}

#[test]
fn test_read_missing_path() {
    let executor = ReadExecutor::new();
    let call = ToolCallSpec {
        tool: "Read".to_string(),
        input: json!({}),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Missing"));
}

#[test]
fn test_read_nonexistent_file() {
    let executor = ReadExecutor::new();
    let call = ToolCallSpec {
        tool: "Read".to_string(),
        input: json!({ "file_path": "/nonexistent/file.txt" }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Failed to read"));
}

#[test]
fn test_read_real_file() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "Hello, World!").unwrap();

    let executor = ReadExecutor::new();
    let call = ToolCallSpec {
        tool: "Read".to_string(),
        input: json!({ "file_path": temp.path().to_str().unwrap() }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(!result.is_error);
    assert!(result.text().unwrap().contains("Hello, World!"));
}

#[test]
fn test_tool_name() {
    assert_eq!(ReadExecutor::new().tool_name(), "Read");
}
