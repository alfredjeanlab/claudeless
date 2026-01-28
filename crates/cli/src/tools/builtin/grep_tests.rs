// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]
use super::*;
use crate::tools::builtin::extract_str;
use serde_json::json;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_extract_pattern() {
    let input = json!({ "pattern": "fn main" });
    assert_eq!(extract_str(&input, "pattern"), Some("fn main"));
}

#[test]
fn test_grep_missing_pattern() {
    let executor = GrepExecutor::new();
    let call = ToolCallSpec {
        tool: "Grep".to_string(),
        input: json!({}),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Missing 'pattern'"));
}

#[test]
fn test_grep_invalid_regex() {
    let executor = GrepExecutor::new();
    let call = ToolCallSpec {
        tool: "Grep".to_string(),
        input: json!({ "pattern": "[invalid" }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(result.is_error);
    assert!(result.text().unwrap().contains("Invalid regex"));
}

#[test]
fn test_grep_no_matches() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "Hello, World!").unwrap();

    let executor = GrepExecutor::new();
    let call = ToolCallSpec {
        tool: "Grep".to_string(),
        input: json!({
            "pattern": "nonexistent",
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
fn test_grep_with_matches() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "Hello, World!").unwrap();
    writeln!(file, "Goodbye, World!").unwrap();
    writeln!(file, "Hello again!").unwrap();

    let executor = GrepExecutor::new();
    let call = ToolCallSpec {
        tool: "Grep".to_string(),
        input: json!({
            "pattern": "Hello",
            "path": temp_dir.path().to_str().unwrap()
        }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

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

    let executor = GrepExecutor::new();
    let call = ToolCallSpec {
        tool: "Grep".to_string(),
        input: json!({
            "pattern": "hello",
            "path": temp_dir.path().to_str().unwrap(),
            "-i": true
        }),
        result: None,
    };
    let ctx = BuiltinContext::default();
    let result = executor.execute(&call, "toolu_123", &ctx);

    assert!(!result.is_error);
    let text = result.text().unwrap();
    // Should match all three lines
    assert!(text.contains("HELLO"));
    assert!(text.contains("hello"));
    assert!(text.contains("Hello"));
}

#[test]
fn test_tool_name() {
    assert_eq!(GrepExecutor::new().tool_name(), "Grep");
}
