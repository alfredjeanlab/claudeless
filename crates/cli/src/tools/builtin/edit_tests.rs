// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::super::test_helpers::execute;
use super::*;
use crate::tools::builtin::{extract_bool, extract_file_path, extract_str};
use serde_json::json;
use std::io::Write;
use tempfile::NamedTempFile;
use yare::parameterized;

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

#[parameterized(
    missing_path = { json!({ "old_string": "a", "new_string": "b" }), "Missing" },
    file_not_found = { json!({ "file_path": "/nonexistent/file.txt", "old_string": "old", "new_string": "new" }), "Failed to read" },
)]
fn edit_error_cases(input: serde_json::Value, expected: &str) {
    let result = execute::<EditExecutor>(input);
    assert!(result.is_error);
    assert!(result.text().unwrap().contains(expected));
}

#[test]
fn test_edit_string_not_found() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "Hello, World!").unwrap();

    let result = execute::<EditExecutor>(json!({
        "file_path": temp.path().to_str().unwrap(),
        "old_string": "nonexistent",
        "new_string": "new"
    }));
    assert!(result.is_error);
    assert!(result.text().unwrap().contains("not found"));
}

#[test]
fn test_edit_successful() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "Hello, World!").unwrap();

    let result = execute::<EditExecutor>(json!({
        "file_path": temp.path().to_str().unwrap(),
        "old_string": "World",
        "new_string": "Rust"
    }));
    assert!(!result.is_error);
    assert!(result.text().unwrap().contains("Successfully edited"));
    assert!(fs::read_to_string(temp.path())
        .unwrap()
        .contains("Hello, Rust!"));
}

#[test]
fn test_tool_name() {
    assert_eq!(
        EditExecutor::new().tool_name(),
        crate::tools::ToolName::Edit
    );
}
