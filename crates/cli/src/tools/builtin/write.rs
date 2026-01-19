// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File write executor.

use std::fs;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{BuiltinContext, BuiltinToolExecutor};

/// Executor for file writing.
#[derive(Clone, Debug, Default)]
pub struct WriteExecutor;

impl WriteExecutor {
    /// Create a new Write executor.
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

    /// Extract content from tool input.
    fn extract_content(input: &serde_json::Value) -> Option<&str> {
        input.get("content").and_then(|v| v.as_str())
    }
}

impl BuiltinToolExecutor for WriteExecutor {
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
                    "Missing 'file_path' or 'path' field in Write tool input",
                )
            }
        };

        let content = match Self::extract_content(&call.input) {
            Some(c) => c,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "Missing 'content' field in Write tool input",
                )
            }
        };

        let resolved_path = ctx.resolve_path(path);

        // Create parent directories if needed
        if let Some(parent) = resolved_path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return ToolExecutionResult::error(
                        tool_use_id,
                        format!("Failed to create parent directories: {}", e),
                    );
                }
            }
        }

        match fs::write(&resolved_path, content) {
            Ok(()) => ToolExecutionResult::success(
                tool_use_id,
                format!(
                    "Successfully wrote {} bytes to {}",
                    content.len(),
                    resolved_path.display()
                ),
            ),
            Err(e) => ToolExecutionResult::error(
                tool_use_id,
                format!("Failed to write file '{}': {}", resolved_path.display(), e),
            ),
        }
    }

    fn tool_name(&self) -> &'static str {
        "Write"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_extract_path() {
        let input1 = json!({ "file_path": "/tmp/test.txt" });
        assert_eq!(WriteExecutor::extract_path(&input1), Some("/tmp/test.txt"));

        let input2 = json!({ "path": "/tmp/test.txt" });
        assert_eq!(WriteExecutor::extract_path(&input2), Some("/tmp/test.txt"));
    }

    #[test]
    fn test_extract_content() {
        let input = json!({ "content": "Hello, World!" });
        assert_eq!(
            WriteExecutor::extract_content(&input),
            Some("Hello, World!")
        );
    }

    #[test]
    fn test_write_missing_path() {
        let executor = WriteExecutor::new();
        let call = ToolCallSpec {
            tool: "Write".to_string(),
            input: json!({ "content": "test" }),
            result: None,
        };
        let ctx = BuiltinContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(result.is_error);
        assert!(result.text().unwrap().contains("Missing"));
    }

    #[test]
    fn test_write_missing_content() {
        let executor = WriteExecutor::new();
        let call = ToolCallSpec {
            tool: "Write".to_string(),
            input: json!({ "file_path": "/tmp/test.txt" }),
            result: None,
        };
        let ctx = BuiltinContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(result.is_error);
        assert!(result.text().unwrap().contains("Missing 'content'"));
    }

    #[test]
    fn test_write_real_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let executor = WriteExecutor::new();
        let call = ToolCallSpec {
            tool: "Write".to_string(),
            input: json!({
                "file_path": file_path.to_str().unwrap(),
                "content": "Hello, World!"
            }),
            result: None,
        };
        let ctx = BuiltinContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(!result.is_error);
        assert!(result.text().unwrap().contains("Successfully wrote"));

        // Verify the file was written
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_write_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("subdir/nested/test.txt");

        let executor = WriteExecutor::new();
        let call = ToolCallSpec {
            tool: "Write".to_string(),
            input: json!({
                "file_path": file_path.to_str().unwrap(),
                "content": "nested content"
            }),
            result: None,
        };
        let ctx = BuiltinContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(!result.is_error);

        // Verify the file was written
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "nested content");
    }

    #[test]
    fn test_tool_name() {
        assert_eq!(WriteExecutor::new().tool_name(), "Write");
    }
}
