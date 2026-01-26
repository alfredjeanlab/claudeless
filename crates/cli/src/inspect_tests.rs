// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::hooks::protocol::HookPayload;

#[test]
fn test_inspector_with_temp_dir() {
    let inspector = StateInspector::with_temp_dir().unwrap();
    inspector.assert_initialized();
}

#[test]
fn test_todo_assertions() {
    let inspector = StateInspector::with_temp_dir().unwrap();

    inspector.assert_todo_count(0);
    inspector.assert_pending_count(0);

    // Add todos
    {
        let mut todos = inspector.todos.lock();
        todos.add("Task 1");
        todos.add("Task 2");
        todos.set_status("todo_1", TodoStatus::Completed);
    }

    inspector.assert_todo_count(2);
    inspector.assert_pending_count(1);
    inspector.assert_completed_count(1);
    inspector.assert_todo_exists("Task 1");
    inspector.assert_todo_status("Task 2", TodoStatus::Completed);
}

#[test]
fn test_session_assertions() {
    let inspector = StateInspector::with_temp_dir().unwrap();

    inspector.assert_session_count(0);

    // Create session and add turns
    {
        let mut sessions = inspector.sessions.lock();
        let session = sessions.create_session_with_id("test");
        session.add_turn("Hello".to_string(), "Hi there!".to_string());
        session.add_turn("How are you?".to_string(), "I'm fine!".to_string());
    }

    inspector.assert_session_count(1);
    inspector.assert_turn_count(2);
    inspector.assert_last_prompt_contains("How are you");
    inspector.assert_last_response_contains("fine");
}

#[test]
fn test_hook_assertions() {
    let inspector = StateInspector::with_temp_dir().unwrap();

    inspector.assert_hook_count(&HookEvent::PreToolExecution, 0);

    // Record hooks
    inspector.record_hook(HookMessage {
        event: HookEvent::PreToolExecution,
        session_id: "test".to_string(),
        payload: HookPayload::ToolExecution {
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls"}),
            tool_output: None,
        },
    });
    inspector.record_hook(HookMessage {
        event: HookEvent::PostToolExecution,
        session_id: "test".to_string(),
        payload: HookPayload::ToolExecution {
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls"}),
            tool_output: Some("file1\nfile2".to_string()),
        },
    });

    inspector.assert_hook_invoked(&HookEvent::PreToolExecution);
    inspector.assert_hook_invoked(&HookEvent::PostToolExecution);
    inspector.assert_hook_not_invoked(&HookEvent::SessionStart);
    inspector.assert_hook_count(&HookEvent::PreToolExecution, 1);
}

#[test]
fn test_hook_invocations() {
    let inspector = StateInspector::with_temp_dir().unwrap();

    inspector.record_hook(HookMessage::tool_execution(
        "session",
        HookEvent::PreToolExecution,
        "Read",
        serde_json::json!({"file": "test.txt"}),
        None,
    ));
    inspector.record_hook(HookMessage::tool_execution(
        "session",
        HookEvent::PreToolExecution,
        "Write",
        serde_json::json!({"file": "out.txt"}),
        None,
    ));

    let invocations = inspector.hook_invocations(&HookEvent::PreToolExecution);
    assert_eq!(invocations.len(), 2);
}

#[test]
fn test_reset() {
    let inspector = StateInspector::with_temp_dir().unwrap();

    // Add some state
    inspector.todos.lock().add("Task");
    inspector.sessions.lock().create_session();
    inspector.record_hook(HookMessage::session("test", HookEvent::SessionStart, None));

    assert_eq!(inspector.todo_count(), 1);
    assert_eq!(inspector.session_count(), 1);
    assert_eq!(inspector.hook_count(), 1);

    inspector.reset();

    assert_eq!(inspector.todo_count(), 0);
    assert_eq!(inspector.session_count(), 0);
    assert_eq!(inspector.hook_count(), 0);
}

#[test]
fn test_clear_hooks() {
    let inspector = StateInspector::with_temp_dir().unwrap();

    inspector.record_hook(HookMessage::session("test", HookEvent::SessionStart, None));
    assert_eq!(inspector.hook_count(), 1);

    inspector.clear_hooks();
    assert_eq!(inspector.hook_count(), 0);
}

#[test]
fn test_state_accessors() {
    let inspector = StateInspector::with_temp_dir().unwrap();

    // Verify we can access internal state
    assert!(inspector.todos().lock().is_empty());
    assert!(inspector.sessions().lock().is_empty());
    assert!(inspector.state_dir().lock().is_initialized());
}

#[test]
fn test_directory_assertions() {
    let inspector = StateInspector::with_temp_dir().unwrap();

    assert!(inspector.is_initialized());
    assert!(inspector.state_root().exists());
}
