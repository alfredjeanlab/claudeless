// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::super::test_helpers::{
    assert_tool_error_contains, assert_tool_success_contains, execute, TestDir,
};
use super::*;
use crate::tools::builtin::extract_str;
use serde_json::json;

#[test]
fn test_extract_pattern() {
    assert_eq!(
        extract_str(&json!({ "pattern": "fn main" }), "pattern"),
        Some("fn main")
    );
}

#[test]
fn test_grep_invalid_regex() {
    assert_tool_error_contains(
        &execute::<GrepExecutor>(json!({ "pattern": "[invalid" })),
        "Invalid regex",
    );
}

#[test]
fn test_grep_no_matches() {
    let dir = TestDir::new().with_file("test.txt", "Hello, World!\n");
    assert_tool_success_contains(
        &execute::<GrepExecutor>(json!({
            "pattern": "nonexistent",
            "path": dir.path_str()
        })),
        "No matches",
    );
}

#[test]
fn test_grep_with_matches() {
    let dir =
        TestDir::new().with_file("test.txt", "Hello, World!\nGoodbye, World!\nHello again!\n");

    let result = execute::<GrepExecutor>(json!({
        "pattern": "Hello",
        "path": dir.path_str()
    }));
    assert!(!result.is_error);
    let text = result.text().unwrap();
    assert!(text.contains("Hello, World!"));
    assert!(text.contains("Hello again!"));
    assert!(!text.contains("Goodbye"));
}

#[test]
fn test_grep_case_insensitive() {
    let dir = TestDir::new().with_file("test.txt", "HELLO\nhello\nHello\n");

    let result = execute::<GrepExecutor>(json!({
        "pattern": "hello",
        "path": dir.path_str(),
        "-i": true
    }));
    assert!(!result.is_error);
    let text = result.text().unwrap();
    assert!(text.contains("HELLO"));
    assert!(text.contains("hello"));
    assert!(text.contains("Hello"));
}
