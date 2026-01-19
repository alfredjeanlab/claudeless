// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Bash command executor.

use std::process::Command;

use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;

use super::{BuiltinContext, BuiltinToolExecutor};

/// Executor for Bash commands.
#[derive(Clone, Debug, Default)]
pub struct BashExecutor;

impl BashExecutor {
    /// Create a new Bash executor.
    pub fn new() -> Self {
        Self
    }

    /// Extract command from tool input.
    fn extract_command(input: &serde_json::Value) -> Option<&str> {
        input.get("command").and_then(|v| v.as_str())
    }

    /// Execute command for real.
    fn execute_real(command: &str, ctx: &BuiltinContext) -> ToolExecutionResult {
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
                    ToolExecutionResult::success("", stdout.trim())
                } else {
                    let error_msg = if stderr.is_empty() {
                        format!("Command failed with exit code: {:?}", output.status.code())
                    } else {
                        stderr.trim().to_string()
                    };
                    ToolExecutionResult::error("", error_msg)
                }
            }
            Err(e) => ToolExecutionResult::error("", format!("Failed to execute command: {}", e)),
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
        let command = match Self::extract_command(&call.input) {
            Some(cmd) => cmd,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "Missing 'command' field in Bash tool input",
                )
            }
        };

        let mut result = Self::execute_real(command, ctx);
        result.tool_use_id = tool_use_id.to_string();
        result
    }

    fn tool_name(&self) -> &'static str {
        "Bash"
    }
}

#[cfg(test)]
#[path = "bash_tests.rs"]
mod tests;
