// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Built-in tool executors for live mode.
//!
//! This module provides executors for Claude's built-in tools:
//! - Bash - Command execution
//! - Read - File reading
//! - Write - File writing
//! - Edit - File editing
//! - Glob - Pattern matching
//! - Grep - Content search

mod bash;
mod edit;
mod glob;
mod grep;
mod input;
mod read;
pub mod stateful;
mod write;

pub use input::*;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::ToolCallSpec;
use crate::state::StateWriter;
use crate::tools::executor::{ExecutionContext, ToolExecutor};
use crate::tools::result::ToolExecutionResult;
use crate::tools::ToolName;

pub use bash::BashExecutor;
pub use edit::EditExecutor;
pub use glob::GlobExecutor;
pub use grep::GrepExecutor;
pub use read::ReadExecutor;
pub use stateful::{execute_exit_plan_mode, execute_todo_write};
pub use write::WriteExecutor;

/// Registry of built-in tool executors.
pub struct BuiltinExecutor {
    executors: HashMap<String, Box<dyn BuiltinToolExecutor>>,
    /// Optional state writer for TodoWrite/ExitPlanMode tools.
    state_writer: Option<Arc<parking_lot::RwLock<StateWriter>>>,
}

impl Default for BuiltinExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl BuiltinExecutor {
    /// Create a new builtin executor with default configuration.
    pub fn new() -> Self {
        let all_executors: [Box<dyn BuiltinToolExecutor>; 6] = [
            Box::new(BashExecutor::new()),
            Box::new(ReadExecutor::new()),
            Box::new(WriteExecutor::new()),
            Box::new(EditExecutor::new()),
            Box::new(GlobExecutor::new()),
            Box::new(GrepExecutor::new()),
        ];

        let executors = all_executors
            .into_iter()
            .map(|e| (e.tool_name().as_str().to_string(), e))
            .collect();

        Self {
            executors,
            state_writer: None,
        }
    }

    /// Set the state writer for stateful tools (TodoWrite, ExitPlanMode).
    pub fn with_state_writer(mut self, writer: Arc<parking_lot::RwLock<StateWriter>>) -> Self {
        self.state_writer = Some(writer);
        self
    }
}

impl ToolExecutor for BuiltinExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        // Handle stateful tools FIRST (before mock fallback) - these always write to state dir
        if let Some(ref writer) = self.state_writer {
            if let Some(tool_name) = ToolName::parse(&call.tool) {
                match tool_name {
                    ToolName::TodoWrite => {
                        let guard = writer.read();
                        let mut result = execute_todo_write(call, &guard);
                        result.tool_use_id = tool_use_id.to_string();
                        return result;
                    }
                    ToolName::ExitPlanMode => {
                        let guard = writer.read();
                        let mut result = execute_exit_plan_mode(call, &guard);
                        result.tool_use_id = tool_use_id.to_string();
                        return result;
                    }
                    _ => {}
                }
            }
        }

        // Check if we have a pre-configured result (mock fallback)
        if let Some(result) = &call.result {
            return ToolExecutionResult::success(tool_use_id, result);
        }

        // Look up the tool executor
        if let Some(executor) = self.executors.get(&call.tool) {
            let builtin_ctx = BuiltinContext {
                cwd: ctx.cwd.clone(),
            };
            executor.execute(call, tool_use_id, &builtin_ctx)
        } else {
            // Return mock result for unknown stateful tools
            let is_stateful = matches!(
                ToolName::parse(&call.tool),
                Some(ToolName::TodoWrite | ToolName::ExitPlanMode)
            );
            if is_stateful {
                // No state writer, return success with note
                ToolExecutionResult::success(
                    tool_use_id,
                    format!("{} executed (no state writer configured)", call.tool),
                )
            } else {
                ToolExecutionResult::error(
                    tool_use_id,
                    format!("Unknown built-in tool: {}", call.tool),
                )
            }
        }
    }

    fn name(&self) -> &'static str {
        "builtin"
    }
}

/// Context for built-in tool execution.
#[derive(Clone, Debug, Default)]
pub struct BuiltinContext {
    /// Working directory.
    pub cwd: Option<PathBuf>,
}

/// Trait for individual built-in tool executors.
pub trait BuiltinToolExecutor: Send + Sync {
    /// Execute the tool and return the result.
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &BuiltinContext,
    ) -> ToolExecutionResult;

    /// Get the tool name.
    fn tool_name(&self) -> ToolName;
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
