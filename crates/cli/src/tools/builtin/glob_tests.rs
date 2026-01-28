// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use crate::tools::builtin::{extract_directory, extract_str};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_extract_pattern() {
    let input = json!({ "pattern": "*.txt" });
    assert_eq!(extract_str(&input, "pattern"), Some("*.txt"));
}

#[test]
fn test_extract_path() {
    let input1 = json!({ "path": "/tmp" });
    assert_eq!(extract_directory(&input1), Some("/tmp"));

    let input2 = json!({ "directory": "/var" });
    assert_eq!(extract_directory(&input2), Some("/var"));
}

#[test]
fn test_glob_missing_pattern() {
    let executor = GlobExecutor::new();
    let call = ToolCallSpec {
        tool: "Glob".to_string(),
        input: json!({}),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Missing 'pattern'"));
}

#[test]
fn test_glob_no_matches() {
    let temp_dir = TempDir::new().unwrap();

    let executor = GlobExecutor::new();
    let call = ToolCallSpec {
        tool: "Glob".to_string(),
        input: json!({
            "pattern": "*.nonexistent",
            "path": temp_dir.path().to_str().unwrap()
        }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(!result.is_error);
    assert!(result.text().unwrap().contains("No matches"));
}

#[test]
fn test_glob_with_matches() {
    let temp_dir = TempDir::new().unwrap();

    // Create some test files
    fs::write(temp_dir.path().join("file1.txt"), "").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "").unwrap();
    fs::write(temp_dir.path().join("other.rs"), "").unwrap();

    let executor = GlobExecutor::new();
    let call = ToolCallSpec {
        tool: "Glob".to_string(),
        input: json!({
            "pattern": "*.txt",
            "path": temp_dir.path().to_str().unwrap()
        }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(!result.is_error);
    let text = result.text().unwrap();
    assert!(text.contains("file1.txt"));
    assert!(text.contains("file2.txt"));
    assert!(!text.contains("other.rs"));
}

#[test]
fn test_tool_name() {
    assert_eq!(GlobExecutor::new().tool_name(), "Glob");
}
