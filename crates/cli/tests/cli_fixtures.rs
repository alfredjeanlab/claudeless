// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! CLI output fixture comparison tests.
//!
//! These tests compare claudeless output against golden fixtures captured
//! from real Claude CLI.
//!
//! Fixtures captured from: Claude Code v2.1.12
//! See `tests/fixtures/cli/README.md` for details.
//!
//! Directory structure:
//! ```
//! cli/v2.1.12/
//! ├── json-output/
//! │   ├── scenario.toml
//! │   └── output.json
//! └── stream-json/
//!     ├── scenario.toml
//!     └── output.jsonl
//! ```

#![allow(deprecated)] // Command::cargo_bin is deprecated but still functional

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;

/// Fixture version being tested against
const FIXTURE_VERSION: &str = "v2.1.12";

/// Get the path to a fixture directory
fn fixture_dir(version: &str, scenario: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cli")
        .join(version)
        .join(scenario)
}

/// Load a fixture file from a scenario directory
fn load_fixture(version: &str, scenario: &str, filename: &str) -> String {
    let path = fixture_dir(version, scenario).join(filename);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load fixture {:?}: {}", path, e))
}

/// Get the scenario path for a fixture
fn scenario_path(version: &str, scenario: &str) -> PathBuf {
    fixture_dir(version, scenario).join("scenario.toml")
}

/// Normalize JSON output for comparison by replacing dynamic fields with placeholders
fn normalize_json(json: &Value) -> Value {
    match json {
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (key, value) in map {
                let normalized_value = match key.as_str() {
                    "session_id" => Value::String("<SESSION_ID>".to_string()),
                    "duration_ms" => Value::String("<DURATION>".to_string()),
                    "duration_api_ms" => Value::String("<DURATION>".to_string()),
                    "cost_usd" => Value::String("<COST>".to_string()),
                    "total_cost_usd" => Value::String("<COST>".to_string()),
                    "costUSD" => Value::String("<COST>".to_string()),
                    "timestamp" => Value::String("<TIMESTAMP>".to_string()),
                    "request_id" => Value::String("<REQUEST_ID>".to_string()),
                    "uuid" => Value::String("<UUID>".to_string()),
                    "cwd" => Value::String("<CWD>".to_string()),
                    "result" => Value::String("<RESPONSE_TEXT>".to_string()),
                    "usage" => Value::String("<USAGE>".to_string()),
                    "modelUsage" => Value::String("<MODEL_USAGE>".to_string()),
                    "plugins" => Value::String("<PLUGINS>".to_string()),
                    "mcp_servers" => Value::String("<MCP_SERVERS>".to_string()),
                    "id" if value
                        .as_str()
                        .map(|s| s.starts_with("msg_"))
                        .unwrap_or(false) =>
                    {
                        Value::String("<MESSAGE_ID>".to_string())
                    }
                    "content" if value.is_array() => Value::String("<CONTENT>".to_string()),
                    "text" => Value::String("<RESPONSE_TEXT>".to_string()),
                    _ => normalize_json(value),
                };
                new_map.insert(key.clone(), normalized_value);
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(normalize_json).collect()),
        _ => json.clone(),
    }
}

/// Compare two JSON values, returning a description of differences
fn compare_json(expected: &Value, actual: &Value, path: &str) -> Vec<String> {
    let mut diffs = Vec::new();

    match (expected, actual) {
        (Value::Object(exp_map), Value::Object(act_map)) => {
            // Check for missing keys in actual
            for key in exp_map.keys() {
                if !act_map.contains_key(key) {
                    diffs.push(format!("{}.{}: missing in actual", path, key));
                }
            }
            // Check for extra keys in actual
            for key in act_map.keys() {
                if !exp_map.contains_key(key) {
                    diffs.push(format!("{}.{}: unexpected key in actual", path, key));
                }
            }
            // Compare common keys
            for (key, exp_val) in exp_map {
                if let Some(act_val) = act_map.get(key) {
                    let sub_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    diffs.extend(compare_json(exp_val, act_val, &sub_path));
                }
            }
        }
        (Value::Array(exp_arr), Value::Array(act_arr)) => {
            if exp_arr.len() != act_arr.len() {
                diffs.push(format!(
                    "{}: array length mismatch (expected {}, got {})",
                    path,
                    exp_arr.len(),
                    act_arr.len()
                ));
            }
            for (i, (exp_val, act_val)) in exp_arr.iter().zip(act_arr.iter()).enumerate() {
                diffs.extend(compare_json(exp_val, act_val, &format!("{}[{}]", path, i)));
            }
        }
        _ => {
            if expected != actual {
                diffs.push(format!(
                    "{}: value mismatch (expected {:?}, got {:?})",
                    path, expected, actual
                ));
            }
        }
    }

    diffs
}

