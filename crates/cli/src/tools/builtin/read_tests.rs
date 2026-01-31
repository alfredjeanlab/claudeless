// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::super::test_helpers::execute;
use super::*;
use crate::tools::builtin::extract_file_path;
use serde_json::json;
use std::io::Write;
use tempfile::NamedTempFile;
use yare::parameterized;

#[parameterized(
    file_path = { json!({ "file_path": "/tmp/test.txt" }), Some("/tmp/test.txt") },
    path = { json!({ "path": "/tmp/test.txt" }), Some("/tmp/test.txt") },
    empty = { json!({}), None },
)]
fn extract_path(input: serde_json::Value, expected: Option<&str>) {
    assert_eq!(extract_file_path(&input), expected);
}

#[parameterized(
    missing_path = { json!({}), "Missing" },
    nonexistent = { json!({ "file_path": "/nonexistent/file.txt" }), "Failed to read" },
)]
fn read_error_cases(input: serde_json::Value, expected: &str) {
    let result = execute::<ReadExecutor>(input);
    assert!(result.is_error);
    assert!(result.text().unwrap().contains(expected));
}

#[test]
fn test_read_real_file() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "Hello, World!").unwrap();

    let result = execute::<ReadExecutor>(json!({ "file_path": temp.path().to_str().unwrap() }));
    assert!(!result.is_error);
    assert!(result.text().unwrap().contains("Hello, World!"));
}

#[test]
fn test_tool_name() {
    assert_eq!(ReadExecutor.tool_name(), crate::tools::ToolName::Read);
}
