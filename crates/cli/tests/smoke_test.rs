// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Smoke tests validating claudeless behavior matches real Claude CLI.
//!
//! These tests document the expected behavior based on real Claude CLI v2.1.12.
//! Tests that fail indicate areas where claudeless differs from real Claude
//! and need to be fixed.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

#![allow(deprecated)] // Command::cargo_bin is deprecated but still functional

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use std::time::Instant;
use tempfile::NamedTempFile;

fn write_scenario(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

// =============================================================================
// Basic Output Format Tests
// =============================================================================

mod text_output {
    use super::*;

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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "hello world",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello! How can I help you today?"));
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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        let output = cmd
            .args([
                "--scenario",
                scenario.path().to_str().unwrap(),
                "-p",
                "test",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let stdout = String::from_utf8_lossy(&output);
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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test",
        ])
        .assert()
        .code(0);
    }
}

/// JSON output format tests.
///
/// Real Claude CLI v2.1.12 uses a result wrapper format:
/// ```json
/// {
///   "type": "result",
///   "subtype": "success",
///   "result": "response text",
///   "session_id": "...",
///   "cost_usd": 0.001,
///   ...
/// }
/// ```
mod json_output {
    use super::*;

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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        let output = cmd
            .args([
                "--scenario",
                scenario.path().to_str().unwrap(),
                "--output-format",
                "json",
                "-p",
                "test",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let stdout = String::from_utf8_lossy(&output);
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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        let output = cmd
            .args([
                "--scenario",
                scenario.path().to_str().unwrap(),
                "--output-format",
                "json",
                "-p",
                "test",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let stdout = String::from_utf8_lossy(&output);
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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--output-format",
            "json",
            "-p",
            "test",
        ])
        .assert()
        .code(0);
    }
}

/// Stream JSON output format tests.
///
/// Real Claude CLI v2.1.12 uses this event sequence:
/// 1. {"type":"system","subtype":"init",...} - initialization with tools, model, settings
/// 2. {"type":"assistant","message":{...},...} - assistant message with content
/// 3. {"type":"result","subtype":"success",...} - final result summary
///
/// Note: Real Claude requires --verbose with --output-format=stream-json and -p
mod stream_json_output {
    use super::*;

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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        let output = cmd
            .args([
                "--scenario",
                scenario.path().to_str().unwrap(),
                "--output-format",
                "stream-json",
                "-p",
                "test",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let stdout = String::from_utf8_lossy(&output);

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
    ///
    /// FIXME: epic-05x-fix-cli - enable after fixing output format
    #[test]
    #[ignore]
    fn test_stream_json_starts_with_system_init() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"
            "#,
        );

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        let output = cmd
            .args([
                "--scenario",
                scenario.path().to_str().unwrap(),
                "--output-format",
                "stream-json",
                "-p",
                "test",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let stdout = String::from_utf8_lossy(&output);
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
    ///
    /// FIXME: epic-05x-fix-cli - enable after fixing output format
    #[test]
    #[ignore]
    fn test_stream_json_ends_with_result() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"
            "#,
        );

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        let output = cmd
            .args([
                "--scenario",
                scenario.path().to_str().unwrap(),
                "--output-format",
                "stream-json",
                "-p",
                "test",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let stdout = String::from_utf8_lossy(&output);
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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--output-format",
            "stream-json",
            "-p",
            "test",
        ])
        .assert()
        .code(0);
    }
}

// =============================================================================
// Failure Mode Tests
// =============================================================================

mod failure_modes {
    use super::*;

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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test",
        ])
        .assert()
        .failure()
        .code(1);

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() >= 50,
            "Timeout should delay at least 50ms"
        );
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    #[test]
    fn test_auth_error_exit_code() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["--failure", "auth-error", "-p", "test"])
            .assert()
            .failure()
            .code(1);
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    #[test]
    fn test_rate_limit_exit_code() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["--failure", "rate-limit", "-p", "test"])
            .assert()
            .failure()
            .code(1);
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    #[test]
    fn test_network_unreachable_exit_code() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["--failure", "network-unreachable", "-p", "test"])
            .assert()
            .failure()
            .code(1);
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    #[test]
    fn test_out_of_credits_exit_code() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["--failure", "out-of-credits", "-p", "test"])
            .assert()
            .failure()
            .code(1);
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    #[test]
    fn test_partial_response_exit_code_2() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["--failure", "partial-response", "-p", "test"])
            .assert()
            .code(2); // Partial response uses exit code 2
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    #[test]
    fn test_malformed_json_exit_code() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["--failure", "malformed-json", "-p", "test"])
            .assert()
            .success(); // Malformed JSON still exits with 0
    }
}

