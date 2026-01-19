// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool execution engine trait and implementations.

use std::path::PathBuf;

use crate::config::{ToolCallSpec, ToolExecutionMode};
use crate::permission::{PermissionChecker, PermissionResult};

use super::result::ToolExecutionResult;

/// Context for tool execution.
#[derive(Clone, Debug, Default)]
pub struct ExecutionContext {
    /// Working directory for tool execution.
    pub cwd: Option<PathBuf>,

    /// Session ID for tracking.
    pub session_id: Option<String>,
}

impl ExecutionContext {
    /// Set the working directory.
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set the session ID.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// Trait for tool execution engines.
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool call and return the result.
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &ExecutionContext,
    ) -> ToolExecutionResult;

    /// Get the name of this executor for debugging.
    fn name(&self) -> &'static str;
}

/// Mock executor that returns pre-configured results from the tool call spec.
#[derive(Clone, Debug, Default)]
pub struct MockExecutor;

impl MockExecutor {
    /// Create a new mock executor.
    pub fn new() -> Self {
        Self
    }
}

impl ToolExecutor for MockExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        _ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        match &call.result {
            Some(result) => ToolExecutionResult::success(tool_use_id, result),
            None => ToolExecutionResult::no_mock_result(tool_use_id, &call.tool),
        }
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}

/// Disabled executor that always returns an error.
#[derive(Clone, Debug, Default)]
pub struct DisabledExecutor;

impl DisabledExecutor {
    /// Create a new disabled executor.
    pub fn new() -> Self {
        Self
    }
}

impl ToolExecutor for DisabledExecutor {
    fn execute(
        &self,
        _call: &ToolCallSpec,
        tool_use_id: &str,
        _ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        ToolExecutionResult::disabled(tool_use_id)
    }

    fn name(&self) -> &'static str {
        "disabled"
    }
}

/// Executor that checks permissions before delegating to an inner executor.
pub struct PermissionCheckingExecutor {
    /// Inner executor to delegate to.
    inner: Box<dyn ToolExecutor>,
    /// Permission checker.
    checker: PermissionChecker,
}

impl PermissionCheckingExecutor {
    /// Create a new permission-checking executor.
    pub fn new(inner: Box<dyn ToolExecutor>, checker: PermissionChecker) -> Self {
        Self { inner, checker }
    }

    /// Get the action type for a tool.
    fn get_action(&self, tool_name: &str) -> &'static str {
        match tool_name {
            "Bash" => "execute",
            "Read" => "read",
            "Write" | "Edit" | "NotebookEdit" => "write",
            "Glob" | "Grep" => "read",
            "WebFetch" | "WebSearch" => "network",
            "Task" => "delegate",
            _ => "execute", // Default for MCP tools
        }
    }
}

impl ToolExecutor for PermissionCheckingExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        let action = self.get_action(&call.tool);
        match self.checker.check(&call.tool, action) {
            PermissionResult::Allowed => self.inner.execute(call, tool_use_id, ctx),
            PermissionResult::Denied { reason } => {
                ToolExecutionResult::permission_denied(tool_use_id, reason)
            }
            PermissionResult::NeedsPrompt { tool, action } => {
                // In the simulator, NeedsPrompt is treated as denied since
                // we don't have interactive prompting
                ToolExecutionResult::permission_denied(
                    tool_use_id,
                    format!(
                        "Tool '{}' requires permission for '{}' action",
                        tool, action
                    ),
                )
            }
        }
    }

    fn name(&self) -> &'static str {
        "permission_checking"
    }
}

/// Create an executor based on the execution mode.
pub fn create_executor(mode: ToolExecutionMode) -> Box<dyn ToolExecutor> {
    match mode {
        ToolExecutionMode::Disabled => Box::new(DisabledExecutor::new()),
        ToolExecutionMode::Mock => Box::new(MockExecutor::new()),
        ToolExecutionMode::Live => Box::new(super::builtin::BuiltinExecutor::new()),
    }
}

/// Create an executor with permission checking.
pub fn create_executor_with_permissions(
    mode: ToolExecutionMode,
    checker: PermissionChecker,
) -> Box<dyn ToolExecutor> {
    let inner = create_executor(mode);
    Box::new(PermissionCheckingExecutor::new(inner, checker))
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;
