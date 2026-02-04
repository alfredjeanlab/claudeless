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

#[test]
fn test_ask_user_question_with_provided_answers() {
    let call = ToolCallSpec {
        tool: "AskUserQuestion".to_string(),
        input: json!({
            "questions": [
                {
                    "question": "What language?",
                    "header": "Language",
                    "options": [
                        { "label": "Rust", "description": "Systems programming" },
                        { "label": "Python", "description": "Scripting" }
                    ],
                    "multiSelect": false
                }
            ],
            "answers": {
                "What language?": "Python"
            }
        }),
        result: None,
    };

    let result = execute_ask_user_question(&call);
    assert!(!result.is_error);

    // Content text is a human-readable summary (matches real Claude Code format)
    let text = result.text().unwrap();
    assert!(text.contains("User has answered your questions:"));
    assert!(text.contains("\"What language?\"=\"Python\""));

    // tool_use_result has structured data
    let result_json = result.tool_use_result.unwrap();
    assert_eq!(result_json["answers"]["What language?"], "Python");
}

#[test]
fn test_ask_user_question_auto_select_first() {
    let call = ToolCallSpec {
        tool: "AskUserQuestion".to_string(),
        input: json!({
            "questions": [
                {
                    "question": "How should I format the output?",
                    "header": "Format",
                    "options": [
                        { "label": "Summary", "description": "Brief overview" },
                        { "label": "Detailed", "description": "Full explanation" }
                    ],
                    "multiSelect": false
                }
            ]
        }),
        result: None,
    };

    let result = execute_ask_user_question(&call);
    assert!(!result.is_error);

    let text = result.text().unwrap();
    assert!(text.contains("\"How should I format the output?\"=\"Summary\""));

    let result_json = result.tool_use_result.unwrap();
    assert_eq!(
        result_json["answers"]["How should I format the output?"],
        "Summary"
    );
}

#[test]
fn test_ask_user_question_multi_select_auto() {
    let call = ToolCallSpec {
        tool: "AskUserQuestion".to_string(),
        input: json!({
            "questions": [
                {
                    "question": "Which sections?",
                    "header": "Sections",
                    "options": [
                        { "label": "Introduction", "description": "Opening context" },
                        { "label": "Conclusion", "description": "Final summary" }
                    ],
                    "multiSelect": true
                }
            ]
        }),
        result: None,
    };

    let result = execute_ask_user_question(&call);
    assert!(!result.is_error);

    // Auto-select picks the first option
    let text = result.text().unwrap();
    assert!(text.contains("\"Which sections?\"=\"Introduction\""));

    let result_json = result.tool_use_result.unwrap();
    assert_eq!(result_json["answers"]["Which sections?"], "Introduction");
}

#[test]
fn test_ask_user_question_empty_questions() {
    let call = ToolCallSpec {
        tool: "AskUserQuestion".to_string(),
        input: json!({
            "questions": []
        }),
        result: None,
    };

    let result = execute_ask_user_question(&call);
    assert!(!result.is_error);

    let text = result.text().unwrap();
    assert!(text.contains("User has answered your questions"));

    let result_json = result.tool_use_result.unwrap();
    assert_eq!(result_json["answers"], json!({}));
}

#[test]
fn test_ask_user_question_malformed_input() {
    let call = ToolCallSpec {
        tool: "AskUserQuestion".to_string(),
        input: json!({}),
        result: None,
    };

    let result = execute_ask_user_question(&call);
    assert!(!result.is_error);

    let text = result.text().unwrap();
    assert!(text.contains("User has answered your questions"));

    let result_json = result.tool_use_result.unwrap();
    assert_eq!(result_json["answers"], json!({}));
    assert_eq!(result_json["questions"], json!([]));
}
