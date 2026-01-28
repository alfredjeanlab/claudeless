// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use chrono::Utc;
use serde_json::json;

fn create_test_state_writer() -> StateWriter {
    StateWriter::new(
        uuid::Uuid::new_v4().to_string(),
        "/tmp/test",
        Utc::now(),
        "claude-sonnet-4-20250514",
        "/tmp/test",
    )
    .unwrap()
}

#[test]
fn test_parse_todo_item() {
    let value = json!({
        "content": "Build the project",
        "status": "pending",
        "activeForm": "Building the project"
    });

    let item = parse_todo_item(&value).unwrap();
    assert_eq!(item.content, "Build the project");
    assert_eq!(item.status, TodoStatus::Pending);
    assert_eq!(item.active_form, Some("Building the project".to_string()));
}

#[test]
fn test_parse_todo_item_in_progress() {
    let value = json!({
        "content": "Running tests",
        "status": "in_progress",
        "activeForm": "Running tests"
    });

    let item = parse_todo_item(&value).unwrap();
    assert_eq!(item.status, TodoStatus::InProgress);
}

#[test]
fn test_parse_todo_item_completed() {
    let value = json!({
        "content": "Done task",
        "status": "completed",
        "activeForm": "Done"
    });

    let item = parse_todo_item(&value).unwrap();
    assert_eq!(item.status, TodoStatus::Completed);
}

#[test]
fn test_execute_todo_write() {
    let writer = create_test_state_writer();
    let call = ToolCallSpec {
        tool: "TodoWrite".to_string(),
        input: json!({
            "todos": [
                {
                    "content": "Task 1",
                    "status": "pending",
                    "activeForm": "Doing task 1"
                },
                {
                    "content": "Task 2",
                    "status": "in_progress",
                    "activeForm": "Doing task 2"
                }
            ]
        }),
        result: None,
    };

    let result = execute_todo_write(&call, &writer);
    assert!(!result.is_error);
    assert!(result
        .text()
        .unwrap()
        .contains("Todos have been modified successfully"));

    // Verify file was created
    assert!(writer.todo_path().exists());
}

#[test]
fn test_execute_exit_plan_mode() {
    let writer = create_test_state_writer();
    let call = ToolCallSpec {
        tool: "ExitPlanMode".to_string(),
        input: json!({
            "plan_content": "# My Plan\n\n## Steps\n\n1. Do this\n2. Do that"
        }),
        result: None,
    };

    let result = execute_exit_plan_mode(&call, &writer);
    assert!(!result.is_error);
    assert!(result.text().unwrap().contains("Plan saved as"));
    assert!(result.text().unwrap().ends_with(".md"));
}

#[test]
fn test_execute_exit_plan_mode_no_content() {
    let writer = create_test_state_writer();
    let call = ToolCallSpec {
        tool: "ExitPlanMode".to_string(),
        input: json!({}),
        result: None,
    };

    let result = execute_exit_plan_mode(&call, &writer);
    assert!(!result.is_error);
    // Should use default content
    assert!(result.text().unwrap().contains("Plan saved as"));
}
