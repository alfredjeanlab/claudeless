// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::super::test_helpers::{
    assert_tool_error_contains, assert_tool_success_contains, execute, TestFile,
};
use super::*;
use crate::tools::builtin::{extract_bool, extract_file_path, extract_str};
use serde_json::json;

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
fn test_edit_nonexistent_file() {
    assert_tool_error_contains(
        &execute::<EditExecutor>(json!({
            "file_path": "/nonexistent/file.txt",
            "old_string": "old",
            "new_string": "new"
        })),
        "Failed to read",
    );
}

#[test]
fn test_edit_string_not_found() {
    let file = TestFile::new("test.txt").with_content("Hello, World!\n");
    assert_tool_error_contains(
        &execute::<EditExecutor>(json!({
            "file_path": file.path_str(),
            "old_string": "nonexistent",
            "new_string": "new"
        })),
        "not found",
    );
}

#[test]
fn test_edit_successful() {
    let file = TestFile::new("test.txt").with_content("Hello, World!\n");
    assert_tool_success_contains(
        &execute::<EditExecutor>(json!({
            "file_path": file.path_str(),
            "old_string": "World",
            "new_string": "Rust"
        })),
        "Successfully edited",
    );
    assert!(fs::read_to_string(&file.path)
        .unwrap()
        .contains("Hello, Rust!"));
}
