// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Bash command executor.

use std::process::Command;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{extract_str, BuiltinContext, BuiltinToolExecutor};

/// Executor for Bash commands.
#[derive(Clone, Debug, Default)]
pub struct BashExecutor;

impl BashExecutor {
    /// Create a new Bash executor.
    pub fn new() -> Self {
        Self
    }

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

                if output.status.success() {
                    ToolExecutionResult::success(tool_use_id, stdout.trim())
                } else {
                    let error_msg = if stderr.is_empty() {
                        format!("Command failed with exit code: {:?}", output.status.code())
                    } else {
                        stderr.trim().to_string()
                    };
                    ToolExecutionResult::error(tool_use_id, error_msg)
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
        let command = match extract_str(&call.input, "command") {
            Some(cmd) => cmd,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "Missing 'command' field in Bash tool input",
                )
            }
        };

        Self::execute_real(command, ctx, tool_use_id)
    }

    fn tool_name(&self) -> &'static str {
        "Bash"
    }
}

#[cfg(test)]
#[path = "bash_tests.rs"]
mod tests;
