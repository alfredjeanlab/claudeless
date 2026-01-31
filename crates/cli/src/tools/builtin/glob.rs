// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Glob pattern matching executor.

use std::path::PathBuf;

use glob::glob as glob_match;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{extract_directory, extract_str, require_field, BuiltinContext, BuiltinToolExecutor};

/// Executor for glob pattern matching.
#[derive(Clone, Debug, Default)]
pub struct GlobExecutor;

impl GlobExecutor {
    /// Create a new Glob executor.
    pub fn new() -> Self {
        Self
    }
}

impl BuiltinToolExecutor for GlobExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &BuiltinContext,
    ) -> ToolExecutionResult {
        let pattern = require_field!(call.input, "pattern", extract_str, tool_use_id, call.tool);

        // Get the base directory
        let base_dir = extract_directory(&call.input)
            .map(PathBuf::from)
            .or_else(|| ctx.cwd.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        // Construct full pattern
        let full_pattern = if pattern.starts_with('/') || pattern.contains(':') {
            // Absolute pattern or Windows path
            pattern.to_string()
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
#[path = "glob_tests.rs"]
mod tests;
