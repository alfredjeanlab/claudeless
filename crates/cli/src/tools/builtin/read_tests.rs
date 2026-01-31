// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::super::test_helpers::{
    assert_tool_error_contains, assert_tool_success_contains, execute, TestFile,
};
use super::*;
use crate::tools::builtin::extract_file_path;
use serde_json::json;
use yare::parameterized;

#[parameterized(
    file_path = { json!({ "file_path": "/tmp/test.txt" }), Some("/tmp/test.txt") },
    path = { json!({ "path": "/tmp/test.txt" }), Some("/tmp/test.txt") },
    empty = { json!({}), None },
)]
fn extract_path(input: serde_json::Value, expected: Option<&str>) {
    assert_eq!(extract_file_path(&input), expected);
}

#[test]
fn test_read_nonexistent_file() {
    assert_tool_error_contains(
        &execute::<ReadExecutor>(json!({ "file_path": "/nonexistent/file.txt" })),
        "Failed to read",
    );
}

#[test]
fn test_read_real_file() {
    let file = TestFile::new("test.txt").with_content("Hello, World!\n");
    assert_tool_success_contains(
        &execute::<ReadExecutor>(json!({ "file_path": file.path_str() })),
        "Hello, World!",
    );
}
