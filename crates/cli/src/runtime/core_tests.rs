// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use clap::Parser;
use std::fs;

use crate::cli::{Cli, FORCE_TUI};
use crate::config::{ResolvedTimeouts, ToolCallSpec};
use crate::hooks::{HookConfig, HookEvent, HookExecutor};
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
    fs::write(
        &script,
        format!(
            "#!/bin/bash\necho \"fired\" >> {}\n",
            marker.to_string_lossy()
        ),
    )
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
    hook_executor.register(
        HookEvent::PreToolExecution,
        HookConfig::new(&script, 5000),
    );

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    // Force TUI mode so ExitPlanMode triggers the early return
    FORCE_TUI.set(Some(true));

    let tool_calls = vec![ToolCallSpec {
        tool: "ExitPlanMode".to_string(),
        input: serde_json::json!({}),
        result: None,
    }];

    let (results, pending) = runtime
        .execute_tools_for_turn("test", "", &tool_calls)
        .await;

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
    hook_executor.register(
        HookEvent::PreToolExecution,
        HookConfig::new(&script, 5000),
    );

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    // Force TUI mode so AskUserQuestion triggers the early return
    FORCE_TUI.set(Some(true));

    let tool_calls = vec![ToolCallSpec {
        tool: "AskUserQuestion".to_string(),
        input: serde_json::json!({"questions": []}),
        result: None,
    }];

    let (results, pending) = runtime
        .execute_tools_for_turn("test", "", &tool_calls)
        .await;

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
    hook_executor.register(
        HookEvent::PreToolExecution,
        HookConfig::new(&script, 5000).with_blocking(true),
    );

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    // Force TUI mode
    FORCE_TUI.set(Some(true));

    let tool_calls = vec![ToolCallSpec {
        tool: "ExitPlanMode".to_string(),
        input: serde_json::json!({}),
        result: None,
    }];

    let (results, pending) = runtime
        .execute_tools_for_turn("test", "", &tool_calls)
        .await;

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
    hook_executor.register(
        HookEvent::PreToolExecution,
        HookConfig::new(&script, 5000),
    );

    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let mut runtime = build_test_runtime(Some(hook_executor), cli);

    let tool_calls = vec![ToolCallSpec {
        tool: "Read".to_string(),
        input: serde_json::json!({"file_path": "/dev/null"}),
        result: Some("file content".to_string()),
    }];

    let (results, pending) = runtime
        .execute_tools_for_turn("test", "", &tool_calls)
        .await;

    // Hook should have fired
    assert!(marker.exists(), "PreToolUse hook should fire for regular tools");

    // Regular tools execute normally
    assert!(pending.is_none());
    assert_eq!(results.len(), 1);
    assert!(!results[0].is_error);
}
