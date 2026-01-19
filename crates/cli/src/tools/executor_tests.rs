#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
