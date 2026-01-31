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

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use tempfile::NamedTempFile;

fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

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
    #[ignore] // TODO(implement): stream-json with -p should require --verbose
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
    ///
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
}

// =============================================================================
// Delay Tests
// =============================================================================

mod delay {
    use super::*;

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
    ///
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
    ///
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
    ///
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
    ///
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
}

// =============================================================================
// Exit Code Tests
// =============================================================================

mod exit_codes {
    use super::*;

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
}
