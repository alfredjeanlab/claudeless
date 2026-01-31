// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Content search (grep) executor.

use std::fs;
use std::path::PathBuf;

use regex::Regex;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{extract_bool, extract_str, require_field, BuiltinContext, BuiltinToolExecutor};
use crate::tools::ToolName;

/// Executor for content search (grep-like functionality).
#[derive(Clone, Debug, Default)]
pub struct GrepExecutor;

impl GrepExecutor {
    /// Create a new Grep executor.
    pub fn new() -> Self {
        Self
    }

    /// Recursively collect files from a directory.
    fn collect_files(path: &PathBuf, glob_pattern: Option<&str>) -> Vec<PathBuf> {
        let mut files = Vec::new();

        if path.is_file() {
            files.push(path.clone());
        } else if path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        // Apply glob filter if provided
                        if let Some(pattern) = glob_pattern {
                            let file_name = entry_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            if let Ok(glob) = glob::Pattern::new(pattern) {
                                if glob.matches(file_name) {
                                    files.push(entry_path);
                                }
                            }
                        } else {
                            files.push(entry_path);
                        }
                    } else if entry_path.is_dir() {
                        files.extend(Self::collect_files(&entry_path, glob_pattern));
                    }
                }
            }
        }

        files
    }
}

impl BuiltinToolExecutor for GrepExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &BuiltinContext,
    ) -> ToolExecutionResult {
        let pattern_str =
            require_field!(call.input, "pattern", extract_str, tool_use_id, call.tool);

        // Build regex
        let case_insensitive = extract_bool(&call.input, "-i", false);
        let regex = if case_insensitive {
            Regex::new(&format!("(?i){}", pattern_str))
        } else {
            Regex::new(pattern_str)
        };

        let regex = match regex {
            Ok(r) => r,
            Err(e) => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    format!("Invalid regex pattern '{}': {}", pattern_str, e),
                )
            }
        };

        // Get search path
        let search_path = extract_str(&call.input, "path")
            .map(PathBuf::from)
            .or_else(|| ctx.cwd.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        // Get glob filter
        let glob_pattern = extract_str(&call.input, "glob");

        // Collect files to search
        let files = Self::collect_files(&search_path, glob_pattern);

        // Search through files
        let mut matches = Vec::new();
        for file in files {
            if let Ok(content) = fs::read_to_string(&file) {
                for (line_num, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        matches.push(format!("{}:{}:{}", file.display(), line_num + 1, line));
                    }
                }
            }
        }

        if matches.is_empty() {
            ToolExecutionResult::success(tool_use_id, "No matches found")
        } else {
            ToolExecutionResult::success(tool_use_id, matches.join("\n"))
        }
    }

    fn tool_name(&self) -> ToolName {
        ToolName::Grep
    }
}

#[cfg(test)]
#[path = "grep_tests.rs"]
mod tests;