// =============================================================================
// Delay Tests
// =============================================================================

mod delay {
    use super::*;

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    #[test]
    fn test_delay_ms_adds_response_delay() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "delayed"
            "#,
        );

        let start = Instant::now();

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--delay-ms",
            "200",
            "-p",
            "test",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("delayed"));

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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("delayed via scenario"));

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() >= 150,
            "Response should be delayed by at least 150ms, but took {}ms",
            elapsed.as_millis()
        );
    }
}

// =============================================================================
// Unsupported Flag Tests
//
// Real Claude CLI v2.1.12 supports these flags. claudeless should either
// implement them or safely ignore them for compatibility.
// =============================================================================

mod unsupported_flags {
    use super::*;

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    ///
    /// Real Claude supports --add-dir. claudeless should accept it (even if ignored).
    ///
    /// FIXME: epic-05x-fix-cli - enable after adding missing CLI flags
    #[test]
    #[ignore]
    fn test_add_dir_flag_should_be_accepted() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"
            "#,
        );

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--add-dir",
            "/tmp",
            "-p",
            "test",
        ])
        .assert()
        .success(); // Should accept the flag, not error
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    ///
    /// Real Claude supports --json-schema. claudeless should accept it (even if ignored).
    ///
    /// FIXME: epic-05x-fix-cli - enable after adding missing CLI flags
    #[test]
    #[ignore]
    fn test_json_schema_flag_should_be_accepted() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"
            "#,
        );

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--json-schema",
            r#"{"type":"object"}"#,
            "-p",
            "test",
        ])
        .assert()
        .success();
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    ///
    /// Real Claude supports --tools. claudeless should accept it (even if ignored).
    ///
    /// FIXME: epic-05x-fix-cli - enable after adding missing CLI flags
    #[test]
    #[ignore]
    fn test_tools_flag_should_be_accepted() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"
            "#,
        );

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--tools",
            "Bash,Edit",
            "-p",
            "test",
        ])
        .assert()
        .success();
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    ///
    /// Real Claude supports --agent. claudeless should accept it (even if ignored).
    ///
    /// FIXME: epic-05x-fix-cli - enable after adding missing CLI flags
    #[test]
    #[ignore]
    fn test_agent_flag_should_be_accepted() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"
            "#,
        );

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--agent",
            "custom-agent",
            "-p",
            "test",
        ])
        .assert()
        .success();
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    ///
    /// Real Claude supports --append-system-prompt. claudeless should accept it.
    ///
    /// FIXME: epic-05x-fix-cli - enable after adding missing CLI flags
    #[test]
    #[ignore]
    fn test_append_system_prompt_flag_should_be_accepted() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"
            "#,
        );

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--append-system-prompt",
            "extra instructions",
            "-p",
            "test",
        ])
        .assert()
        .success();
    }
}

// =============================================================================
// Exit Code Tests
// =============================================================================

mod exit_codes {
    use super::*;

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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test",
        ])
        .assert()
        .code(0);
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    #[test]
    fn test_error_exit_code_1() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["--failure", "auth-error", "-p", "test"])
            .assert()
            .code(1);
    }

    /// Behavior observed with: claude --version 2.1.12 (Claude Code)
    #[test]
    fn test_partial_exit_code_2() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["--failure", "partial-response", "-p", "test"])
            .assert()
            .code(2);
    }
}

// =============================================================================
// Model Flag Tests
// =============================================================================

mod model_flag {
    use super::*;

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
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--model",
            "haiku",
            "-p",
            "test",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("response regardless of model"));
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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "--model",
            "claude-haiku-4-5-20251001",
            "-p",
            "test",
        ])
        .assert()
        .success();
    }
}
