// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Glob pattern matching executor.

use std::path::PathBuf;

use glob::glob as glob_match;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{BuiltinContext, BuiltinToolExecutor};

/// Executor for glob pattern matching.
#[derive(Clone, Debug, Default)]
pub struct GlobExecutor;

impl GlobExecutor {
    /// Create a new Glob executor.
    pub fn new() -> Self {
        Self
    }

    /// Extract pattern from tool input.
    fn extract_pattern(input: &serde_json::Value) -> Option<&str> {
        input.get("pattern").and_then(|v| v.as_str())
    }

    /// Extract path/directory from tool input.
    fn extract_path(input: &serde_json::Value) -> Option<&str> {
        input
            .get("path")
            .or_else(|| input.get("directory"))
            .and_then(|v| v.as_str())
    }
}

impl BuiltinToolExecutor for GlobExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &BuiltinContext,
    ) -> ToolExecutionResult {
        let pattern = match Self::extract_pattern(&call.input) {
            Some(p) => p,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "Missing 'pattern' field in Glob tool input",
                )
            }
        };

        // Get the base directory
        let base_dir = Self::extract_path(&call.input)
            .map(|p| ctx.resolve_path(p))
            .or_else(|| ctx.cwd.clone())
            .or_else(|| ctx.sandbox_root.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        // Construct full pattern
        let full_pattern = if pattern.starts_with('/') || pattern.contains(':') {
            // Absolute pattern or Windows path
            ctx.resolve_path(pattern).to_string_lossy().to_string()
        } else {
            // Relative pattern
            base_dir.join(pattern).to_string_lossy().to_string()
        };

        // Execute glob
        match glob_match(&full_pattern) {
            Ok(paths) => {
                let matches: Vec<String> = paths
                    .filter_map(|entry| entry.ok())
                    .map(|path| path.to_string_lossy().to_string())
                    .collect();

                if matches.is_empty() {
                    ToolExecutionResult::success(tool_use_id, "No matches found")
                } else {
                    ToolExecutionResult::success(tool_use_id, matches.join("\n"))
                }
            }
            Err(e) => ToolExecutionResult::error(
                tool_use_id,
                format!("Invalid glob pattern '{}': {}", pattern, e),
            ),
        }
    }

    fn tool_name(&self) -> &'static str {
        "Glob"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_pattern() {
        let input = json!({ "pattern": "*.txt" });
        assert_eq!(GlobExecutor::extract_pattern(&input), Some("*.txt"));
    }

    #[test]
    fn test_extract_path() {
        let input1 = json!({ "path": "/tmp" });
        assert_eq!(GlobExecutor::extract_path(&input1), Some("/tmp"));

        let input2 = json!({ "directory": "/var" });
        assert_eq!(GlobExecutor::extract_path(&input2), Some("/var"));
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
}
