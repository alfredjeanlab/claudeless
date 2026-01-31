// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use crate::state::todos::ClaudeTodoItem;

#[test]
fn test_state_writer_creation() {
    let writer = StateWriter::new(
        "test-session-id",
        "/tmp/test-project",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/test-project",
    )
    .unwrap();

    assert_eq!(writer.session_id, "test-session-id");
    assert!(writer.state_dir().is_initialized());
}

#[test]
fn test_state_writer_paths() {
    let writer = StateWriter::new(
        "abc123",
        "/tmp/test",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/test",
    )
    .unwrap();

    // Todo path should be {sessionId}-agent-{sessionId}.json
    let todo_path = writer.todo_path();
    assert!(todo_path
        .to_string_lossy()
        .contains("abc123-agent-abc123.json"));

    // Session JSONL path should end with {sessionId}.jsonl
    let jsonl_path = writer.session_jsonl_path();
    assert!(jsonl_path.to_string_lossy().ends_with("abc123.jsonl"));
}

#[test]
fn test_state_writer_record_turn() {
    let mut writer = StateWriter::new(
        Uuid::new_v4().to_string(),
        "/tmp/test-project",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/test-project",
    )
    .unwrap();

    writer.record_turn("Hello", "Hi there!").unwrap();

    // Check that JSONL file exists
    assert!(writer.session_jsonl_path().exists());

    // Check that sessions-index.json exists
    let index_path = writer.project_dir().join("sessions-index.json");
    assert!(index_path.exists());

    // Verify sessions-index content
    let index = SessionsIndex::load(&index_path).unwrap();
    assert_eq!(index.len(), 1);
    let entry = index.get(&writer.session_id).unwrap();
    assert_eq!(entry.first_prompt, "Hello");
    assert_eq!(entry.message_count, 2);
}

#[test]
fn test_state_writer_write_todos() {
    let writer = StateWriter::new(
        "session-123",
        "/tmp/test",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/test",
    )
    .unwrap();

    let items = vec![
        TodoItem {
            id: "todo_0".to_string(),
            content: "Build project".to_string(),
            active_form: Some("Building project".to_string()),
            status: TodoStatus::Pending,
            priority: 0,
        },
        TodoItem {
            id: "todo_1".to_string(),
            content: "Run tests".to_string(),
            active_form: Some("Running tests".to_string()),
            status: TodoStatus::InProgress,
            priority: 1,
        },
    ];

    writer.write_todos(&items).unwrap();

    // Verify file exists and content is correct
    let todo_path = writer.todo_path();
    assert!(todo_path.exists());

    let content = std::fs::read_to_string(&todo_path).unwrap();
    let parsed: Vec<ClaudeTodoItem> = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].content, "Build project");
    assert_eq!(parsed[0].status, TodoStatus::Pending);
    assert_eq!(parsed[0].active_form, "Building project");
    assert_eq!(parsed[1].status, TodoStatus::InProgress);
}

#[test]
fn test_state_writer_create_plan() {
    let writer = StateWriter::new(
        "session-123",
        "/tmp/test",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/test",
    )
    .unwrap();

    let content = "# Test Plan\n\n## Overview\n\nThis is a test plan.";
    let name = writer.create_plan(content).unwrap();

    // Verify name format (3 parts separated by hyphens)
    let parts: Vec<&str> = name.split('-').collect();
    assert_eq!(parts.len(), 3);

    // Verify file exists
    let plan_path = writer.state_dir().plans_dir().join(format!("{}.md", name));
    assert!(plan_path.exists());

    // Verify content
    let saved_content = std::fs::read_to_string(&plan_path).unwrap();
    assert_eq!(saved_content, content);
}

#[test]
fn test_state_writer_record_assistant_response_final_sets_stop_reason() {
    let mut writer = StateWriter::new(
        Uuid::new_v4().to_string(),
        "/tmp/test-project",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/test-project",
    )
    .unwrap();

    let user_uuid = writer.record_user_message("Hello").unwrap();
    writer
        .record_assistant_response_final(&user_uuid, "Done.")
        .unwrap();

    // Read JSONL and verify stop_reason
    let content = std::fs::read_to_string(writer.session_jsonl_path()).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Second line should be assistant with stop_reason: end_turn
    assert!(lines[1].contains("\"stop_reason\":\"end_turn\""));
}

