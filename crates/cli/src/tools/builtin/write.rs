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
        _ctx: &BuiltinContext,
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

        let resolved_path = std::path::PathBuf::from(path);

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
#[path = "write_tests.rs"]
mod tests;
