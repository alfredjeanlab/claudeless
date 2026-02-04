// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Smoke tests for stream-json output format.
//!
//! Real Claude CLI v2.1.12 uses this event sequence:
//! 1. {"type":"system","subtype":"init",...} - initialization with tools, model, settings
//! 2. {"type":"assistant","message":{...},...} - assistant message with content
//! 3. {"type":"result","subtype":"success",...} - final result summary
//!
//! Note: Real Claude requires --verbose with --output-format=stream-json and -p
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::write_scenario;
use std::path::PathBuf;
use std::process::Command;

fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

/// Behavior observed with: claude --version 2.1.23 (Claude Code)
///
/// Real Claude requires --verbose when using --output-format=stream-json with -p:
/// ```
/// $ claude -p "test" --output-format stream-json
/// Error: When using --print, --output-format=stream-json requires --verbose
/// ```
#[test]
#[ignore] // TODO(implement): stream-json with -p should require --verbose
fn test_stream_json_print_requires_verbose() {
    let output = Command::new(claudeless_bin())
        .args(["--output-format", "stream-json", "-p", "test"])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("When using --print, --output-format=stream-json requires --verbose"),
        "Expected error message: {}",
        stderr
    );
}

/// Behavior observed with: claude --version 2.1.23 (Claude Code)
///
/// With --verbose, stream-json output works correctly with -p.
#[test]
fn test_stream_json_print_with_verbose_succeeds() {
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
            "stream-json",
            "--verbose",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(first_line).unwrap();

    // With --verbose, stream should start with system init
    assert_eq!(parsed["type"], "system");
    assert_eq!(parsed["subtype"], "init");
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_stream_json_is_ndjson() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Streamed response"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--output-format",
            "stream-json",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Each line should be valid JSON
    for line in stdout.lines() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(
            parsed.is_ok(),
            "Line should be valid JSON: {} - Error: {:?}",
            line,
            parsed.err()
        );
    }
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude starts stream with {"type":"system","subtype":"init",...}
#[test]
#[ignore] // DEFERRED: Requires output format fix (epic-05x-fix-cli)
fn test_stream_json_starts_with_system_init() {
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
            "stream-json",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(first_line).unwrap();

    // Real Claude starts with system init event
    assert_eq!(
        parsed["type"], "system",
        "Real Claude starts with type=system, not message_start"
    );
    assert_eq!(
        parsed["subtype"], "init",
        "Real Claude starts with subtype=init"
    );
    assert!(
        parsed["tools"].is_array(),
        "System init should include tools array"
    );
    assert!(
        parsed["model"].is_string(),
        "System init should include model"
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude ends stream with {"type":"result","subtype":"success",...}
#[test]
fn test_stream_json_ends_with_result() {
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
            "stream-json",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let last_line = stdout.lines().last().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(last_line).unwrap();

    // Real Claude ends with result event
    assert_eq!(
        parsed["type"], "result",
        "Real Claude ends with type=result, not message_stop"
    );
    assert_eq!(parsed["subtype"], "success");
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_stream_json_exit_code_zero_on_success() {
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
            "stream-json",
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
