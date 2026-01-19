// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Content search (grep) executor.

use std::fs;
use std::path::PathBuf;

use regex::Regex;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{BuiltinContext, BuiltinToolExecutor};

/// Executor for content search (grep-like functionality).
#[derive(Clone, Debug, Default)]
pub struct GrepExecutor;

impl GrepExecutor {
    /// Create a new Grep executor.
    pub fn new() -> Self {
        Self
    }

    /// Extract pattern from tool input.
    fn extract_pattern(input: &serde_json::Value) -> Option<&str> {
        input.get("pattern").and_then(|v| v.as_str())
    }

    /// Extract path from tool input.
    fn extract_path(input: &serde_json::Value) -> Option<&str> {
        input.get("path").and_then(|v| v.as_str())
    }

    /// Extract glob filter from tool input.
    fn extract_glob(input: &serde_json::Value) -> Option<&str> {
        input.get("glob").and_then(|v| v.as_str())
    }

    /// Check if case-insensitive mode is enabled.
    fn case_insensitive(input: &serde_json::Value) -> bool {
        input.get("-i").and_then(|v| v.as_bool()).unwrap_or(false)
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
        let pattern_str = match Self::extract_pattern(&call.input) {
            Some(p) => p,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "Missing 'pattern' field in Grep tool input",
                )
            }
        };

        // Build regex
        let case_insensitive = Self::case_insensitive(&call.input);
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
        let search_path = Self::extract_path(&call.input)
            .map(|p| ctx.resolve_path(p))
            .or_else(|| ctx.cwd.clone())
            .or_else(|| ctx.sandbox_root.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        // Get glob filter
        let glob_pattern = Self::extract_glob(&call.input);

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

    fn tool_name(&self) -> &'static str {
        "Grep"
    }
}

#[cfg(test)]
#[path = "grep_tests.rs"]
mod tests;
