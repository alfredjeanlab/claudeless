// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Smoke tests for CLI flag compatibility.
//!
//! Real Claude CLI v2.1.12 supports various flags that claudeless should either
//! implement or safely ignore for compatibility.
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
// Unsupported Flag Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude supports --add-dir. claudeless should accept it (even if ignored).
#[test]
#[ignore] // DEFERRED: Requires missing CLI flags (epic-05x-fix-cli)
fn test_add_dir_flag_should_be_accepted() {
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
            "--add-dir",
            "/tmp",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    // Should accept the flag, not error
    assert!(output.status.success(), "Expected success: {:?}", output);
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude supports --json-schema. claudeless should accept it (even if ignored).
#[test]
#[ignore] // DEFERRED: Requires missing CLI flags (epic-05x-fix-cli)
fn test_json_schema_flag_should_be_accepted() {
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
            "--json-schema",
            r#"{"type":"object"}"#,
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude supports --tools. claudeless should accept it (even if ignored).
#[test]
#[ignore] // DEFERRED: Requires missing CLI flags (epic-05x-fix-cli)
fn test_tools_flag_should_be_accepted() {
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
            "--tools",
            "Bash,Edit",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude supports --agent. claudeless should accept it (even if ignored).
#[test]
#[ignore] // DEFERRED: Requires missing CLI flags (epic-05x-fix-cli)
fn test_agent_flag_should_be_accepted() {
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
            "--agent",
            "custom-agent",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude supports --append-system-prompt. claudeless should accept it.
#[test]
#[ignore] // DEFERRED: Requires missing CLI flags (epic-05x-fix-cli)
fn test_append_system_prompt_flag_should_be_accepted() {
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
            "--append-system-prompt",
            "extra instructions",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
}

// =============================================================================
// Model Flag Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_model_flag_accepted() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "response regardless of model"
        "#,
    );

    // Model flag should be accepted for compatibility
    let output = Command::new(claudeless_bin())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--model",
            "haiku",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("response regardless of model"),
        "Expected stdout to contain response: {}",
        stdout
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_full_model_name_accepted() {
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
            "--model",
            "claude-haiku-4-5-20251001",
            "-p",
            "test",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
}
