// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

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

    assert_eq!(writer.session_id(), "test-session-id");
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
    let entry = index.get(writer.session_id()).unwrap();
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
    assert_eq!(parsed[0].status, "pending");
    assert_eq!(parsed[0].active_form, "Building project");
    assert_eq!(parsed[1].status, "in_progress");
}

#[test]
fn test_record_turn_has_end_turn_stop_reason() {
    let mut writer = StateWriter::new(
        Uuid::new_v4().to_string(),
        "/tmp/test-project",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/test-project",
    )
    .unwrap();

    writer.record_turn("Hello", "Hi there!").unwrap();

    let content = std::fs::read_to_string(writer.session_jsonl_path()).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);

    // Second line is the assistant message
    let assistant: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(
        assistant["message"]["stop_reason"],
        serde_json::json!("end_turn"),
    );
}

#[test]
fn test_record_assistant_response_has_end_turn_stop_reason() {
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
        .record_assistant_response(&user_uuid, "Hi there!")
        .unwrap();

    let content = std::fs::read_to_string(writer.session_jsonl_path()).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);

    let assistant: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(
        assistant["message"]["stop_reason"],
        serde_json::json!("end_turn"),
    );
}

#[test]
fn test_record_assistant_tool_use_has_tool_use_stop_reason() {
    let mut writer = StateWriter::new(
        Uuid::new_v4().to_string(),
        "/tmp/test-project",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/test-project",
    )
    .unwrap();

    let user_uuid = writer.record_user_message("List files").unwrap();
    let content = vec![
        ContentBlock::Text {
            text: "Let me list the files.".to_string(),
        },
        ContentBlock::ToolUse {
            id: "toolu_123".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
        },
    ];
    writer
        .record_assistant_tool_use(&user_uuid, content)
        .unwrap();

    let file_content = std::fs::read_to_string(writer.session_jsonl_path()).unwrap();
    let lines: Vec<&str> = file_content.lines().collect();
    assert_eq!(lines.len(), 2);

    let assistant: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(
        assistant["message"]["stop_reason"],
        serde_json::json!("tool_use"),
    );
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
