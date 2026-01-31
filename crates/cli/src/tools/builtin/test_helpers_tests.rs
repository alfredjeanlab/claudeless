// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_helpers::{assert_tool_error_contains, execute_tool};
use serde_json::json;
use yare::parameterized;

/// Consolidated tests for missing required field errors.
#[parameterized(
    bash_command = { "Bash", json!({}), "command" },
    read_path = { "Read", json!({}), "file_path" },
    write_path = { "Write", json!({ "content": "x" }), "file_path" },
    write_content = { "Write", json!({ "file_path": "/tmp/x" }), "content" },
    edit_path = { "Edit", json!({ "old_string": "a", "new_string": "b" }), "file_path" },
    glob_pattern = { "Glob", json!({}), "pattern" },
    grep_pattern = { "Grep", json!({}), "pattern" },
)]
fn missing_field_returns_error(tool: &str, input: serde_json::Value, field: &str) {
    assert_tool_error_contains(&execute_tool(tool, input), &format!("Missing '{field}'"));
}
