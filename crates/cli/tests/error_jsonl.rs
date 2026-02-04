// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for error JSONL recording.
//!
//! These tests verify that failure modes correctly record error entries
//! to the session JSONL file in real Claude Code format: `type: "assistant"`
//! with `isApiErrorMessage: true`.

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::{NamedTempFile, TempDir};

fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

/// Create a temporary scenario file with the given content.
fn write_scenario(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

/// Find the API error line in a JSONL file.
///
/// Returns the first line with `type: "assistant"` and `isApiErrorMessage: true`.
fn find_api_error_line(jsonl_path: &PathBuf) -> Option<Value> {
    let content = fs::read_to_string(jsonl_path).ok()?;
    for line in content.lines() {
        if line.is_empty() {
            continue;
        }
        if let Ok(json) = serde_json::from_str::<Value>(line) {
            if json.get("type") == Some(&Value::String("assistant".to_string()))
                && json.get("isApiErrorMessage") == Some(&Value::Bool(true))
            {
                return Some(json);
            }
        }
    }
    None
}

/// Find the JSONL file in a state directory.
///
/// The file is located at: `{state_dir}/projects/{normalized_path}/{session_id}.jsonl`
fn find_jsonl_file(state_dir: &TempDir) -> Option<PathBuf> {
    let projects_dir = state_dir.path().join("projects");
    if !projects_dir.exists() {
        return None;
    }

    // Find the first project directory
    for project_entry in fs::read_dir(&projects_dir).ok()? {
        let project_entry = project_entry.ok()?;
        if project_entry.file_type().ok()?.is_dir() {
            // Find the .jsonl file in this project
            for file_entry in fs::read_dir(project_entry.path()).ok()? {
                let file_entry = file_entry.ok()?;
                let path = file_entry.path();
                if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// Assert common fields on an API error line.
fn assert_api_error_structure(line: &Value, expected_error: &str) {
    assert_eq!(line["type"], "assistant", "should have type: assistant");
    assert_eq!(
        line["isApiErrorMessage"], true,
        "should have isApiErrorMessage: true"
    );
    assert_eq!(
        line["error"], expected_error,
        "should have error: {}",
        expected_error
    );
    assert_eq!(
        line["message"]["model"], "<synthetic>",
        "should have message.model: <synthetic>"
    );
    assert_eq!(
        line["message"]["stop_reason"], "stop_sequence",
        "should have message.stop_reason: stop_sequence"
    );
    assert_eq!(
        line["message"]["usage"]["input_tokens"], 0,
        "should have zero input_tokens"
    );
    assert_eq!(
        line["message"]["content"][0]["type"], "text",
        "should have text content block"
    );
    assert!(line["sessionId"].is_string(), "should have sessionId");
    assert!(line["uuid"].is_string(), "should have uuid");
    assert!(line["cwd"].is_string(), "should have cwd");
}

// =============================================================================
// Rate Limit Error Tests
// =============================================================================

/// Test that --failure rate-limit produces JSONL with API error entry.
#[test]
fn error_jsonl_rate_limit() {
    let state_dir = TempDir::new().unwrap();
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--failure",
            "rate-limit",
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );

    let jsonl_path = find_jsonl_file(&state_dir).expect("JSONL file should exist");
    let error_line =
        find_api_error_line(&jsonl_path).expect("API error line should exist in JSONL");

    assert_api_error_structure(&error_line, "rate_limit");
}

// =============================================================================
// Network Error Tests
// =============================================================================

/// Test that --failure network-unreachable produces JSONL with API error entry.
#[test]
fn error_jsonl_network_unreachable() {
    let state_dir = TempDir::new().unwrap();
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--failure",
            "network-unreachable",
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );

    let jsonl_path = find_jsonl_file(&state_dir).expect("JSONL file should exist");
    let error_line =
        find_api_error_line(&jsonl_path).expect("API error line should exist in JSONL");

    assert_api_error_structure(&error_line, "unknown");
}

// =============================================================================
// Scenario-Based Failure Tests
// =============================================================================

/// Test that scenario-based failures produce JSONL API error entries.
#[test]
fn error_jsonl_scenario_failure() {
    let state_dir = TempDir::new().unwrap();
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "contains", text = "fail" }
        response = ""
        failure = { type = "rate_limit", retry_after = 120 }

        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "fail this request",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );

    let jsonl_path = find_jsonl_file(&state_dir).expect("JSONL file should exist");
    let error_line =
        find_api_error_line(&jsonl_path).expect("API error line should exist in JSONL");

    assert_api_error_structure(&error_line, "rate_limit");
}

/// Test that scenario network_unreachable failure produces JSONL API error entry.
#[test]
fn error_jsonl_scenario_network_failure() {
    let state_dir = TempDir::new().unwrap();
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = ""
        failure = { type = "network_unreachable" }
        "#,
    );

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );

    let jsonl_path = find_jsonl_file(&state_dir).expect("JSONL file should exist");
    let error_line =
        find_api_error_line(&jsonl_path).expect("API error line should exist in JSONL");

    assert_api_error_structure(&error_line, "unknown");
}

