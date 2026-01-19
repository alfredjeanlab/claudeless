// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File read executor.

use std::fs;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{BuiltinContext, BuiltinToolExecutor};

/// Executor for file reading.
#[derive(Clone, Debug, Default)]
pub struct ReadExecutor;

impl ReadExecutor {
    /// Create a new Read executor.
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
}

impl BuiltinToolExecutor for ReadExecutor {
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
                    "Missing 'file_path' or 'path' field in Read tool input",
                )
            }
        };

        let resolved_path = ctx.resolve_path(path);

        match fs::read_to_string(&resolved_path) {
            Ok(content) => ToolExecutionResult::success(tool_use_id, content),
            Err(e) => ToolExecutionResult::error(
                tool_use_id,
                format!("Failed to read file '{}': {}", resolved_path.display(), e),
            ),
        }
    }

    fn tool_name(&self) -> &'static str {
        "Read"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_extract_path() {
        let input1 = json!({ "file_path": "/tmp/test.txt" });
        assert_eq!(ReadExecutor::extract_path(&input1), Some("/tmp/test.txt"));

        let input2 = json!({ "path": "/tmp/test.txt" });
        assert_eq!(ReadExecutor::extract_path(&input2), Some("/tmp/test.txt"));

        let empty = json!({});
        assert_eq!(ReadExecutor::extract_path(&empty), None);
    }

    #[test]
    fn test_read_missing_path() {
        let executor = ReadExecutor::new();
        let call = ToolCallSpec {
            tool: "Read".to_string(),
            input: json!({}),
            result: None,
        };
        let ctx = BuiltinContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(result.is_error);
        assert!(result.text().unwrap().contains("Missing"));
    }

    #[test]
    fn test_read_nonexistent_file() {
        let executor = ReadExecutor::new();
        let call = ToolCallSpec {
            tool: "Read".to_string(),
            input: json!({ "file_path": "/nonexistent/file.txt" }),
            result: None,
        };
        let ctx = BuiltinContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(result.is_error);
        assert!(result.text().unwrap().contains("Failed to read"));
    }

    #[test]
    fn test_read_real_file() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "Hello, World!").unwrap();

        let executor = ReadExecutor::new();
        let call = ToolCallSpec {
            tool: "Read".to_string(),
            input: json!({ "file_path": temp.path().to_str().unwrap() }),
            result: None,
        };
        let ctx = BuiltinContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(!result.is_error);
        assert!(result.text().unwrap().contains("Hello, World!"));
    }

    #[test]
    fn test_tool_name() {
        assert_eq!(ReadExecutor::new().tool_name(), "Read");
    }
}
