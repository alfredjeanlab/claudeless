// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared input extraction helpers for builtin tools.

use serde_json::Value;

/// Extract a required field from tool input, returning early with an error if missing.
///
/// # Variants
///
/// For extraction functions that take `(input, field_name)`:
/// ```ignore
/// let cmd = require_field!(call.input, "command", extract_str, tool_use_id, call.tool);
/// ```
///
/// For extraction functions that take just `(input)`, use `=>` with field description:
/// ```ignore
/// let path = require_field!(call.input, extract_file_path => "'file_path' or 'path'", tool_use_id, call.tool);
/// ```
#[macro_export]
macro_rules! require_field {
    // For extraction functions that take (input, field_name)
    ($input:expr, $field:literal, $extract_fn:ident, $tool_use_id:expr, $tool_name:expr) => {
        match $extract_fn(&$input, $field) {
            Some(v) => v,
            None => {
                return $crate::tools::result::ToolExecutionResult::error(
                    $tool_use_id,
                    format!("Missing '{}' field in {} tool input", $field, $tool_name),
                )
            }
        }
    };
    // For extraction functions that take just (input), with custom field description
    ($input:expr, $extract_fn:ident => $field_desc:literal, $tool_use_id:expr, $tool_name:expr) => {
        match $extract_fn(&$input) {
            Some(v) => v,
            None => {
                return $crate::tools::result::ToolExecutionResult::error(
                    $tool_use_id,
                    format!("Missing {} field in {} tool input", $field_desc, $tool_name),
                )
            }
        }
    };
}

pub use require_field;

/// Extract a file path from tool input.
/// Checks "file_path" first, then "path" as fallback.
pub(crate) fn extract_file_path(input: &Value) -> Option<&str> {
    input
        .get("file_path")
        .or_else(|| input.get("path"))
        .and_then(|v| v.as_str())
}

/// Extract a directory/path from tool input.
/// Checks "path" first, then "directory" as fallback.
pub(crate) fn extract_directory(input: &Value) -> Option<&str> {
    input
        .get("path")
        .or_else(|| input.get("directory"))
        .and_then(|v| v.as_str())
}

/// Extract a string field by name.
pub(crate) fn extract_str<'a>(input: &'a Value, key: &str) -> Option<&'a str> {
    input.get(key).and_then(|v| v.as_str())
}

/// Extract a boolean field with default.
pub(crate) fn extract_bool(input: &Value, key: &str, default: bool) -> bool {
    input.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
