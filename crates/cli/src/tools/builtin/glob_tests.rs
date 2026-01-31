// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::super::test_helpers::{assert_tool_success_contains, execute, TestDir};
use super::*;
use crate::tools::builtin::{extract_directory, extract_str};
use serde_json::json;
use yare::parameterized;

#[parameterized(
    pattern = { json!({ "pattern": "*.txt" }), "pattern", Some("*.txt") },
    path = { json!({ "path": "/tmp" }), "path", Some("/tmp") },
)]
fn extract_str_fields(input: serde_json::Value, field: &str, expected: Option<&str>) {
    assert_eq!(extract_str(&input, field), expected);
}

#[parameterized(
    path = { json!({ "path": "/tmp" }), Some("/tmp") },
    directory = { json!({ "directory": "/var" }), Some("/var") },
)]
fn extract_directory_fields(input: serde_json::Value, expected: Option<&str>) {
    assert_eq!(extract_directory(&input), expected);
}

#[test]
fn test_glob_no_matches() {
    let dir = TestDir::new();
    assert_tool_success_contains(
        &execute::<GlobExecutor>(json!({
            "pattern": "*.nonexistent",
            "path": dir.path_str()
        })),
        "No matches",
    );
}

#[test]
fn test_glob_with_matches() {
    let dir = TestDir::new()
        .with_file("file1.txt", "")
        .with_file("file2.txt", "")
        .with_file("other.rs", "");

    let result = execute::<GlobExecutor>(json!({
        "pattern": "*.txt",
        "path": dir.path_str()
    }));
    assert!(!result.is_error);
    let text = result.text().unwrap();
    assert!(text.contains("file1.txt"));
    assert!(text.contains("file2.txt"));
    assert!(!text.contains("other.rs"));
}
