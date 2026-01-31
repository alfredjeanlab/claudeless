// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Bash command executor.

use std::process::Command;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{extract_str, require_field, BuiltinContext, BuiltinToolExecutor};
use crate::tools::tool_name::ToolName;

/// Executor for Bash commands.
#[derive(Clone, Debug, Default)]
pub struct BashExecutor;

impl BashExecutor {
    /// Execute command for real.
    fn execute_real(command: &str, ctx: &BuiltinContext, tool_use_id: &str) -> ToolExecutionResult {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);

        // Set working directory if specified
        if let Some(ref cwd) = ctx.cwd {
            cmd.current_dir(cwd);
        }

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                // Always include exit code in output for log extraction
                let result_text = if output.status.success() {
                    format!("{}\n\nExit code: {}", stdout.trim(), exit_code)
                } else {
                    let error_content = if stderr.is_empty() {
                        stdout.trim().to_string()
                    } else {
                        format!("{}\n{}", stdout.trim(), stderr.trim())
                    };
                    format!("{}\n\nExit code: {}", error_content.trim(), exit_code)
                };

                if output.status.success() {
                    ToolExecutionResult::success(tool_use_id, result_text)
                } else {
                    ToolExecutionResult::error(tool_use_id, result_text)
                }
            }
            Err(e) => {
                ToolExecutionResult::error(tool_use_id, format!("Failed to execute command: {}", e))
            }
        }
    }
}

impl BuiltinToolExecutor for BashExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &BuiltinContext,
    ) -> ToolExecutionResult {
        let command = require_field!(call.input, "command", extract_str, tool_use_id, call.tool);

        Self::execute_real(command, ctx, tool_use_id)
    }

    fn tool_name(&self) -> ToolName {
        ToolName::Bash
    }
}

#[cfg(test)]
#[path = "bash_tests.rs"]
mod tests;
