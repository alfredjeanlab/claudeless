// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File edit executor.

use std::fs;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{extract_bool, extract_file_path, extract_str, BuiltinContext, BuiltinToolExecutor};

/// Executor for file editing (search and replace).
#[derive(Clone, Debug, Default)]
pub struct EditExecutor;

impl EditExecutor {
    /// Create a new Edit executor.
    pub fn new() -> Self {
        Self
    }
}

impl BuiltinToolExecutor for EditExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        _ctx: &BuiltinContext,
    ) -> ToolExecutionResult {
        let path = match extract_file_path(&call.input) {
            Some(p) => p,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "Missing 'file_path' or 'path' field in Edit tool input",
                )
            }
        };

        let old_string = match extract_str(&call.input, "old_string") {
            Some(s) => s,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "Missing 'old_string' field in Edit tool input",
                )
            }
        };

        let new_string = match extract_str(&call.input, "new_string") {
            Some(s) => s,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "Missing 'new_string' field in Edit tool input",
                )
            }
        };

        let resolved_path = std::path::PathBuf::from(path);

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
        let replace_all = extract_bool(&call.input, "replace_all", false);
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
#[path = "edit_tests.rs"]
mod tests;
