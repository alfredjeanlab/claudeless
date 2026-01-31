// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Smoke tests for exit code behavior.
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

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// When stdin is not a TTY and no prompt is provided, real Claude errors:
/// "Error: Input must be provided either through stdin or as a prompt argument when using --print"
#[test]
fn test_no_prompt_non_tty_errors() {
    let output = Command::new(claudeless_bin())
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
        stderr.contains(
            "Input must be provided either through stdin or as a prompt argument when using --print"
        ),
        "Expected error message: {}",
        stderr
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// When stdin is not a TTY but a prompt IS provided (positional arg, no -p flag),
/// real Claude outputs a response. This tests that positional prompts work without -p.
#[test]
fn test_positional_prompt_non_tty_succeeds() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "contains", text = "hello" }
        response = "Hello! How can I help you today?"
        "#,
    );

    let output = Command::new(claudeless_bin())
        .args(["--scenario", scenario.path().to_str().unwrap(), "Say hello"])
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
fn test_success_exit_code_0() {
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

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_error_exit_code_1() {
    let output = Command::new(claudeless_bin())
        .args(["--failure", "auth-error", "-p", "test"])
        .output()
        .expect("Failed to run claudeless");

    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_partial_exit_code_2() {
    let output = Command::new(claudeless_bin())
        .args(["--failure", "partial-response", "-p", "test"])
        .output()
        .expect("Failed to run claudeless");

    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2: {:?}",
        output
    );
}
