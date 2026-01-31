// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File edit executor.

use std::fs;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{
    extract_bool, extract_file_path, extract_str, require_field, BuiltinContext,
    BuiltinToolExecutor,
};
use crate::tools::tool_name::ToolName;

/// Executor for file editing (search and replace).
#[derive(Clone, Debug, Default)]
pub struct EditExecutor;

impl BuiltinToolExecutor for EditExecutor {
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
        let old_string = require_field!(
            call.input,
            "old_string",
            extract_str,
            tool_use_id,
            call.tool
        );
        let new_string = require_field!(
            call.input,
            "new_string",
            extract_str,
            tool_use_id,
            call.tool
        );

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

    fn tool_name(&self) -> ToolName {
        ToolName::Edit
    }
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