// =============================================================================
// Other Failure Mode Tests
// =============================================================================

/// Test that --failure auth-error produces JSONL with API error entry.
#[test]
fn error_jsonl_auth_error() {
    let state_dir = TempDir::new().unwrap();
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--failure",
            "auth-error",
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );

    let jsonl_path = find_jsonl_file(&state_dir).expect("JSONL file should exist");
    let error_line =
        find_api_error_line(&jsonl_path).expect("API error line should exist in JSONL");

    assert_api_error_structure(&error_line, "authentication_failed");
}

/// Test that --failure out-of-credits produces JSONL with API error entry.
#[test]
fn error_jsonl_out_of_credits() {
    let state_dir = TempDir::new().unwrap();
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--failure",
            "out-of-credits",
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );

    let jsonl_path = find_jsonl_file(&state_dir).expect("JSONL file should exist");
    let error_line =
        find_api_error_line(&jsonl_path).expect("API error line should exist in JSONL");

    assert_api_error_structure(&error_line, "billing_error");
}

/// Test that --failure connection-timeout produces JSONL with API error entry.
#[test]
fn error_jsonl_connection_timeout() {
    let state_dir = TempDir::new().unwrap();

    // Use a short timeout scenario to keep test fast
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = ""
        failure = { type = "connection_timeout", after_ms = 50 }
        "#,
    );

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );

    let jsonl_path = find_jsonl_file(&state_dir).expect("JSONL file should exist");
    let error_line =
        find_api_error_line(&jsonl_path).expect("API error line should exist in JSONL");

    assert_api_error_structure(&error_line, "unknown");
}

/// Test that --failure partial-response produces JSONL with API error entry.
#[test]
fn error_jsonl_partial_response() {
    let state_dir = TempDir::new().unwrap();
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--failure",
            "partial-response",
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    // Partial response uses exit code 2
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2: {:?}",
        output
    );

    let jsonl_path = find_jsonl_file(&state_dir).expect("JSONL file should exist");
    let error_line =
        find_api_error_line(&jsonl_path).expect("API error line should exist in JSONL");

    assert_api_error_structure(&error_line, "");
}

// =============================================================================
// No Session Persistence Tests
// =============================================================================

/// Test that --no-session-persistence skips JSONL recording.
#[test]
fn error_jsonl_no_session_persistence() {
    let state_dir = TempDir::new().unwrap();
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--failure",
            "rate-limit",
            "--no-session-persistence",
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );

    // With --no-session-persistence, no JSONL file should be created
    let jsonl_path = find_jsonl_file(&state_dir);
    assert!(
        jsonl_path.is_none(),
        "No JSONL file should exist with --no-session-persistence"
    );
}

// =============================================================================
// Malformed JSON Tests
// =============================================================================

/// Test that --failure malformed-json does NOT produce error JSONL entry.
///
/// Malformed JSON simulates corrupted output, so it doesn't record to JSONL.
#[test]
fn error_jsonl_malformed_json_no_entry() {
    let state_dir = TempDir::new().unwrap();
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--failure",
            "malformed-json",
            "-p",
            "test prompt",
        ])
        .output()
        .expect("Failed to run claudeless");

    // Malformed JSON exits with 0
    assert!(output.status.success(), "Expected success: {:?}", output);

    // JSONL file may exist but should not have an API error entry
    if let Some(jsonl_path) = find_jsonl_file(&state_dir) {
        let error_line = find_api_error_line(&jsonl_path);
        assert!(
            error_line.is_none(),
            "Malformed JSON should not produce API error JSONL entry"
        );
    }
}
