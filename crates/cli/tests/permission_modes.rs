// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for permission modes.

use rstest::rstest;
use std::path::PathBuf;
use std::process::Command;

fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

/// Test that all permission mode flags are accepted.
#[rstest]
#[case("default", "hello", 0)]
#[case("accept-edits", "hello", 0)]
#[case("bypass-permissions", "hello", 0)]
#[case("plan", "hello", 0)]
#[case("dont-ask", "hello", 0)]
#[case("delegate", "hello", 0)]
fn test_permission_mode_flag_accepted(
    #[case] mode: &str,
    #[case] prompt: &str,
    #[case] expected_exit: i32,
) {
    let output = Command::new(claudeless_bin())
        .args(["-p", "--permission-mode", mode, prompt])
        .output()
        .expect("Failed to run claudeless");

    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "Expected exit code {}: {:?}",
        expected_exit,
        output
    );
}

#[test]
fn test_invalid_permission_mode_rejected() {
    let output = Command::new(claudeless_bin())
        .args(["-p", "--permission-mode", "invalid", "hello"])
        .output()
        .expect("Failed to run claudeless");

    assert!(!output.status.success(), "Expected failure: {:?}", output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid"),
        "Expected stderr to contain 'invalid': {}",
        stderr
    );
}

#[test]
fn test_default_permission_mode() {
    // Without --permission-mode flag, should use default mode
    let output = Command::new(claudeless_bin())
        .args(["-p", "hello"])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "Expected success: {:?}", output);
}

mod bypass_validation {
    use super::*;

    #[test]
    fn test_bypass_without_allow_fails() {
        let output = Command::new(claudeless_bin())
            .args(["-p", "--dangerously-skip-permissions", "hello"])
            .output()
            .expect("Failed to run claudeless");

        assert!(!output.status.success(), "Expected failure: {:?}", output);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("--allow-dangerously-skip-permissions"),
            "Expected stderr to contain '--allow-dangerously-skip-permissions': {}",
            stderr
        );
    }

    #[test]
    fn test_bypass_with_allow_succeeds() {
        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--allow-dangerously-skip-permissions",
                "--dangerously-skip-permissions",
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_allow_without_bypass_is_noop() {
        let output = Command::new(claudeless_bin())
            .args(["-p", "--allow-dangerously-skip-permissions", "hello"])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_bypass_via_env_vars() {
        let output = Command::new(claudeless_bin())
            .env("CLAUDE_ALLOW_DANGEROUSLY_SKIP_PERMISSIONS", "true")
            .env("CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS", "true")
            .args(["-p", "hello"])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_bypass_env_without_allow_env_fails() {
        let output = Command::new(claudeless_bin())
            .env("CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS", "true")
            .args(["-p", "hello"])
            .output()
            .expect("Failed to run claudeless");

        assert!(!output.status.success(), "Expected failure: {:?}", output);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("--allow-dangerously-skip-permissions"),
            "Expected stderr to contain '--allow-dangerously-skip-permissions': {}",
            stderr
        );
    }

    #[test]
    fn test_bypass_error_mentions_sandbox() {
        let output = Command::new(claudeless_bin())
            .args(["-p", "--dangerously-skip-permissions", "hello"])
            .output()
            .expect("Failed to run claudeless");

        assert!(!output.status.success(), "Expected failure: {:?}", output);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("sandbox"),
            "Expected stderr to contain 'sandbox': {}",
            stderr
        );
    }
}

mod permission_mode_combinations {
    use super::*;

    #[test]
    fn test_bypass_flag_with_permission_mode() {
        // Both bypass flags and --permission-mode bypass-permissions should work together
        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--allow-dangerously-skip-permissions",
                "--dangerously-skip-permissions",
                "--permission-mode",
                "bypass-permissions",
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_accept_edits_mode() {
        let output = Command::new(claudeless_bin())
            .args(["-p", "--permission-mode", "accept-edits", "hello"])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_plan_mode() {
        let output = Command::new(claudeless_bin())
            .args(["-p", "--permission-mode", "plan", "hello"])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }
}
