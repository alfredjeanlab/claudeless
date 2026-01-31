// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File write executor.

use std::fs;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{extract_file_path, extract_str, require_field, BuiltinContext, BuiltinToolExecutor};
use crate::tools::ToolName;

/// Executor for file writing.
#[derive(Clone, Debug, Default)]
pub struct WriteExecutor;

impl BuiltinToolExecutor for WriteExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        _ctx: &BuiltinContext,
    ) -> ToolExecutionResult {
        let path = require_field!(
            call.input,
            extract_file_path => "'file_path' or 'path'",
            tool_use_id,
            call.tool
        );
        let content = require_field!(call.input, "content", extract_str, tool_use_id, call.tool);

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

    fn tool_name(&self) -> ToolName {
        ToolName::Write
    }
}

#[cfg(test)]
#[path = "write_tests.rs"]
mod tests;