#[test]
fn test_state_writer_record_assistant_tool_use_sets_stop_reason() {
    let mut writer = StateWriter::new(
        Uuid::new_v4().to_string(),
        "/tmp/test-project",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/test-project",
    )
    .unwrap();

    let user_uuid = writer.record_user_message("Run a command").unwrap();
    writer
        .record_assistant_tool_use(
            &user_uuid,
            vec![ContentBlock::ToolUse {
                id: "toolu_123".to_string(),
                name: "Bash".to_string(),
                input: serde_json::json!({"command": "ls"}),
            }],
        )
        .unwrap();

    // Read JSONL and verify stop_reason
    let content = std::fs::read_to_string(writer.session_jsonl_path()).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Second line should be assistant with stop_reason: tool_use
    assert!(lines[1].contains("\"stop_reason\":\"tool_use\""));
}

#[test]
fn test_state_writer_record_tool_result_writes_dual_records() {
    let mut writer = StateWriter::new(
        Uuid::new_v4().to_string(),
        "/tmp/test-project",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/test-project",
    )
    .unwrap();

    let user_uuid = writer.record_user_message("Run a command").unwrap();
    let assistant_uuid = writer
        .record_assistant_tool_use(
            &user_uuid,
            vec![ContentBlock::ToolUse {
                id: "toolu_123".to_string(),
                name: "Bash".to_string(),
                input: serde_json::json!({"command": "echo hello"}),
            }],
        )
        .unwrap();

    writer
        .record_tool_result(
            "toolu_123",
            "hello\n\nExit code: 0",
            &assistant_uuid,
            serde_json::json!({}),
        )
        .unwrap();

    // Read JSONL and verify both user and result records
    let content = std::fs::read_to_string(writer.session_jsonl_path()).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Should have 4 lines: user, assistant, user (tool_result), result
    assert_eq!(lines.len(), 4);

    // Third line should be user with tool_result content
    assert!(lines[2].contains("\"type\":\"user\""));
    assert!(lines[2].contains("tool_result"));

    // Fourth line should be result record
    assert!(lines[3].contains("\"type\":\"result\""));
    assert!(lines[3].contains("toolu_123"));
    assert!(lines[3].contains("Exit code: 0"));
}

#[test]
fn test_state_writer_record_error() {
    let writer = StateWriter::new(
        "error-test-session",
        "/tmp/error-test",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/error-test",
    )
    .unwrap();

    // Record an error
    writer
        .record_error(
            "Rate limited. Retry after 60 seconds.",
            Some("rate_limit_error"),
            Some(60),
            50,
        )
        .unwrap();

    // Verify JSONL file exists and contains error
    let jsonl_path = writer.session_jsonl_path();
    assert!(jsonl_path.exists());

    let content = std::fs::read_to_string(&jsonl_path).unwrap();
    let line: serde_json::Value = serde_json::from_str(content.trim()).unwrap();

    assert_eq!(line["type"], "result");
    assert_eq!(line["subtype"], "error");
    assert_eq!(line["isError"], true);
    assert_eq!(line["sessionId"], "error-test-session");
    assert_eq!(line["errorType"], "rate_limit_error");
    assert_eq!(line["retryAfter"], 60);
    assert_eq!(line["durationMs"], 50);
}

#[test]
fn test_state_writer_record_error_without_optional_fields() {
    let writer = StateWriter::new(
        "network-error-session",
        "/tmp/network-error-test",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/network-error-test",
    )
    .unwrap();

    // Record a network error without retry_after
    writer
        .record_error(
            "Network error: Connection refused",
            Some("network_error"),
            None,
            5000,
        )
        .unwrap();

    // Verify JSONL file content
    let jsonl_path = writer.session_jsonl_path();
    let content = std::fs::read_to_string(&jsonl_path).unwrap();
    let line: serde_json::Value = serde_json::from_str(content.trim()).unwrap();

    assert_eq!(line["type"], "result");
    assert_eq!(line["errorType"], "network_error");
    assert_eq!(line["durationMs"], 5000);
    // retryAfter should not be present
    assert!(line.get("retryAfter").is_none());
}