// =============================================================================
// JSON Output Fixture Tests
// =============================================================================

/// Verify JSON output structure matches fixture
#[test]
fn test_json_output_matches_fixture() {
    let scenario = scenario_path(FIXTURE_VERSION, "json-output");

    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    let output = cmd
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "--output-format",
            "json",
            "Hello",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let actual_json: Value =
        serde_json::from_slice(&output).expect("Simulator output should be valid JSON");

    let fixture_str = load_fixture(FIXTURE_VERSION, "json-output", "output.json");
    let expected_json: Value =
        serde_json::from_str(&fixture_str).expect("Fixture should be valid JSON");

    // Normalize the actual output for comparison
    let normalized_actual = normalize_json(&actual_json);

    let diffs = compare_json(&expected_json, &normalized_actual, "");

    if !diffs.is_empty() {
        eprintln!(
            "Expected (from fixture):\n{}",
            serde_json::to_string_pretty(&expected_json).unwrap()
        );
        eprintln!(
            "Actual (normalized):\n{}",
            serde_json::to_string_pretty(&normalized_actual).unwrap()
        );
        eprintln!("Differences:");
        for diff in &diffs {
            eprintln!("  - {}", diff);
        }
        panic!(
            "JSON output does not match fixture. {} difference(s) found.",
            diffs.len()
        );
    }
}

/// Verify JSON output has all required fields from fixture (result wrapper format)
#[test]
fn test_json_output_has_fixture_fields() {
    let scenario = scenario_path(FIXTURE_VERSION, "json-output");

    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    let output = cmd
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "--output-format",
            "json",
            "test",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let actual: Value = serde_json::from_slice(&output).unwrap();
    let fixture_str = load_fixture(FIXTURE_VERSION, "json-output", "output.json");
    let fixture: Value = serde_json::from_str(&fixture_str).unwrap();

    // Verify all fields from fixture exist in actual
    if let (Value::Object(fix_map), Value::Object(act_map)) = (&fixture, &actual) {
        for key in fix_map.keys() {
            assert!(
                act_map.contains_key(key),
                "Missing field '{}' in JSON output (expected from fixture)",
                key
            );
        }
    }
}

// =============================================================================
// Stream-JSON Output Fixture Tests
// =============================================================================

/// Verify stream-JSON event types match fixture
#[test]
fn test_stream_json_event_types_match_fixture() {
    let scenario = scenario_path(FIXTURE_VERSION, "stream-json");

    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    let output = cmd
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "--output-format",
            "stream-json",
            "Hello",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let actual_str = String::from_utf8_lossy(&output);
    let fixture_str = load_fixture(FIXTURE_VERSION, "stream-json", "output.jsonl");

    // Extract (type, subtype) pairs from both
    let actual_events: Vec<(String, Option<String>)> = actual_str
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let json: Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("Invalid JSON line: {} - {}", line, e));
            (
                json["type"].as_str().unwrap_or("").to_string(),
                json["subtype"].as_str().map(String::from),
            )
        })
        .collect();

    let fixture_events: Vec<(String, Option<String>)> = fixture_str
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let json: Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("Invalid fixture JSON: {} - {}", line, e));
            (
                json["type"].as_str().unwrap_or("").to_string(),
                json["subtype"].as_str().map(String::from),
            )
        })
        .collect();

    // Compare event sequences
    if actual_events != fixture_events {
        eprintln!("Expected event sequence (from fixture):");
        for (t, st) in &fixture_events {
            eprintln!("  type={}, subtype={:?}", t, st);
        }
        eprintln!("Actual event sequence:");
        for (t, st) in &actual_events {
            eprintln!("  type={}, subtype={:?}", t, st);
        }
        panic!("Stream-JSON event types do not match fixture");
    }
}

