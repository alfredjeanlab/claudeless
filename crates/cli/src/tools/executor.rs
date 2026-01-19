// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool execution engine trait and implementations.

use std::path::PathBuf;

use crate::config::{ToolCallSpec, ToolExecutionConfig, ToolExecutionMode};
use crate::permission::{PermissionChecker, PermissionResult};

use super::result::ToolExecutionResult;

/// Context for tool execution.
#[derive(Clone, Debug, Default)]
pub struct ExecutionContext {
    /// Working directory for tool execution.
    pub cwd: Option<PathBuf>,

    /// Sandbox root directory (for simulated mode).
    pub sandbox_root: Option<PathBuf>,

    /// Whether real bash execution is allowed.
    pub allow_real_bash: bool,

    /// Session ID for tracking.
    pub session_id: Option<String>,
}

impl ExecutionContext {
    /// Create context from tool execution config.
    pub fn from_config(config: &ToolExecutionConfig) -> Self {
        Self {
            cwd: None,
            sandbox_root: config.sandbox_root.as_ref().map(PathBuf::from),
            allow_real_bash: config.allow_real_bash,
            session_id: None,
        }
    }

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
        ToolExecutionMode::Simulated => Box::new(super::builtin::BuiltinExecutor::new()),
        ToolExecutionMode::RealMcp => Box::new(super::mcp::McpExecutor::new()),
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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mock_executor_with_result() {
        let executor = MockExecutor::new();
        let call = ToolCallSpec {
            tool: "Bash".to_string(),
            input: json!({ "command": "ls" }),
            result: Some("file1.txt\nfile2.txt".to_string()),
        };
        let ctx = ExecutionContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(!result.is_error);
        assert_eq!(result.tool_use_id, "toolu_123");
        assert_eq!(result.text(), Some("file1.txt\nfile2.txt"));
    }

    #[test]
    fn test_mock_executor_without_result() {
        let executor = MockExecutor::new();
        let call = ToolCallSpec {
            tool: "Read".to_string(),
            input: json!({ "path": "/tmp/test.txt" }),
            result: None,
        };
        let ctx = ExecutionContext::default();
        let result = executor.execute(&call, "toolu_456", &ctx);

        assert!(result.is_error);
        assert!(result.text().unwrap().contains("No mock result"));
        assert!(result.text().unwrap().contains("Read"));
    }

    #[test]
    fn test_disabled_executor() {
        let executor = DisabledExecutor::new();
        let call = ToolCallSpec {
            tool: "Bash".to_string(),
            input: json!({ "command": "ls" }),
            result: Some("output".to_string()),
        };
        let ctx = ExecutionContext::default();
        let result = executor.execute(&call, "toolu_789", &ctx);

        assert!(result.is_error);
        assert!(result.text().unwrap().contains("disabled"));
    }

    #[test]
    fn test_execution_context_from_config() {
        let config = ToolExecutionConfig {
            mode: ToolExecutionMode::Simulated,
            sandbox_root: Some("/tmp/sandbox".to_string()),
            allow_real_bash: true,
            ..Default::default()
        };
        let ctx = ExecutionContext::from_config(&config);

        assert_eq!(ctx.sandbox_root, Some(PathBuf::from("/tmp/sandbox")));
        assert!(ctx.allow_real_bash);
    }

    #[test]
    fn test_execution_context_builder() {
        let ctx = ExecutionContext::default()
            .with_cwd("/home/user")
            .with_session_id("session-123");

        assert_eq!(ctx.cwd, Some(PathBuf::from("/home/user")));
        assert_eq!(ctx.session_id, Some("session-123".to_string()));
    }

    #[test]
    fn test_create_executor_disabled() {
        let executor = create_executor(ToolExecutionMode::Disabled);
        assert_eq!(executor.name(), "disabled");
    }

    #[test]
    fn test_create_executor_mock() {
        let executor = create_executor(ToolExecutionMode::Mock);
        assert_eq!(executor.name(), "mock");
    }

    #[test]
    fn test_executor_name() {
        assert_eq!(MockExecutor::new().name(), "mock");
        assert_eq!(DisabledExecutor::new().name(), "disabled");
    }

    #[test]
    fn test_permission_checking_executor_allowed() {
        use crate::permission::{PermissionBypass, PermissionMode};

        let inner = Box::new(MockExecutor::new());
        let checker = PermissionChecker::new(
            PermissionMode::BypassPermissions,
            PermissionBypass::default(),
        );
        let executor = PermissionCheckingExecutor::new(inner, checker);

        let call = ToolCallSpec {
            tool: "Bash".to_string(),
            input: json!({ "command": "ls" }),
            result: Some("output".to_string()),
        };
        let ctx = ExecutionContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(!result.is_error);
        assert_eq!(result.text(), Some("output"));
    }

    #[test]
    fn test_permission_checking_executor_denied() {
        use crate::permission::{PermissionBypass, PermissionMode};

        let inner = Box::new(MockExecutor::new());
        let checker = PermissionChecker::new(PermissionMode::DontAsk, PermissionBypass::default());
        let executor = PermissionCheckingExecutor::new(inner, checker);

        let call = ToolCallSpec {
            tool: "Bash".to_string(),
            input: json!({ "command": "rm -rf /" }),
            result: Some("never executed".to_string()),
        };
        let ctx = ExecutionContext::default();
        let result = executor.execute(&call, "toolu_456", &ctx);

        assert!(result.is_error);
        assert!(result.text().unwrap().contains("Permission denied"));
    }

    #[test]
    fn test_permission_checking_executor_needs_prompt() {
        use crate::permission::{PermissionBypass, PermissionMode};

        let inner = Box::new(MockExecutor::new());
        let checker = PermissionChecker::new(PermissionMode::Default, PermissionBypass::default());
        let executor = PermissionCheckingExecutor::new(inner, checker);

        let call = ToolCallSpec {
            tool: "Bash".to_string(),
            input: json!({ "command": "ls" }),
            result: Some("never executed".to_string()),
        };
        let ctx = ExecutionContext::default();
        let result = executor.execute(&call, "toolu_789", &ctx);

        // NeedsPrompt is treated as denied in the simulator
        assert!(result.is_error);
        assert!(result.text().unwrap().contains("requires permission"));
    }

    #[test]
    fn test_create_executor_with_permissions() {
        use crate::permission::{PermissionBypass, PermissionMode};

        let checker = PermissionChecker::new(
            PermissionMode::BypassPermissions,
            PermissionBypass::default(),
        );
        let executor = create_executor_with_permissions(ToolExecutionMode::Mock, checker);
        assert_eq!(executor.name(), "permission_checking");
    }
}
