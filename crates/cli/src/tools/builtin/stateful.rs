// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Stateful tool executors (TodoWrite, ExitPlanMode).
//!
//! These tools write to the state directory and require a `StateWriter`.

use crate::config::ToolCallSpec;
use crate::state::{StateWriter, TodoItem, TodoStatus};
use crate::tools::result::{ToolExecutionResult, ToolResultContent};

/// Execute the TodoWrite tool.
///
/// Parses todo items from the tool call input and writes them to the
/// state directory in Claude CLI format.
pub fn execute_todo_write(call: &ToolCallSpec, state_writer: &StateWriter) -> ToolExecutionResult {
    // Parse todo items from call.input
    let todos: Vec<TodoItem> = match call.input.get("todos") {
        Some(serde_json::Value::Array(arr)) => arr.iter().filter_map(parse_todo_item).collect(),
        _ => vec![],
    };

    // Build tool_use_result with oldTodos/newTodos
    let new_todos_json: Vec<serde_json::Value> = todos
        .iter()
        .map(|t| {
            serde_json::json!({
                "content": t.content,
                "status": match t.status {
                    TodoStatus::Pending => "pending",
                    TodoStatus::InProgress => "in_progress",
                    TodoStatus::Completed => "completed",
                },
                "activeForm": t.active_form.clone().unwrap_or_else(|| t.content.clone())
            })
        })
        .collect();

    let tool_use_result = serde_json::json!({
        "oldTodos": [],
        "newTodos": new_todos_json
    });

    match state_writer.write_todos(&todos) {
        Ok(()) => ToolExecutionResult {
            tool_use_id: String::new(), // Set by caller
            content: vec![ToolResultContent::Text {
                text: "Todos have been modified successfully. Ensure that you continue to use the todo list to track your progress. Please proceed with the current tasks if applicable".to_string(),
            }],
            is_error: false,
            tool_use_result: Some(tool_use_result),
        },
        Err(e) => ToolExecutionResult {
            tool_use_id: String::new(),
            content: vec![ToolResultContent::Text {
                text: format!("Failed to write todos: {}", e),
            }],
            is_error: true,
            tool_use_result: None,
        },
    }
}

/// Parse a single todo item from JSON.
fn parse_todo_item(value: &serde_json::Value) -> Option<TodoItem> {
    let content = value.get("content")?.as_str()?.to_string();
    let status = match value.get("status")?.as_str()? {
        "pending" => TodoStatus::Pending,
        "in_progress" => TodoStatus::InProgress,
        "completed" => TodoStatus::Completed,
        _ => TodoStatus::Pending,
    };
    let active_form = value
        .get("activeForm")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(TodoItem {
        id: format!("todo_{}", rand_id()),
        content,
        active_form,
        status,
        priority: 0,
    })
}

/// Generate a simple random ID.
fn rand_id() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut hasher = DefaultHasher::new();
    nanos.hash(&mut hasher);
    hasher.finish()
}

/// Execute the ExitPlanMode tool.
///
/// Creates a plan file in markdown format with word-based naming.
pub fn execute_exit_plan_mode(
    call: &ToolCallSpec,
    state_writer: &StateWriter,
) -> ToolExecutionResult {
    // Try to get plan content from various possible field names
    let content = call
        .input
        .get("plan_content")
        .or_else(|| call.input.get("planContent"))
        .or_else(|| call.input.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("# Plan\n\nNo content provided.");

    match state_writer.create_plan(content) {
        Ok(name) => ToolExecutionResult {
            tool_use_id: String::new(), // Set by caller
            content: vec![ToolResultContent::Text {
                text: format!("Plan saved as {}.md", name),
            }],
            is_error: false,
            tool_use_result: None,
        },
        Err(e) => ToolExecutionResult {
            tool_use_id: String::new(),
            content: vec![ToolResultContent::Text {
                text: format!("Failed to save plan: {}", e),
            }],
            is_error: true,
            tool_use_result: None,
        },
    }
}

#[cfg(test)]
mod tests {
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
}
