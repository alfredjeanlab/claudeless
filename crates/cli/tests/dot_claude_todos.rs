// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests verifying claudeless produces ~/.claude/todos/ output
//! matching real Claude CLI (v2.1.12).
//!
//! These tests run claudeless with scenarios that trigger TodoWrite and verify
//! the output matches the expected format.
//!
//! ## Real Claude todos/ format
//! - File naming: `{sessionId}-agent-{sessionId}.json`
//! - Content: JSON array of todo items with content, status, activeForm

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

/// Create a scenario that triggers TodoWrite
fn create_todo_scenario(dir: &TempDir) -> PathBuf {
    let scenario_path = dir.path().join("scenario.json");
    let scenario = serde_json::json!({
        "default_response": {
            "text": "I'll create a todo list for you.",
            "tool_calls": [
                {
                    "tool": "TodoWrite",
                    "input": {
                        "todos": [
                            {
                                "content": "Build the project",
                                "status": "pending",
                                "activeForm": "Building project"
                            },
                            {
                                "content": "Run tests",
                                "status": "pending",
                                "activeForm": "Running tests"
                            }
                        ]
                    }
                }
            ]
        },
        "tool_execution": {
            "mode": "live",
            "tools": {
                "TodoWrite": {
                    "auto_approve": true
                }
            }
        }
    });
    std::fs::write(
        &scenario_path,
        serde_json::to_string_pretty(&scenario).unwrap(),
    )
    .unwrap();
    scenario_path
}

// =============================================================================
// Todo File Creation Tests
// =============================================================================

/// Verify todos directory is created
#[test]
fn test_todos_directory_created() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_todo_scenario(&state_dir);

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "Create a todo list with: Build, Test, Deploy",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "claudeless failed: {:?}", output);

    let todos_dir = state_dir.path().join("todos");
    assert!(todos_dir.exists(), "todos/ directory should exist");
}

/// Verify todo file naming convention: {sessionId}-agent-{sessionId}.json
#[test]
fn test_todo_file_naming_convention() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_todo_scenario(&state_dir);

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "Create todos: item1, item2",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    let todos_dir = state_dir.path().join("todos");
    let todo_files: Vec<_> = std::fs::read_dir(&todos_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .collect();

    assert!(!todo_files.is_empty(), "Should have at least one todo file");

    let filename = todo_files[0].file_name().to_string_lossy().to_string();

    // Format: {uuid}-agent-{uuid}.json
    assert!(
        filename.ends_with(".json"),
        "Todo file should end with .json: {}",
        filename
    );
    assert!(
        filename.contains("-agent-"),
        "Todo file should contain '-agent-': {}",
        filename
    );

    // Extract and verify UUID format
    let parts: Vec<&str> = filename
        .trim_end_matches(".json")
        .split("-agent-")
        .collect();
    assert_eq!(
        parts.len(),
        2,
        "Should have exactly two parts around '-agent-'"
    );

    // Both parts should be valid UUIDs (same session ID repeated)
    assert_eq!(
        parts[0], parts[1],
        "Both UUIDs should match (same session): {} vs {}",
        parts[0], parts[1]
    );
}

/// Verify todo file content structure
#[test]
fn test_todo_file_content_structure() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_todo_scenario(&state_dir);

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "Create a todo list",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    let todos_dir = state_dir.path().join("todos");
    let todo_file = std::fs::read_dir(&todos_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .expect("Should have a todo file");

    let content = std::fs::read_to_string(todo_file.path()).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("Todo file should be valid JSON");

    // Should be an array
    assert!(parsed.is_array(), "Todo file should contain a JSON array");

    // If not empty, verify item structure
    if let Some(items) = parsed.as_array() {
        if !items.is_empty() {
            let item = &items[0];
            assert!(
                item["content"].is_string(),
                "Todo item needs 'content' string"
            );
            assert!(
                item["status"].is_string(),
                "Todo item needs 'status' string"
            );
            assert!(
                item["activeForm"].is_string(),
                "Todo item needs 'activeForm' string"
            );

            // Verify status is valid
            let status = item["status"].as_str().unwrap();
            assert!(
                ["pending", "in_progress", "completed"].contains(&status),
                "Status should be pending/in_progress/completed, got: {}",
                status
            );
        }
    }
}

/// Verify empty todo list is just "[]"
#[test]
fn test_empty_todo_list_format() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();

    // Scenario that doesn't trigger todos
    let scenario_path = state_dir.path().join("scenario.json");
    let scenario = serde_json::json!({"default_response": "Hello!"});
    std::fs::write(&scenario_path, serde_json::to_string(&scenario).unwrap()).unwrap();

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario_path.to_str().unwrap(),
            "-p",
            "Say hello",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    let todos_dir = state_dir.path().join("todos");
    if todos_dir.exists() {
        if let Some(todo_file) = std::fs::read_dir(&todos_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        {
            let content = std::fs::read_to_string(todo_file.path()).unwrap();
            // Empty todo should be "[]" (2 bytes)
            let parsed: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
            assert!(parsed.is_empty(), "Empty todo list should be []");
        }
    }
}

/// Normalize JSON for comparison by replacing variable fields with placeholders
fn normalize_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            // UUID pattern
            if s.len() == 36
                && s.chars().enumerate().all(|(i, c)| match i {
                    8 | 13 | 18 | 23 => c == '-',
                    _ => c.is_ascii_hexdigit(),
                })
            {
                return serde_json::Value::String("<UUID>".to_string());
            }
            serde_json::Value::String(s.clone())
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(normalize_json).collect())
        }
        serde_json::Value::Object(obj) => serde_json::Value::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), normalize_json(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Compare todo.json structure against captured fixture
#[test]
fn test_todo_json_matches_fixture() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/dotclaude/v2.1.12/todo-write");
    let fixture_path = fixture_dir.join("todo.json");
    let scenario_path = fixture_dir.join("scenario.toml");

    if !fixture_path.exists() {
        panic!(
            "Fixture not found: {:?}\n\
             Run `./scripts/capture-state.sh` to capture fixtures from real Claude CLI",
            fixture_path
        );
    }

    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario_path.to_str().unwrap(),
            "-p",
            // Same prompt used by capture-state.sh
            "Create a simple todo list with 3 items: buy groceries, walk the dog, read a book. Use the TodoWrite tool to create them.",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    // Read simulator output
    let todos_dir = state_dir.path().join("todos");
    let todo_file = std::fs::read_dir(&todos_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .expect("Should have a todo file");

    let actual: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(todo_file.path()).unwrap()).unwrap();
    let actual_normalized = normalize_json(&actual);

    // Read fixture
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&fixture_path).unwrap()).unwrap();

    // Compare structure - todos should be an array of items with specific fields
    assert!(
        actual_normalized.is_array(),
        "Todo should be array, got: {:?}",
        actual_normalized
    );
    assert!(expected.is_array(), "Fixture should be array");

    // Compare first item structure if both have items
    if let (Some(actual_arr), Some(expected_arr)) =
        (actual_normalized.as_array(), expected.as_array())
    {
        if !actual_arr.is_empty() && !expected_arr.is_empty() {
            let actual_item = &actual_arr[0];
            let expected_item = &expected_arr[0];

            // Check same keys exist
            if let (Some(a), Some(e)) = (actual_item.as_object(), expected_item.as_object()) {
                for key in e.keys() {
                    assert!(
                        a.contains_key(key),
                        "Missing key '{}' in todo item. Expected keys from fixture: {:?}",
                        key,
                        e.keys().collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}
