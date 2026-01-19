// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Integration tests for permission modes.

#![allow(deprecated)] // Command::cargo_bin is deprecated but still functional

use assert_cmd::Command;
use predicates::prelude::*;
use rstest::rstest;

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
    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    cmd.args(["-p", "--permission-mode", mode, prompt])
        .assert()
        .code(expected_exit);
}

#[test]
fn test_invalid_permission_mode_rejected() {
    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    cmd.args(["-p", "--permission-mode", "invalid", "hello"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn test_default_permission_mode() {
    // Without --permission-mode flag, should use default mode
    let mut cmd = Command::cargo_bin("claudeless").unwrap();
    cmd.args(["-p", "hello"]).assert().success();
}

mod bypass_validation {
    use super::*;

    #[test]
    fn test_bypass_without_allow_fails() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["-p", "--dangerously-skip-permissions", "hello"])
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "--allow-dangerously-skip-permissions",
            ));
    }

    #[test]
    fn test_bypass_with_allow_succeeds() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "-p",
            "--allow-dangerously-skip-permissions",
            "--dangerously-skip-permissions",
            "hello",
        ])
        .assert()
        .success();
    }

    #[test]
    fn test_allow_without_bypass_is_noop() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["-p", "--allow-dangerously-skip-permissions", "hello"])
            .assert()
            .success();
    }

    #[test]
    fn test_bypass_via_env_vars() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.env("CLAUDE_ALLOW_DANGEROUSLY_SKIP_PERMISSIONS", "true")
            .env("CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS", "true")
            .args(["-p", "hello"])
            .assert()
            .success();
    }

    #[test]
    fn test_bypass_env_without_allow_env_fails() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.env("CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS", "true")
            .args(["-p", "hello"])
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "--allow-dangerously-skip-permissions",
            ));
    }

    #[test]
    fn test_bypass_error_mentions_sandbox() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["-p", "--dangerously-skip-permissions", "hello"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("sandbox"));
    }
}

mod permission_mode_combinations {
    use super::*;

    #[test]
    fn test_bypass_flag_with_permission_mode() {
        // Both bypass flags and --permission-mode bypass-permissions should work together
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "-p",
            "--allow-dangerously-skip-permissions",
            "--dangerously-skip-permissions",
            "--permission-mode",
            "bypass-permissions",
            "hello",
        ])
        .assert()
        .success();
    }

    #[test]
    fn test_accept_edits_mode() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["-p", "--permission-mode", "accept-edits", "hello"])
            .assert()
            .success();
    }

    #[test]
    fn test_plan_mode() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["-p", "--permission-mode", "plan", "hello"])
            .assert()
            .success();
    }
}
