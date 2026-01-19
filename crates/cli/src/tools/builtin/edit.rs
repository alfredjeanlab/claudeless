// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File edit executor.

use std::fs;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{BuiltinContext, BuiltinToolExecutor};

/// Executor for file editing (search and replace).
#[derive(Clone, Debug, Default)]
pub struct EditExecutor;

impl EditExecutor {
    /// Create a new Edit executor.
    pub fn new() -> Self {
        Self
    }

    /// Extract file path from tool input.
    fn extract_path(input: &serde_json::Value) -> Option<&str> {
        input
            .get("file_path")
            .or_else(|| input.get("path"))
            .and_then(|v| v.as_str())
    }

    /// Extract old_string from tool input.
    fn extract_old_string(input: &serde_json::Value) -> Option<&str> {
        input.get("old_string").and_then(|v| v.as_str())
    }

    /// Extract new_string from tool input.
    fn extract_new_string(input: &serde_json::Value) -> Option<&str> {
        input.get("new_string").and_then(|v| v.as_str())
    }

    /// Check if replace_all is enabled.
    fn replace_all(input: &serde_json::Value) -> bool {
        input
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }
}

impl BuiltinToolExecutor for EditExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &BuiltinContext,
    ) -> ToolExecutionResult {
        let path = match Self::extract_path(&call.input) {
            Some(p) => p,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "Missing 'file_path' or 'path' field in Edit tool input",
                )
            }
        };

        let old_string = match Self::extract_old_string(&call.input) {
            Some(s) => s,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "Missing 'old_string' field in Edit tool input",
                )
            }
        };

        let new_string = match Self::extract_new_string(&call.input) {
            Some(s) => s,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "Missing 'new_string' field in Edit tool input",
                )
            }
        };

        let resolved_path = ctx.resolve_path(path);

        // Read the file
        let content = match fs::read_to_string(&resolved_path) {
            Ok(c) => c,
            Err(e) => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    format!("Failed to read file '{}': {}", resolved_path.display(), e),
                )
            }
        };

        // Check if old_string exists
        if !content.contains(old_string) {
            return ToolExecutionResult::error(
                tool_use_id,
                format!(
                    "old_string not found in file '{}'. Make sure it matches exactly.",
                    resolved_path.display()
                ),
            );
        }

        // Perform replacement
        let replace_all = Self::replace_all(&call.input);
        let (new_content, count) = if replace_all {
            let count = content.matches(old_string).count();
            (content.replace(old_string, new_string), count)
        } else {
            // Replace first occurrence only
            let count = if content.contains(old_string) { 1 } else { 0 };
            (content.replacen(old_string, new_string, 1), count)
        };

        // Check for uniqueness when not using replace_all
        if !replace_all && content.matches(old_string).count() > 1 {
            return ToolExecutionResult::error(
                tool_use_id,
                format!(
                    "old_string is not unique in file '{}'. Found {} occurrences. \
                     Use replace_all=true to replace all, or provide more context.",
                    resolved_path.display(),
                    content.matches(old_string).count()
                ),
            );
        }

        // Write the modified content
        match fs::write(&resolved_path, &new_content) {
            Ok(()) => ToolExecutionResult::success(
                tool_use_id,
                format!(
                    "Successfully edited {}: replaced {} occurrence(s)",
                    resolved_path.display(),
                    count
                ),
            ),
            Err(e) => ToolExecutionResult::error(
                tool_use_id,
                format!("Failed to write file '{}': {}", resolved_path.display(), e),
            ),
        }
    }

    fn tool_name(&self) -> &'static str {
        "Edit"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_extract_fields() {
        let input = json!({
            "file_path": "/tmp/test.txt",
            "old_string": "old",
            "new_string": "new",
            "replace_all": true
        });

        assert_eq!(EditExecutor::extract_path(&input), Some("/tmp/test.txt"));
        assert_eq!(EditExecutor::extract_old_string(&input), Some("old"));
        assert_eq!(EditExecutor::extract_new_string(&input), Some("new"));
        assert!(EditExecutor::replace_all(&input));
    }

    #[test]
    fn test_edit_missing_fields() {
        let executor = EditExecutor::new();
        let ctx = BuiltinContext::default();

        // Missing path
        let call = ToolCallSpec {
            tool: "Edit".to_string(),
            input: json!({ "old_string": "a", "new_string": "b" }),
            result: None,
        };
        let result = executor.execute(&call, "toolu_123", &ctx);
        assert!(result.is_error);
        assert!(result.text().unwrap().contains("Missing"));
    }

    #[test]
    fn test_edit_file_not_found() {
        let executor = EditExecutor::new();
        let call = ToolCallSpec {
            tool: "Edit".to_string(),
            input: json!({
                "file_path": "/nonexistent/file.txt",
                "old_string": "old",
                "new_string": "new"
            }),
            result: None,
        };
        let ctx = BuiltinContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(result.is_error);
        assert!(result.text().unwrap().contains("Failed to read"));
    }

    #[test]
    fn test_edit_string_not_found() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "Hello, World!").unwrap();

        let executor = EditExecutor::new();
        let call = ToolCallSpec {
            tool: "Edit".to_string(),
            input: json!({
                "file_path": temp.path().to_str().unwrap(),
                "old_string": "nonexistent",
                "new_string": "new"
            }),
            result: None,
        };
        let ctx = BuiltinContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(result.is_error);
        assert!(result.text().unwrap().contains("not found"));
    }

    #[test]
    fn test_edit_successful() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "Hello, World!").unwrap();

        let executor = EditExecutor::new();
        let call = ToolCallSpec {
            tool: "Edit".to_string(),
            input: json!({
                "file_path": temp.path().to_str().unwrap(),
                "old_string": "World",
                "new_string": "Rust"
            }),
            result: None,
        };
        let ctx = BuiltinContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(!result.is_error);
        assert!(result.text().unwrap().contains("Successfully edited"));

        // Verify the change
        let content = fs::read_to_string(temp.path()).unwrap();
        assert!(content.contains("Hello, Rust!"));
    }

    #[test]
    fn test_tool_name() {
        assert_eq!(EditExecutor::new().tool_name(), "Edit");
    }
}
