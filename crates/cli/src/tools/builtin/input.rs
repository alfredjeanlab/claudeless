// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared input extraction helpers for builtin tools.

use serde_json::Value;

/// Extract a file path from tool input.
/// Checks "file_path" first, then "path" as fallback.
pub fn extract_file_path(input: &Value) -> Option<&str> {
    input
        .get("file_path")
        .or_else(|| input.get("path"))
        .and_then(|v| v.as_str())
}

/// Extract a directory/path from tool input.
/// Checks "path" first, then "directory" as fallback.
pub fn extract_directory(input: &Value) -> Option<&str> {
    input
        .get("path")
        .or_else(|| input.get("directory"))
        .and_then(|v| v.as_str())
}

/// Extract a pattern from tool input.
pub fn extract_pattern(input: &Value) -> Option<&str> {
    input.get("pattern").and_then(|v| v.as_str())
}

/// Extract a command from tool input.
pub fn extract_command(input: &Value) -> Option<&str> {
    input.get("command").and_then(|v| v.as_str())
}

/// Extract a string field by name.
pub fn extract_str<'a>(input: &'a Value, key: &str) -> Option<&'a str> {
    input.get(key).and_then(|v| v.as_str())
}

/// Extract a boolean field with default.
pub fn extract_bool(input: &Value, key: &str, default: bool) -> bool {
    input.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
