// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::super::test_helpers::{assert_tool_success_contains, execute, TestFile};
use super::*;
use crate::tools::builtin::{extract_file_path, extract_str};
use serde_json::json;
use yare::parameterized;

#[parameterized(
    file_path = { json!({ "file_path": "/tmp/test.txt" }), Some("/tmp/test.txt") },
    path = { json!({ "path": "/tmp/test.txt" }), Some("/tmp/test.txt") },
)]
fn extract_path(input: serde_json::Value, expected: Option<&str>) {
    assert_eq!(extract_file_path(&input), expected);
}

#[test]
fn test_extract_content() {
    assert_eq!(
        extract_str(&json!({ "content": "Hello, World!" }), "content"),
        Some("Hello, World!")
    );
}

#[test]
fn test_write_real_file() {
    let file = TestFile::new("test.txt");
    assert_tool_success_contains(
        &execute::<WriteExecutor>(json!({
            "file_path": file.path_str(),
            "content": "Hello, World!"
        })),
        "Successfully wrote",
    );
    assert_eq!(fs::read_to_string(&file.path).unwrap(), "Hello, World!");
}

#[test]
fn test_write_creates_parent_dirs() {
    let file = TestFile::new("subdir/nested/test.txt");
    assert_tool_success_contains(
        &execute::<WriteExecutor>(json!({
            "file_path": file.path_str(),
            "content": "nested content"
        })),
        "Successfully wrote",
    );
    assert_eq!(fs::read_to_string(&file.path).unwrap(), "nested content");
}