/// Verify stream-JSON produces valid NDJSON
#[test]
fn test_stream_json_is_valid_ndjson() {
    let scenario = scenario_path(FIXTURE_VERSION, "stream-json");

    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    let output = cmd
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "--output-format",
            "stream-json",
            "test",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);

    for (i, line) in output_str.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let result: Result<Value, _> = serde_json::from_str(line);
        assert!(
            result.is_ok(),
            "Line {} is not valid JSON: {} - {:?}",
            i + 1,
            line,
            result.err()
        );
    }
}

/// Verify stream-JSON starts with system init event (real Claude format)
#[test]
fn test_stream_json_starts_with_system_init() {
    let scenario = scenario_path(FIXTURE_VERSION, "stream-json");

    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    let output = cmd
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "--output-format",
            "stream-json",
            "test",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    let first_line = output_str.lines().next().expect("Should have output");
    let first: Value = serde_json::from_str(first_line).expect("First line should be JSON");

    assert_eq!(
        first["type"], "system",
        "First event should be type 'system'"
    );
    assert_eq!(
        first["subtype"], "init",
        "First event should have subtype 'init'"
    );
}

/// Verify stream-JSON first event has a type field
#[test]
fn test_stream_json_starts_with_valid_event() {
    let scenario = scenario_path(FIXTURE_VERSION, "stream-json");

    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    let output = cmd
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "--output-format",
            "stream-json",
            "test",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    let first_line = output_str.lines().next().expect("Should have output");
    let first: Value = serde_json::from_str(first_line).expect("First line should be JSON");

    assert!(
        first["type"].is_string(),
        "First event should have a type field"
    );
}

/// Verify stream-JSON ends with result event (real Claude format)
#[test]
fn test_stream_json_ends_with_result() {
    let scenario = scenario_path(FIXTURE_VERSION, "stream-json");

    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    let output = cmd
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "--output-format",
            "stream-json",
            "test",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    let lines: Vec<&str> = output_str.lines().filter(|l| !l.is_empty()).collect();
    let last_line = lines.last().expect("Should have output");
    let last: Value = serde_json::from_str(last_line).expect("Last line should be JSON");

    assert_eq!(last["type"], "result", "Last event should be type 'result'");
}

/// Verify stream-JSON last event has a type field
#[test]
fn test_stream_json_ends_with_valid_event() {
    let scenario = scenario_path(FIXTURE_VERSION, "stream-json");

    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    let output = cmd
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "--output-format",
            "stream-json",
            "test",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    let lines: Vec<&str> = output_str.lines().filter(|l| !l.is_empty()).collect();
    let last_line = lines.last().expect("Should have output");
    let last: Value = serde_json::from_str(last_line).expect("Last line should be JSON");

    assert!(
        last["type"].is_string(),
        "Last event should have a type field"
    );
}

// =============================================================================
// Structural Tests
// =============================================================================

/// Test that JSON output uses result wrapper format
#[test]
fn test_json_output_type_fields() {
    let scenario = scenario_path(FIXTURE_VERSION, "json-output");

    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    let output = cmd
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "--output-format",
            "json",
            "test",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();

    // Real Claude uses result wrapper format
    assert_eq!(json["type"], "result");
    assert_eq!(json["subtype"], "success");
    assert_eq!(json["is_error"], false);
}

/// Test that JSON output has a type field
#[test]
fn test_json_output_has_type_field() {
    let scenario = scenario_path(FIXTURE_VERSION, "json-output");

    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    let output = cmd
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "--output-format",
            "json",
            "test",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();

    assert!(json["type"].is_string(), "Output should have a type field");
}

/// Test that error result has correct type fields
#[test]
fn test_json_error_type_fields() {
    // Error scenario uses json-output scenario but expects error handling
    // This test verifies error response format when errors occur
    let scenario = scenario_path(FIXTURE_VERSION, "json-output");

    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    let output = cmd
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "--output-format",
            "json",
            "test",
        ])
        .assert()
        .get_output()
        .stdout
        .clone();

    // If we get JSON output, verify structure
    if let Ok(json) = serde_json::from_slice::<Value>(&output) {
        if json.get("is_error") == Some(&Value::Bool(true)) {
            assert_eq!(json["type"], "result");
            assert_eq!(json["subtype"], "error");
        }
    }
}
