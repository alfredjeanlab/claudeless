// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use clap::Parser;
use std::fs;

use std::collections::HashMap;

use crate::cli::{Cli, FORCE_TUI};
use crate::config::{
    ResolvedTimeouts, ScenarioConfig, ToolCall, ToolConfig, ToolExecutionMode, ToolsConfig,
};
use crate::hooks::{HookConfig, HookEvent, HookExecutor};
use crate::scenario::Scenario;
use crate::tools::executor::MockExecutor;

use super::{Runtime, RuntimeContext};

/// Build a minimal Runtime for testing with the given hook executor and CLI.
fn build_test_runtime(hook_executor: Option<HookExecutor>, cli: Cli) -> Runtime {
    let context = RuntimeContext::build(None, &cli);
    Runtime::new(
        context,
        None, // scenario
        Box::new(MockExecutor::new()),
        None, // state
        hook_executor,
        None, // mcp_manager
        cli,
        ResolvedTimeouts::default(),
    )
}

/// Create a hook script that writes the tool name to a marker file.
fn create_hook_script(dir: &std::path::Path) -> std::path::PathBuf {
    let marker = dir.join("hook_fired");
    let script = dir.join("pre_hook.sh");
    // Parse stdin JSON to extract tool_name, append it to the marker file
    fs::write(&script, format!("#!/bin/bash\necho \"fired\" >> {}\n", marker.to_string_lossy()))
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    script
}

