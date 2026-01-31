// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Smoke tests for text and JSON output formats.
//!
//! These tests document the expected behavior based on real Claude CLI v2.1.12.
//! Tests that fail indicate areas where claudeless differs from real Claude
//! and need to be fixed.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::write_scenario;
use std::path::PathBuf;
use std::process::Command;

fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

// =============================================================================
// Text Output Format Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_basic_text_output_with_scenario() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "contains", text = "hello" }
        response = "Hello! How can I help you today?"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "hello world",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello! How can I help you today?"),
        "Expected stdout to contain response: {}",
        stdout
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_text_output_is_plain_text_no_json() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Simple response"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Text output should NOT be JSON
    assert!(
        !stdout.starts_with('{'),
        "Text output should not be JSON, got: {}",
        stdout
    );
    assert!(stdout.contains("Simple response"));
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_text_output_exit_code_zero_on_success() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit code 0: {:?}",
        output
    );
}

// =============================================================================
// JSON Output Format Tests
//
// Real Claude CLI v2.1.12 uses a result wrapper format:
// {
//   "type": "result",
//   "subtype": "success",
//   "result": "response text",
//   "session_id": "...",
//   "cost_usd": 0.001,
//   ...
// }
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude returns a result wrapper, not raw API message format.
#[test]
fn test_json_output_uses_result_wrapper_format() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Test response"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--output-format",
            "json",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Real Claude uses result wrapper format
    assert_eq!(
        parsed["type"], "result",
        "Real Claude returns type=result, not type=message"
    );
    assert_eq!(
        parsed["subtype"], "success",
        "Real Claude returns subtype=success"
    );
    assert_eq!(parsed["is_error"], false);
    assert!(
        parsed["result"].is_string(),
        "Real Claude has 'result' field with response text"
    );
    assert!(parsed["session_id"].is_string());
    assert!(parsed["duration_ms"].is_number());
    assert!(
        parsed["cost_usd"].is_number(),
        "Real Claude includes cost_usd"
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_json_output_result_contains_response_text() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Expected response text"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--output-format",
            "json",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Real Claude puts response text in "result" field, not content[0].text
    assert_eq!(
        parsed["result"], "Expected response text",
        "Response should be in 'result' field"
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_json_output_exit_code_zero_on_success() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--output-format",
            "json",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit code 0: {:?}",
        output
    );
}
