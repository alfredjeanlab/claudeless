// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::super::test_helpers::{assert_tool_success_contains, execute};
use super::*;
use crate::tools::builtin::{extract_directory, extract_str};
use serde_json::json;
use std::fs;
use tempfile::TempDir;
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
    let temp_dir = TempDir::new().unwrap();
    assert_tool_success_contains(
        &execute::<GlobExecutor>(json!({
            "pattern": "*.nonexistent",
            "path": temp_dir.path().to_str().unwrap()
        })),
        "No matches",
    );
}

#[test]
fn test_glob_with_matches() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("file1.txt"), "").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "").unwrap();
    fs::write(temp_dir.path().join("other.rs"), "").unwrap();

    let result = execute::<GlobExecutor>(json!({
        "pattern": "*.txt",
        "path": temp_dir.path().to_str().unwrap()
    }));
    assert!(!result.is_error);
    let text = result.text().unwrap();
    assert!(text.contains("file1.txt"));
    assert!(text.contains("file2.txt"));
    assert!(!text.contains("other.rs"));
}

#[test]
fn test_tool_name() {
    assert_eq!(GlobExecutor.tool_name(), crate::tools::ToolName::Glob);
}
