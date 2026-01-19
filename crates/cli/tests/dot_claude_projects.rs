// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests verifying claudeless produces ~/.claude/projects/ output
//! matching real Claude CLI (v2.1.12).
//!
//! These tests run claudeless and verify the state directory contents match
//! the expected format from the real Claude CLI.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

mod common;

fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

/// Create a minimal scenario file
fn create_scenario(dir: &TempDir, response: &str) -> PathBuf {
    let scenario_path = dir.path().join("scenario.json");
    let scenario = serde_json::json!({
        "default_response": response
    });
    std::fs::write(&scenario_path, serde_json::to_string(&scenario).unwrap()).unwrap();
    scenario_path
}

// =============================================================================
// Project Directory Naming Tests
// =============================================================================

/// Verify project directory uses path normalization (/ and . become -)
#[test]
fn test_project_dir_uses_normalized_path_name() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_scenario(&state_dir, "Hello!");

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "claudeless failed: {:?}", output);

    // Check projects directory was created
    let projects_dir = state_dir.path().join("projects");
    assert!(projects_dir.exists(), "projects/ directory should exist");

    // Project dir name should be normalized path (/ -> -, . -> -)
    let entries: Vec<_> = std::fs::read_dir(&projects_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(
        entries.len(),
        1,
        "Should have exactly one project directory"
    );

    let project_name = entries[0].file_name().to_string_lossy().to_string();
    assert!(
        project_name.starts_with('-'),
        "Project dir should start with - (normalized absolute path): {}",
        project_name
    );
    assert!(
        !project_name.contains('/'),
        "Project dir should not contain /: {}",
        project_name
    );
    assert!(
        !project_name.contains('.') || project_name == ".",
        "Project dir should not contain . (except single dot): {}",
        project_name
    );
}

// =============================================================================
// Sessions Index Format Tests
// =============================================================================

/// Verify sessions-index.json is created with correct structure
#[test]
fn test_sessions_index_json_created() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_scenario(&state_dir, "Hello!");

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    // Find the project directory
    let projects_dir = state_dir.path().join("projects");
    let project_dir = std::fs::read_dir(&projects_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .next()
        .expect("Should have project directory");

    let sessions_index = project_dir.path().join("sessions-index.json");
    assert!(
        sessions_index.exists(),
        "sessions-index.json should exist at {:?}",
        sessions_index
    );

    let content = std::fs::read_to_string(&sessions_index).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("sessions-index.json should be valid JSON");

    // Verify structure matches real Claude CLI
    assert_eq!(parsed["version"], 1, "version should be 1");
    assert!(parsed["entries"].is_array(), "entries should be an array");

    let entries = parsed["entries"].as_array().unwrap();
    assert!(
        !entries.is_empty(),
        "Should have at least one session entry"
    );

    let entry = &entries[0];
    assert!(entry["sessionId"].is_string(), "sessionId required");
    assert!(entry["fullPath"].is_string(), "fullPath required");
    assert!(entry["fileMtime"].is_number(), "fileMtime required");
    assert!(entry["firstPrompt"].is_string(), "firstPrompt required");
    assert!(entry["messageCount"].is_number(), "messageCount required");
    assert!(entry["created"].is_string(), "created (ISO8601) required");
    assert!(entry["modified"].is_string(), "modified (ISO8601) required");
    assert!(entry["gitBranch"].is_string(), "gitBranch required");
    assert!(entry["projectPath"].is_string(), "projectPath required");
    assert!(entry["isSidechain"].is_boolean(), "isSidechain required");
}

// =============================================================================
// Session JSONL Format Tests
// =============================================================================

/// Verify session file is JSONL format (not plain JSON)
#[test]
fn test_session_file_is_jsonl_format() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_scenario(&state_dir, "Hello!");

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    // Find session file (*.jsonl)
    let projects_dir = state_dir.path().join("projects");
    let project_dir = std::fs::read_dir(&projects_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .next()
        .unwrap();

    let jsonl_files: Vec<_> = std::fs::read_dir(project_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "jsonl").unwrap_or(false))
        .collect();

    assert!(
        !jsonl_files.is_empty(),
        "Should have at least one .jsonl session file"
    );

    let session_content = std::fs::read_to_string(jsonl_files[0].path()).unwrap();

    // Each line should be valid JSON
    for (i, line) in session_content.lines().enumerate() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(
            parsed.is_ok(),
            "Line {} should be valid JSON: {} - Error: {:?}",
            i + 1,
            line,
            parsed.err()
        );
    }
}

