// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Smoke tests for failure simulation and response delays.
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
use std::time::Instant;

fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

// =============================================================================
// Failure Mode Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_connection_timeout_via_scenario() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = ""
        failure = { type = "connection_timeout", after_ms = 50 }
        "#,
    );

    let start = Instant::now();

    let output = Command::new(claudeless_bin())
        .args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test",
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

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() >= 50,
        "Timeout should delay at least 50ms"
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_auth_error_exit_code() {
    let output = Command::new(claudeless_bin())
        .args(["--failure", "auth-error", "-p", "test"])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_rate_limit_exit_code() {
    let output = Command::new(claudeless_bin())
        .args(["--failure", "rate-limit", "-p", "test"])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_network_unreachable_exit_code() {
    let output = Command::new(claudeless_bin())
        .args(["--failure", "network-unreachable", "-p", "test"])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_out_of_credits_exit_code() {
    let output = Command::new(claudeless_bin())
        .args(["--failure", "out-of-credits", "-p", "test"])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1: {:?}",
        output
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_partial_response_exit_code_2() {
    let output = Command::new(claudeless_bin())
        .args(["--failure", "partial-response", "-p", "test"])
        .output()
        .expect("Failed to run claudeless");

    // Partial response uses exit code 2
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2: {:?}",
        output
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_malformed_json_exit_code() {
    let output = Command::new(claudeless_bin())
        .args(["--failure", "malformed-json", "-p", "test"])
        .output()
        .expect("Failed to run claudeless");

    // Malformed JSON still exits with 0
    assert!(output.status.success(), "Expected success: {:?}", output);
}

// =============================================================================
// Delay Tests
// =============================================================================

/// Response delay via [timeouts] section in scenario
#[test]
fn test_response_delay_via_scenario_timeouts() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [timeouts]
        response_delay_ms = 200
        [[responses]]
        pattern = { type = "any" }
        response = "delayed"
        "#,
    );

    let start = Instant::now();

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
    assert!(
        stdout.contains("delayed"),
        "Expected stdout to contain 'delayed': {}",
        stdout
    );

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() >= 200,
        "Response should be delayed by at least 200ms, but took {}ms",
        elapsed.as_millis()
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_delay_ms_via_scenario_response() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = { text = "delayed via scenario", delay_ms = 150 }
        "#,
    );

    let start = Instant::now();

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
    assert!(
        stdout.contains("delayed via scenario"),
        "Expected stdout to contain 'delayed via scenario': {}",
        stdout
    );

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() >= 150,
        "Response should be delayed by at least 150ms, but took {}ms",
        elapsed.as_millis()
    );
}