/// Create a blocking hook script that exits with code 2 (block).
fn create_blocking_hook_script(dir: &std::path::Path) -> std::path::PathBuf {
    let script = dir.join("block_hook.sh");
    fs::write(&script, "#!/bin/bash\necho 'blocked by test' >&2\nexit 2\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    script
}

#[tokio::test(flavor = "current_thread")]
async fn pre_tool_use_hook_fires_for_exit_plan_mode_in_tui() {
    let tmp = tempfile::tempdir().unwrap();
    let script = create_hook_script(tmp.path());
    let marker = tmp.path().join("hook_fired");

    let mut hook_executor = HookExecutor::new();
    hook_executor.register(HookEvent::PreToolExecution, HookConfig::new(&script, 5000));

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    // Force TUI mode so ExitPlanMode triggers the early return
    FORCE_TUI.set(Some(true));

    let tool_calls = vec![ToolCall {
        call: "ExitPlanMode".to_string(),
        input: serde_json::json!({}),
        result: None,
    }];

    let (results, pending) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    FORCE_TUI.set(None);

    // Hook should have fired
    assert!(marker.exists(), "PreToolUse hook should fire for ExitPlanMode in TUI mode");

    // TUI mode should set pending_permission (early return)
    assert!(pending.is_some(), "ExitPlanMode in TUI mode should return pending_permission");

    // No tool results since the tool wasn't executed (pending permission)
    assert!(results.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn pre_tool_use_hook_fires_for_ask_user_question_in_tui() {
    let tmp = tempfile::tempdir().unwrap();
    let script = create_hook_script(tmp.path());
    let marker = tmp.path().join("hook_fired");

    let mut hook_executor = HookExecutor::new();
    hook_executor.register(HookEvent::PreToolExecution, HookConfig::new(&script, 5000));

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    // Force TUI mode so AskUserQuestion triggers the early return
    FORCE_TUI.set(Some(true));

    let tool_calls = vec![ToolCall {
        call: "AskUserQuestion".to_string(),
        input: serde_json::json!({"questions": []}),
        result: None,
    }];

    let (results, pending) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    FORCE_TUI.set(None);

    // Hook should have fired
    assert!(marker.exists(), "PreToolUse hook should fire for AskUserQuestion in TUI mode");

    // TUI mode should set pending_permission (early return)
    assert!(pending.is_some(), "AskUserQuestion in TUI mode should return pending_permission");

    // No tool results since the tool wasn't executed (pending permission)
    assert!(results.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn pre_tool_use_hook_blocking_prevents_tui_pending_permission() {
    let tmp = tempfile::tempdir().unwrap();
    let script = create_blocking_hook_script(tmp.path());

    let mut hook_executor = HookExecutor::new();
    hook_executor
        .register(HookEvent::PreToolExecution, HookConfig::new(&script, 5000).with_blocking(true));

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    // Force TUI mode
    FORCE_TUI.set(Some(true));

    let tool_calls = vec![ToolCall {
        call: "ExitPlanMode".to_string(),
        input: serde_json::json!({}),
        result: None,
    }];

    let (results, pending) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    FORCE_TUI.set(None);

    // Blocking hook should prevent the TUI early return — tool gets error result instead
    assert!(pending.is_none(), "Blocking hook should prevent pending_permission");
    assert_eq!(results.len(), 1);
    assert!(results[0].is_error);
}

#[tokio::test(flavor = "current_thread")]
async fn pre_tool_use_hook_fires_for_regular_tools() {
    let tmp = tempfile::tempdir().unwrap();
    let script = create_hook_script(tmp.path());
    let marker = tmp.path().join("hook_fired");

    let mut hook_executor = HookExecutor::new();
    hook_executor.register(HookEvent::PreToolExecution, HookConfig::new(&script, 5000));

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    let tool_calls = vec![ToolCall {
        call: "Read".to_string(),
        input: serde_json::json!({"file_path": "/dev/null"}),
        result: Some("file content".to_string()),
    }];

    let (results, pending) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    // Hook should have fired
    assert!(marker.exists(), "PreToolUse hook should fire for regular tools");

    // Regular tools execute normally
    assert!(pending.is_none());
    assert_eq!(results.len(), 1);
    assert!(!results[0].is_error);
}

#[tokio::test(flavor = "current_thread")]
async fn post_tool_use_hook_fires_on_success() {
    let tmp = tempfile::tempdir().unwrap();
    let marker = tmp.path().join("post_hook_fired");
    let script = tmp.path().join("post_hook.sh");
    std::fs::write(
        &script,
        format!("#!/bin/bash\necho \"fired\" >> {}\n", marker.to_string_lossy()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut hook_executor = HookExecutor::new();
    hook_executor.register(HookEvent::PostToolExecution, HookConfig::new(&script, 5000));

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    let tool_calls = vec![ToolCall {
        call: "Read".to_string(),
        input: serde_json::json!({"file_path": "/dev/null"}),
        result: Some("file content".to_string()),
    }];

    let (results, _) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    // PostToolUse should fire for successful tool execution
    assert!(marker.exists(), "PostToolUse hook should fire for successful tools");
    assert_eq!(results.len(), 1);
    assert!(!results[0].is_error);
}

#[tokio::test(flavor = "current_thread")]
async fn post_tool_use_hook_does_not_fire_on_error() {
    let tmp = tempfile::tempdir().unwrap();
    let marker = tmp.path().join("post_hook_fired");
    let script = tmp.path().join("post_hook.sh");
    std::fs::write(
        &script,
        format!("#!/bin/bash\necho \"fired\" >> {}\n", marker.to_string_lossy()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut hook_executor = HookExecutor::new();
    hook_executor.register(HookEvent::PostToolExecution, HookConfig::new(&script, 5000));

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    // Tool with no result configured => MockExecutor returns error
    let tool_calls = vec![ToolCall {
        call: "Read".to_string(),
        input: serde_json::json!({"file_path": "/dev/null"}),
        result: None,
    }];

    let (results, _) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    // PostToolUse should NOT fire for error tool execution
    assert!(!marker.exists(), "PostToolUse hook should not fire for error tools");
    assert_eq!(results.len(), 1);
    assert!(results[0].is_error);
}

#[tokio::test(flavor = "current_thread")]
async fn post_tool_use_failure_hook_fires_on_error() {
    let tmp = tempfile::tempdir().unwrap();
    let marker = tmp.path().join("failure_hook_fired");
    let script = tmp.path().join("failure_hook.sh");
    std::fs::write(
        &script,
        format!("#!/bin/bash\necho \"fired\" >> {}\n", marker.to_string_lossy()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut hook_executor = HookExecutor::new();
    hook_executor.register(HookEvent::PostToolExecutionFailure, HookConfig::new(&script, 5000));

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    // Tool with no result configured => MockExecutor returns error
    let tool_calls = vec![ToolCall {
        call: "Read".to_string(),
        input: serde_json::json!({"file_path": "/dev/null"}),
        result: None,
    }];

    let (results, _) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    // PostToolUseFailure should fire for error tool execution
    assert!(marker.exists(), "PostToolUseFailure hook should fire for error tools");
    assert_eq!(results.len(), 1);
    assert!(results[0].is_error);
}

#[tokio::test(flavor = "current_thread")]
async fn post_tool_use_failure_hook_does_not_fire_on_success() {
    let tmp = tempfile::tempdir().unwrap();
    let marker = tmp.path().join("failure_hook_fired");
    let script = tmp.path().join("failure_hook.sh");
    std::fs::write(
        &script,
        format!("#!/bin/bash\necho \"fired\" >> {}\n", marker.to_string_lossy()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut hook_executor = HookExecutor::new();
    hook_executor.register(HookEvent::PostToolExecutionFailure, HookConfig::new(&script, 5000));

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    let tool_calls = vec![ToolCall {
        call: "Read".to_string(),
        input: serde_json::json!({"file_path": "/dev/null"}),
        result: Some("file content".to_string()),
    }];

    let (results, _) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    // PostToolUseFailure should NOT fire for successful tool execution
    assert!(!marker.exists(), "PostToolUseFailure hook should not fire for successful tools");
    assert_eq!(results.len(), 1);
    assert!(!results[0].is_error);
}

/// Build a Runtime with a scenario that has per-tool configs (canned results/errors).
fn build_test_runtime_with_scenario(
    tool_configs: HashMap<String, ToolConfig>,
    hook_executor: Option<HookExecutor>,
    cli: Cli,
) -> Runtime {
    let config = ScenarioConfig {
        tools: Some(ToolsConfig { mode: ToolExecutionMode::Mock, tools: tool_configs }),
        ..Default::default()
    };
    let scenario = Scenario::from_config(config).unwrap();
    let context = RuntimeContext::build(None, &cli);
    Runtime::new(
        context,
        Some(scenario),
        Box::new(MockExecutor::new()),
        None,
        hook_executor,
        None,
        cli,
        ResolvedTimeouts::default(),
    )
}

#[tokio::test(flavor = "current_thread")]
async fn scenario_canned_result_injected_into_tool_call() {
    let mut tools = HashMap::new();
    tools.insert(
        "Read".to_string(),
        ToolConfig { result: Some("canned content".to_string()), ..Default::default() },
    );

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime_with_scenario(tools, None, cli);

    // Tool call has no inline result — should get canned result from scenario
    let tool_calls = vec![ToolCall {
        call: "Read".to_string(),
        input: serde_json::json!({"file_path": "/nonexistent"}),
        result: None,
    }];

    let (results, _) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    assert_eq!(results.len(), 1);
    assert!(!results[0].is_error, "canned result should produce success");
    assert_eq!(results[0].text(), Some("canned content"));
}

#[tokio::test(flavor = "current_thread")]
async fn scenario_canned_error_injected_into_tool_call() {
    let mut tools = HashMap::new();
    tools.insert(
        "Write".to_string(),
        ToolConfig { error: Some("Permission denied".to_string()), ..Default::default() },
    );

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime_with_scenario(tools, None, cli);

    let tool_calls = vec![ToolCall {
        call: "Write".to_string(),
        input: serde_json::json!({"file_path": "/tmp/out", "content": "hi"}),
        result: None,
    }];

    let (results, _) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    assert_eq!(results.len(), 1);
    // MockExecutor sees the injected result string "Error: Permission denied"
    // and returns it as success (the error semantics are for the simulated tool)
    assert!(!results[0].is_error);
    assert_eq!(results[0].text(), Some("Error: Permission denied"));
}

#[tokio::test(flavor = "current_thread")]
async fn inline_result_takes_precedence_over_scenario_config() {
    let mut tools = HashMap::new();
    tools.insert(
        "Read".to_string(),
        ToolConfig { result: Some("scenario canned".to_string()), ..Default::default() },
    );

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime_with_scenario(tools, None, cli);

    // Tool call has an inline result — should NOT be overridden
    let tool_calls = vec![ToolCall {
        call: "Read".to_string(),
        input: serde_json::json!({"file_path": "/dev/null"}),
        result: Some("inline result".to_string()),
    }];

    let (results, _) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    assert_eq!(results.len(), 1);
    assert!(!results[0].is_error);
    assert_eq!(results[0].text(), Some("inline result"));
}

#[tokio::test(flavor = "current_thread")]
async fn scenario_canned_result_fires_post_tool_use_hook() {
    let tmp = tempfile::tempdir().unwrap();
    let marker = tmp.path().join("post_hook_fired");
    let script = tmp.path().join("post_hook.sh");
    fs::write(&script, format!("#!/bin/bash\necho \"fired\" >> {}\n", marker.to_string_lossy()))
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut hook_executor = HookExecutor::new();
    hook_executor.register(HookEvent::PostToolExecution, HookConfig::new(&script, 5000));

    let mut tools = HashMap::new();
    tools.insert(
        "Read".to_string(),
        ToolConfig { result: Some("canned content".to_string()), ..Default::default() },
    );

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime_with_scenario(tools, Some(hook_executor), cli);

    let tool_calls = vec![ToolCall {
        call: "Read".to_string(),
        input: serde_json::json!({"file_path": "/nonexistent"}),
        result: None,
    }];

    let (results, _) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    assert!(
        marker.exists(),
        "PostToolUse should fire when canned result is injected from scenario"
    );
    assert_eq!(results.len(), 1);
    assert!(!results[0].is_error);
}

#[tokio::test(flavor = "current_thread")]
async fn no_scenario_config_leaves_tool_call_unchanged() {
    // Runtime without scenario — tool call with no result stays as-is
    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(None, cli);

    let tool_calls = vec![ToolCall {
        call: "Read".to_string(),
        input: serde_json::json!({"file_path": "/dev/null"}),
        result: None,
    }];

    let (results, _) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    // MockExecutor returns error for tool calls with no result
    assert_eq!(results.len(), 1);
    assert!(results[0].is_error);
}

#[tokio::test(flavor = "current_thread")]
async fn fire_session_end_hook_fires() {
    let tmp = tempfile::tempdir().unwrap();
    let marker = tmp.path().join("session_end_fired");
    let script = tmp.path().join("session_end_hook.sh");
    fs::write(&script, format!("#!/bin/bash\necho \"fired\" >> {}\n", marker.to_string_lossy()))
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut hook_executor = HookExecutor::new();
    hook_executor.register(HookEvent::SessionEnd, HookConfig::new(&script, 5000));

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let runtime = build_test_runtime(Some(hook_executor), cli);

    runtime.fire_session_end_hook("prompt_input_exit").await;

    assert!(marker.exists(), "SessionEnd hook should fire");
}

#[tokio::test(flavor = "current_thread")]
async fn fire_prompt_submit_hook_fires() {
    let tmp = tempfile::tempdir().unwrap();
    let marker = tmp.path().join("prompt_submit_fired");
    let script = tmp.path().join("prompt_submit_hook.sh");
    fs::write(&script, format!("#!/bin/bash\necho \"fired\" >> {}\n", marker.to_string_lossy()))
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut hook_executor = HookExecutor::new();
    hook_executor.register(HookEvent::PromptSubmit, HookConfig::new(&script, 5000));

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let runtime = build_test_runtime(Some(hook_executor), cli);

    runtime.fire_prompt_submit_hook("Hello, Claude!").await;

    assert!(marker.exists(), "UserPromptSubmit hook should fire");
}

#[tokio::test(flavor = "current_thread")]
async fn ask_user_question_with_scenario_answers_in_tui_executes_directly() {
    let mut tools = HashMap::new();
    tools.insert(
        "AskUserQuestion".to_string(),
        ToolConfig {
            approve: true,
            answers: Some({
                let mut m = HashMap::new();
                m.insert("Which DB?".to_string(), "PostgreSQL".to_string());
                m
            }),
            ..Default::default()
        },
    );

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime_with_scenario(tools, None, cli);

    // Force TUI mode — scenario answers should still be used instead of pending_permission
    FORCE_TUI.set(Some(true));

    let tool_calls = vec![ToolCall {
        call: "AskUserQuestion".to_string(),
        input: serde_json::json!({
            "questions": [{
                "question": "Which DB?",
                "header": "DB",
                "options": [
                    { "label": "PostgreSQL", "description": "Relational" },
                    { "label": "SQLite", "description": "Embedded" }
                ],
                "multiSelect": false
            }]
        }),
        result: None,
    }];

    let (results, pending) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    FORCE_TUI.set(None);

    // Scenario answers should be injected — no pending permission needed
    assert!(
        pending.is_none(),
        "AskUserQuestion with scenario answers in TUI should execute directly, not pend"
    );

    // Tool should have executed (MockExecutor ran, even if it returns error for unknown tools)
    assert_eq!(results.len(), 1, "Tool should have been executed, not returned as pending");
}

#[tokio::test(flavor = "current_thread")]
async fn ask_user_question_without_scenario_answers_in_tui_returns_pending() {
    // No scenario answers configured — TUI mode should return pending_permission
    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(None, cli);

    FORCE_TUI.set(Some(true));

    let tool_calls = vec![ToolCall {
        call: "AskUserQuestion".to_string(),
        input: serde_json::json!({
            "questions": [{
                "question": "Which DB?",
                "header": "DB",
                "options": [
                    { "label": "PostgreSQL", "description": "Relational" },
                    { "label": "SQLite", "description": "Embedded" }
                ],
                "multiSelect": false
            }]
        }),
        result: None,
    }];

    let (results, pending) = runtime.execute_tools_for_turn("test", "", &tool_calls).await;

    FORCE_TUI.set(None);

    // No scenario answers → TUI mode should set pending_permission for interactive dialog
    assert!(
        pending.is_some(),
        "AskUserQuestion without scenario answers in TUI should return pending_permission"
    );
    assert!(results.is_empty());
}
