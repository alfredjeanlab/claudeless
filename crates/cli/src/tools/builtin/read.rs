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
        _ctx: &BuiltinContext,
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

        let resolved_path = std::path::PathBuf::from(path);

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
#[path = "read_tests.rs"]
mod tests;