/// Verify session JSONL has user message with correct structure
#[test]
fn test_session_jsonl_has_user_message() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_scenario(&state_dir, "Hello!");

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "my test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    // Find and read session file
    let projects_dir = state_dir.path().join("projects");
    let project_dir = std::fs::read_dir(&projects_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .next()
        .unwrap();

    let jsonl_file = std::fs::read_dir(project_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map(|x| x == "jsonl").unwrap_or(false))
        .unwrap();

    let content = std::fs::read_to_string(jsonl_file.path()).unwrap();

    // Find user message line
    let user_msg = content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|v| v["type"] == "user");

    assert!(user_msg.is_some(), "Should have a user message");

    let msg = user_msg.unwrap();
    assert!(msg["uuid"].is_string(), "user message needs uuid");
    assert!(msg["sessionId"].is_string(), "user message needs sessionId");
    assert!(msg["timestamp"].is_string(), "user message needs timestamp");
    assert!(msg["cwd"].is_string(), "user message needs cwd");
    assert_eq!(
        msg["message"]["role"], "user",
        "message.role should be user"
    );
    assert!(
        msg["message"]["content"]
            .as_str()
            .unwrap()
            .contains("my test prompt"),
        "message.content should contain the prompt"
    );
}

/// Verify session JSONL has assistant message with correct structure
#[test]
fn test_session_jsonl_has_assistant_message() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_scenario(&state_dir, "Hello from Claude!");

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args(["--scenario", scenario.to_str().unwrap(), "-p", "test"])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    // Find and read session file
    let projects_dir = state_dir.path().join("projects");
    let project_dir = std::fs::read_dir(&projects_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .next()
        .unwrap();

    let jsonl_file = std::fs::read_dir(project_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map(|x| x == "jsonl").unwrap_or(false))
        .unwrap();

    let content = std::fs::read_to_string(jsonl_file.path()).unwrap();

    // Find assistant message line
    let assistant_msg = content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|v| v["type"] == "assistant");

    assert!(assistant_msg.is_some(), "Should have an assistant message");

    let msg = assistant_msg.unwrap();
    assert!(msg["uuid"].is_string(), "assistant message needs uuid");
    assert!(
        msg["parentUuid"].is_string(),
        "assistant message needs parentUuid"
    );
    assert!(
        msg["sessionId"].is_string(),
        "assistant message needs sessionId"
    );
    assert!(
        msg["timestamp"].is_string(),
        "assistant message needs timestamp"
    );
    assert!(
        msg["requestId"].is_string(),
        "assistant message needs requestId"
    );
    assert_eq!(msg["message"]["role"], "assistant");
    assert!(
        msg["message"]["content"].is_array(),
        "content should be array"
    );
    assert!(
        msg["message"]["model"].is_string(),
        "model should be string"
    );
}

