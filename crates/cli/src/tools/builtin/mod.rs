// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Built-in tool executors for simulated mode.
//!
//! This module provides sandboxed executors for Claude's built-in tools:
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
mod read;
pub mod stateful;
mod write;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::ToolCallSpec;
use crate::state::StateWriter;
use crate::tools::executor::{ExecutionContext, ToolExecutor};
use crate::tools::result::ToolExecutionResult;

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
    sandbox_root: Option<PathBuf>,
    allow_real_bash: bool,
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
        let mut executors: HashMap<String, Box<dyn BuiltinToolExecutor>> = HashMap::new();

        // Register all built-in tool executors
        executors.insert("Bash".to_string(), Box::new(BashExecutor::new()));
        executors.insert("Read".to_string(), Box::new(ReadExecutor::new()));
        executors.insert("Write".to_string(), Box::new(WriteExecutor::new()));
        executors.insert("Edit".to_string(), Box::new(EditExecutor::new()));
        executors.insert("Glob".to_string(), Box::new(GlobExecutor::new()));
        executors.insert("Grep".to_string(), Box::new(GrepExecutor::new()));

        Self {
            executors,
            sandbox_root: None,
            allow_real_bash: false,
            state_writer: None,
        }
    }

    /// Set the sandbox root directory.
    pub fn with_sandbox_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.sandbox_root = Some(root.into());
        self
    }

    /// Enable real bash execution.
    pub fn with_real_bash(mut self, allow: bool) -> Self {
        self.allow_real_bash = allow;
        self
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
            match call.tool.as_str() {
                "TodoWrite" => {
                    let guard = writer.read();
                    let mut result = execute_todo_write(call, &guard);
                    result.tool_use_id = tool_use_id.to_string();
                    return result;
                }
                "ExitPlanMode" => {
                    let guard = writer.read();
                    let mut result = execute_exit_plan_mode(call, &guard);
                    result.tool_use_id = tool_use_id.to_string();
                    return result;
                }
                _ => {}
            }
        }

        // Check if we have a pre-configured result (mock fallback)
        if let Some(result) = &call.result {
            return ToolExecutionResult::success(tool_use_id, result);
        }

        // Look up the tool executor
        if let Some(executor) = self.executors.get(&call.tool) {
            let sandbox_ctx = BuiltinContext {
                sandbox_root: self
                    .sandbox_root
                    .clone()
                    .or_else(|| ctx.sandbox_root.clone()),
                allow_real_bash: self.allow_real_bash || ctx.allow_real_bash,
                cwd: ctx.cwd.clone(),
            };
            executor.execute(call, tool_use_id, &sandbox_ctx)
        } else {
            // Return mock result for unknown stateful tools
            if call.tool == "TodoWrite" || call.tool == "ExitPlanMode" {
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
    /// Sandbox root directory for path resolution.
    pub sandbox_root: Option<PathBuf>,
    /// Whether real bash execution is allowed.
    pub allow_real_bash: bool,
    /// Working directory.
    pub cwd: Option<PathBuf>,
}

impl BuiltinContext {
    /// Resolve a path within the sandbox, preventing traversal.
    pub fn resolve_path(&self, path: &str) -> PathBuf {
        if let Some(ref root) = self.sandbox_root {
            // Normalize and prevent path traversal
            // Filter out ParentDir (..) and RootDir (/) to keep paths within sandbox
            let path = PathBuf::from(path);
            let normalized: PathBuf = path
                .components()
                .filter(|c| {
                    !matches!(
                        c,
                        std::path::Component::ParentDir | std::path::Component::RootDir
                    )
                })
                .collect();
            root.join(normalized)
        } else {
            PathBuf::from(path)
        }
    }
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
    fn tool_name(&self) -> &'static str;
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
