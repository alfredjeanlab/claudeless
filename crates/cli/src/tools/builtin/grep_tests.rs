// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::super::test_helpers::{
    assert_tool_error_contains, assert_tool_success_contains, execute,
};
use super::*;
use crate::tools::builtin::extract_str;
use serde_json::json;
use std::io::Write;
use tempfile::TempDir;

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
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "Hello, World!").unwrap();

    assert_tool_success_contains(
        &execute::<GrepExecutor>(json!({
            "pattern": "nonexistent",
            "path": temp_dir.path().to_str().unwrap()
        })),
        "No matches",
    );
}

#[test]
fn test_grep_with_matches() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "Hello, World!").unwrap();
    writeln!(file, "Goodbye, World!").unwrap();
    writeln!(file, "Hello again!").unwrap();

    let result = execute::<GrepExecutor>(json!({
        "pattern": "Hello",
        "path": temp_dir.path().to_str().unwrap()
    }));
    assert!(!result.is_error);
    let text = result.text().unwrap();
    assert!(text.contains("Hello, World!"));
    assert!(text.contains("Hello again!"));
    assert!(!text.contains("Goodbye"));
}

#[test]
fn test_grep_case_insensitive() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "HELLO").unwrap();
    writeln!(file, "hello").unwrap();
    writeln!(file, "Hello").unwrap();

    let result = execute::<GrepExecutor>(json!({
        "pattern": "hello",
        "path": temp_dir.path().to_str().unwrap(),
        "-i": true
    }));
    assert!(!result.is_error);
    let text = result.text().unwrap();
    assert!(text.contains("HELLO"));
    assert!(text.contains("hello"));
    assert!(text.contains("Hello"));
}

#[test]
fn test_tool_name() {
    assert_eq!(
        GrepExecutor::new().tool_name(),
        crate::tools::ToolName::Grep
    );
}