/// Normalize JSON for comparison by replacing variable fields with placeholders
fn normalize_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            // UUID pattern (standard format with hyphens)
            if s.len() == 36
                && s.chars().enumerate().all(|(i, c)| match i {
                    8 | 13 | 18 | 23 => c == '-',
                    _ => c.is_ascii_hexdigit(),
                })
            {
                return serde_json::Value::String("<UUID>".to_string());
            }
            // Message ID pattern (msg_...)
            if s.starts_with("msg_") {
                return serde_json::Value::String("<MESSAGE_ID>".to_string());
            }
            // Request ID pattern (req_...)
            if s.starts_with("req_") {
                return serde_json::Value::String("<REQUEST_ID>".to_string());
            }
            // Tool use ID pattern (toolu_...)
            if s.starts_with("toolu_") {
                return serde_json::Value::String("<TOOL_USE_ID>".to_string());
            }
            // ISO8601 timestamp pattern
            if s.len() >= 19 && s.starts_with("20") && s.contains('T') {
                return serde_json::Value::String("<TIMESTAMP>".to_string());
            }
            // Temp paths
            if s.starts_with("/tmp/")
                || s.starts_with("/var/folders/")
                || s.starts_with("/private/")
            {
                return serde_json::Value::String("<TEMP_PATH>".to_string());
            }
            serde_json::Value::String(s.clone())
        }
        serde_json::Value::Number(n) => {
            // Mtime (milliseconds since epoch, roughly year 2024+)
            if let Some(i) = n.as_i64() {
                if i > 1700000000000 {
                    return serde_json::Value::String("<MTIME>".to_string());
                }
            }
            serde_json::Value::Number(n.clone())
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

/// Validate message sequence order matches expected pattern.
fn validate_message_sequence(actual: &[serde_json::Value], expected: &[serde_json::Value]) {
    let actual_types: Vec<&str> = actual.iter().filter_map(|m| m["type"].as_str()).collect();
    let expected_types: Vec<&str> = expected.iter().filter_map(|m| m["type"].as_str()).collect();

    assert_eq!(
        actual_types, expected_types,
        "Message sequence mismatch.\nActual:   {:?}\nExpected: {:?}",
        actual_types, expected_types
    );
}

/// Validate all messages of a given type have the expected structure.
fn validate_all_messages_of_type(
    actual: &[serde_json::Value],
    expected: &[serde_json::Value],
    msg_type: &str,
) {
    let actual_msgs: Vec<&serde_json::Value> = actual
        .iter()
        .filter(|m| m["type"].as_str() == Some(msg_type))
        .collect();
    let expected_msgs: Vec<&serde_json::Value> = expected
        .iter()
        .filter(|m| m["type"].as_str() == Some(msg_type))
        .collect();

    // Check count matches
    assert_eq!(
        actual_msgs.len(),
        expected_msgs.len(),
        "Message type '{}' count mismatch. Actual: {}, Expected: {}",
        msg_type,
        actual_msgs.len(),
        expected_msgs.len()
    );

    // Check each message structure (all messages should have expected keys)
    for (i, (actual_msg, expected_msg)) in actual_msgs.iter().zip(&expected_msgs).enumerate() {
        compare_message_keys(actual_msg, expected_msg, &format!("{}[{}]", msg_type, i));
    }
}

/// Compare message keys to ensure actual has all expected keys.
fn compare_message_keys(actual: &serde_json::Value, expected: &serde_json::Value, path: &str) {
    let actual_obj = actual.as_object().expect("Message should be an object");
    let expected_obj = expected.as_object().expect("Expected should be an object");

    for key in expected_obj.keys() {
        assert!(
            actual_obj.contains_key(key),
            "Message at '{}' missing key '{}'. Expected keys: {:?}, Actual keys: {:?}",
            path,
            key,
            expected_obj.keys().collect::<Vec<_>>(),
            actual_obj.keys().collect::<Vec<_>>()
        );

        // Recursively check nested objects
        if expected_obj[key].is_object() && actual_obj.contains_key(key) {
            compare_message_keys(
                &actual_obj[key],
                &expected_obj[key],
                &format!("{}.{}", path, key),
            );
        }
    }
}

/// Compare sessions-index.json structure against captured fixture
#[test]
fn test_sessions_index_matches_fixture() {
    // sessions-index.json is synthetic (not scenario-specific), stored at version root
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/dotclaude/v2.1.12/sessions-index.json");

    if !fixture_path.exists() {
        panic!(
            "Fixture not found: {:?}\n\
             Run `./scripts/capture-state.sh` to capture fixtures from real Claude CLI",
            fixture_path
        );
    }

    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_scenario(&state_dir, "Hello!");

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    // Read simulator output
    let projects_dir = state_dir.path().join("projects");
    let project_dir = std::fs::read_dir(&projects_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .next()
        .expect("Should have project directory");

    let sessions_index = project_dir.path().join("sessions-index.json");
    let actual: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&sessions_index).unwrap()).unwrap();
    let actual_normalized = normalize_json(&actual);

    // Read fixture
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&fixture_path).unwrap()).unwrap();

    // Compare structure (keys and types)
    fn compare_structure(actual: &serde_json::Value, expected: &serde_json::Value, path: &str) {
        match (actual, expected) {
            (serde_json::Value::Object(a), serde_json::Value::Object(e)) => {
                // Check expected keys exist in actual
                for key in e.keys() {
                    assert!(
                        a.contains_key(key),
                        "Missing key at {}.{}: expected key from fixture",
                        path,
                        key
                    );
                    compare_structure(&a[key], &e[key], &format!("{}.{}", path, key));
                }
            }
            (serde_json::Value::Array(a), serde_json::Value::Array(e)) => {
                if !a.is_empty() && !e.is_empty() {
                    compare_structure(&a[0], &e[0], &format!("{}[0]", path));
                }
            }
            _ => {
                // For leaf values, just check type matches
                assert_eq!(
                    std::mem::discriminant(actual),
                    std::mem::discriminant(expected),
                    "Type mismatch at {}: actual={:?}, expected={:?}",
                    path,
                    actual,
                    expected
                );
            }
        }
    }

    compare_structure(&actual_normalized, &expected, "sessions-index");
}

/// Compare session.jsonl structure against captured fixture using TodoWrite scenario
#[test]
fn test_session_jsonl_matches_fixture() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/dotclaude/v2.1.12/todo-write");
    let fixture_path = fixture_dir.join("session.jsonl");
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

    // Use the fixture scenario that matches what capture-state.sh produced
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

    assert!(
        output.status.success(),
        "claudeless failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read simulator output
    let projects_dir = state_dir.path().join("projects");
    let project_dir = std::fs::read_dir(&projects_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .next()
        .unwrap();

    let jsonl_file = std::fs::read_dir(project_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map(|x| x == "jsonl").unwrap_or(false))
        .expect("Should have session file");

    let actual_content = std::fs::read_to_string(jsonl_file.path()).unwrap();
    let actual_lines: Vec<serde_json::Value> = actual_content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .map(|v| normalize_json(&v))
        .collect();

    // Read fixture and normalize
    let expected_content = std::fs::read_to_string(&fixture_path).unwrap();
    let expected_lines: Vec<serde_json::Value> = expected_content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .map(|v| normalize_json(&v))
        .collect();

    // Validate message sequence matches fixture
    // Expected: queue-operation, user, assistant (text), assistant (tool_use), user (tool_result), assistant (final)
    validate_message_sequence(&actual_lines, &expected_lines);

    // Validate all messages of each type have correct structure
    for msg_type in ["queue-operation", "user", "assistant"] {
        validate_all_messages_of_type(&actual_lines, &expected_lines, msg_type);
    }
}
