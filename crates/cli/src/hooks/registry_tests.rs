// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use crate::hooks::protocol::HookMessage;

#[test]
fn test_registry_new() {
    let registry = HookRegistry::new();
    assert!(!registry.has_hooks(&HookEvent::PreToolExecution));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_register_passthrough() {
    let mut registry = HookRegistry::new();
    registry
        .register_passthrough(HookEvent::PreToolExecution)
        .unwrap();

    assert!(registry.has_hooks(&HookEvent::PreToolExecution));

    let message = HookMessage::tool_execution(
        "test_session",
        HookEvent::PreToolExecution,
        "Bash",
        serde_json::json!({"command": "ls"}),
        None,
    );

    let responses = registry.executor().execute(&message).await.unwrap();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].proceed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_register_blocking() {
    let mut registry = HookRegistry::new();
    registry
        .register_blocking(HookEvent::PreToolExecution, "Not allowed")
        .unwrap();

    let message = HookMessage::tool_execution(
        "test_session",
        HookEvent::PreToolExecution,
        "Bash",
        serde_json::json!({"command": "rm -rf /"}),
        None,
    );

    let responses = registry.executor().execute(&message).await.unwrap();
    assert_eq!(responses.len(), 1);
    assert!(!responses[0].proceed);
    assert_eq!(responses[0].error, Some("Not allowed".to_string()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_register_echo() {
    let mut registry = HookRegistry::new();
    registry.register_echo(HookEvent::PromptSubmit).unwrap();

    let message = HookMessage::prompt_submit("test_session", "hello world");

    let responses = registry.executor().execute(&message).await.unwrap();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].proceed);
    assert!(responses[0].data.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_register_logger() {
    let temp = tempfile::tempdir().unwrap();
    let log_path = temp.path().join("hook.log");

    let mut registry = HookRegistry::new();
    registry
        .register_logger(HookEvent::SessionStart, &log_path)
        .unwrap();

    let message = HookMessage::session("test_session", HookEvent::SessionStart, None);

    let responses = registry.executor().execute(&message).await.unwrap();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].proceed);

    // Check that log file was written
    let log_content = std::fs::read_to_string(&log_path).unwrap();
    assert!(log_content.contains("session_start"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_register_inline() {
    let mut registry = HookRegistry::new();
    registry
        .register_inline(
            HookEvent::PreToolExecution,
            r#"cat > /dev/null; echo '{"proceed": true, "data": {"custom": "value"}}'"#,
            false,
        )
        .unwrap();

    let message = HookMessage::tool_execution(
        "test",
        HookEvent::PreToolExecution,
        "Test",
        serde_json::json!({}),
        None,
    );

    let responses = registry.executor().execute(&message).await.unwrap();
    assert!(responses[0].proceed);
    let data = responses[0].data.as_ref().unwrap();
    assert_eq!(data["custom"], "value");
}

#[test]
fn test_clear() {
    let mut registry = HookRegistry::new();
    registry
        .register_passthrough(HookEvent::PreToolExecution)
        .unwrap();
    registry
        .register_passthrough(HookEvent::PostToolExecution)
        .unwrap();

    assert!(registry.has_hooks(&HookEvent::PreToolExecution));
    registry.clear();
    assert!(!registry.has_hooks(&HookEvent::PreToolExecution));
    assert!(!registry.has_hooks(&HookEvent::PostToolExecution));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multiple_hooks_same_event() {
    let mut registry = HookRegistry::new();

    // Register two non-blocking hooks
    registry
        .register_passthrough(HookEvent::PreToolExecution)
        .unwrap();
    registry
        .register_passthrough(HookEvent::PreToolExecution)
        .unwrap();

    let message = HookMessage::tool_execution(
        "test",
        HookEvent::PreToolExecution,
        "Test",
        serde_json::json!({}),
        None,
    );

    let responses = registry.executor().execute(&message).await.unwrap();
    assert_eq!(responses.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_blocking_stops_processing() {
    let mut registry = HookRegistry::new();

    // First hook blocks
    registry
        .register_blocking(HookEvent::PreToolExecution, "Blocked")
        .unwrap();
    // Second hook would proceed, but should never execute
    registry
        .register_passthrough(HookEvent::PreToolExecution)
        .unwrap();

    let message = HookMessage::tool_execution(
        "test",
        HookEvent::PreToolExecution,
        "Test",
        serde_json::json!({}),
        None,
    );

    let responses = registry.executor().execute(&message).await.unwrap();
    // Only one response because blocking hook stopped processing
    assert_eq!(responses.len(), 1);
    assert!(!responses[0].proceed);
}
