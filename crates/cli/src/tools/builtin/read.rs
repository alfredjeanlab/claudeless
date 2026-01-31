// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File read executor.

use std::fs;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{extract_file_path, require_field, BuiltinContext, BuiltinToolExecutor};
use crate::tools::ToolName;

/// Executor for file reading.
#[derive(Clone, Debug, Default)]
pub struct ReadExecutor;

impl BuiltinToolExecutor for ReadExecutor {
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

        let resolved_path = std::path::PathBuf::from(path);

        match fs::read_to_string(&resolved_path) {
            Ok(content) => ToolExecutionResult::success(tool_use_id, content),
            Err(e) => ToolExecutionResult::error(
                tool_use_id,
                format!("Failed to read file '{}': {}", resolved_path.display(), e),
            ),
        }
    }

    fn tool_name(&self) -> ToolName {
        ToolName::Read
    }
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
