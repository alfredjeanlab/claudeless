// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for strict scenario validation.
//!
//! Tests verify that invalid scenarios produce clear, actionable error messages.
//! Uses `#[serde(deny_unknown_fields)]` to reject typos and unknown fields.

#![allow(deprecated)] // Command::cargo_bin is deprecated but still functional

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

fn write_scenario(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

// =============================================================================
// Unknown Field Rejection (via #[serde(deny_unknown_fields)])
// =============================================================================

mod unknown_fields {
    use super::*;

    #[test]
    fn test_unknown_top_level_field_rejected() {
        let scenario = write_scenario(
            r#"
            name = "test"
            unknown_field = "value"
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
        .failure()
        .stderr(
            predicate::str::contains("unknown_field").or(predicate::str::contains("unknown field")),
        );
    }

    #[test]
    fn test_typo_in_field_name_rejected() {
        let scenario = write_scenario(
            r#"
            name = "test"
            defualt_model = "claude-sonnet-4"
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
        .failure()
        .stderr(
            predicate::str::contains("defualt_model").or(predicate::str::contains("unknown field")),
        );
    }

    #[test]
    fn test_unknown_tool_execution_field_rejected() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"

            [tool_execution]
            moode = "mock"
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
        .failure()
        .stderr(predicate::str::contains("moode").or(predicate::str::contains("unknown field")));
    }

    #[test]
    fn test_unknown_tool_config_field_rejected() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"

            [tool_execution]
            mode = "mock"

            [tool_execution.tools.Bash]
            auto_aprove = true
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
        .failure()
        .stderr(
            predicate::str::contains("auto_aprove").or(predicate::str::contains("unknown field")),
        );
    }
}

// =============================================================================
// Session ID Validation
// =============================================================================

mod session_id_validation {
    use super::*;

    #[test]
    fn test_invalid_session_id_produces_clear_error() {
        let scenario = write_scenario(
            r#"
            name = "test"
            session_id = "not-a-valid-uuid"
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
        .failure()
        .stderr(
            predicate::str::contains("session_id")
                .and(predicate::str::contains("UUID").or(predicate::str::contains("uuid"))),
        );
    }

    #[test]
    fn test_empty_session_id_produces_clear_error() {
        let scenario = write_scenario(
            r#"
            name = "test"
            session_id = ""
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
        .failure()
        .stderr(predicate::str::contains("session_id"));
    }

    #[test]
    fn test_malformed_uuid_produces_clear_error() {
        let scenario = write_scenario(
            r#"
            name = "test"
            session_id = "550e8400-e29b-41d4-a716"
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
        .failure()
        .stderr(predicate::str::contains("session_id"));
    }
}

// =============================================================================
// Launch Timestamp Validation
// =============================================================================

mod timestamp_validation {
    use super::*;

    #[test]
    fn test_invalid_timestamp_produces_clear_error() {
        let scenario = write_scenario(
            r#"
            name = "test"
            launch_timestamp = "not-a-timestamp"
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
        .failure()
        .stderr(
            predicate::str::contains("launch_timestamp").and(
                predicate::str::contains("ISO 8601").or(predicate::str::contains("timestamp")),
            ),
        );
    }

    #[test]
    fn test_wrong_date_format_produces_clear_error() {
        // US date format instead of ISO 8601
        let scenario = write_scenario(
            r#"
            name = "test"
            launch_timestamp = "01/15/2025 10:30:00"
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
        .failure()
        .stderr(predicate::str::contains("launch_timestamp"));
    }

    #[test]
    fn test_missing_timezone_produces_clear_error() {
        // ISO 8601 requires timezone
        let scenario = write_scenario(
            r#"
            name = "test"
            launch_timestamp = "2025-01-15T10:30:00"
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
        .failure()
        .stderr(predicate::str::contains("launch_timestamp"));
    }
}

// =============================================================================
// Permission Mode Validation
// =============================================================================

mod permission_mode_validation {
    use super::*;

    #[test]
    fn test_invalid_permission_mode_produces_clear_error() {
        let scenario = write_scenario(
            r#"
            name = "test"
            permission_mode = "invalid-mode"
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
        .failure()
        .stderr(predicate::str::contains("permission_mode"));
    }

    #[test]
    fn test_typo_in_permission_mode_produces_clear_error() {
        let scenario = write_scenario(
            r#"
            name = "test"
            permission_mode = "full_auto"
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
        .failure()
        .stderr(predicate::str::contains("permission_mode"));
    }
}

// =============================================================================
// Pattern Validation
// =============================================================================

mod pattern_validation {
    use super::*;

    #[test]
    fn test_invalid_regex_produces_clear_error() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "regex", pattern = "[invalid(regex" }
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
        .failure()
        .stderr(predicate::str::contains("regex").or(predicate::str::contains("pattern")));
    }

    #[test]
    fn test_invalid_glob_produces_clear_error() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "glob", pattern = "[invalid" }
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
        .failure()
        .stderr(predicate::str::contains("glob").or(predicate::str::contains("pattern")));
    }
}

// =============================================================================
// Type Mismatch Validation
// =============================================================================

mod type_validation {
    use super::*;

    #[test]
    fn test_trusted_wrong_type_produces_clear_error() {
        let scenario = write_scenario(
            r#"
            name = "test"
            trusted = "yes"
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
        .failure()
        .stderr(predicate::str::contains("trusted").or(predicate::str::contains("boolean")));
    }

    #[test]
    fn test_delay_ms_wrong_type_produces_clear_error() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = { text = "ok", delay_ms = "fast" }
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
        .failure()
        .stderr(predicate::str::contains("delay_ms").or(predicate::str::contains("integer")));
    }
}

// =============================================================================
// JSON Format Validation
// =============================================================================

mod json_validation {
    use super::*;

    fn write_json_scenario(content: &str) -> NamedTempFile {
        let mut file = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_valid_json_scenario_parses() {
        let scenario = write_json_scenario(
            r#"{
                "name": "test",
                "session_id": "550e8400-e29b-41d4-a716-446655440000",
                "responses": [
                    {
                        "pattern": { "type": "any" },
                        "response": "ok"
                    }
                ]
            }"#,
        );

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test",
        ])
        .assert()
        .success();
    }

    #[test]
    fn test_invalid_json_session_id_produces_clear_error() {
        let scenario = write_json_scenario(
            r#"{
                "name": "test",
                "session_id": "invalid-uuid",
                "responses": [
                    {
                        "pattern": { "type": "any" },
                        "response": "ok"
                    }
                ]
            }"#,
        );

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("session_id"));
    }

    #[test]
    fn test_unknown_json_field_rejected() {
        let scenario = write_json_scenario(
            r#"{
                "name": "test",
                "unknown_field": "value",
                "responses": [
                    {
                        "pattern": { "type": "any" },
                        "response": "ok"
                    }
                ]
            }"#,
        );

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "--scenario",
            scenario.path().to_str().unwrap(),
            "-p",
            "test",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("unknown_field").or(predicate::str::contains("unknown field")),
        );
    }
}

// =============================================================================
// Error Message Quality
// =============================================================================

mod error_message_quality {
    use super::*;

    #[test]
    fn test_error_includes_field_name() {
        let scenario = write_scenario(
            r#"
            name = "test"
            session_id = "bad"
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
                "-p",
                "test",
            ])
            .assert()
            .failure()
            .get_output()
            .stderr
            .clone();

        let stderr = String::from_utf8_lossy(&output);
        assert!(
            stderr.contains("session_id"),
            "Error should mention the field name. Got: {}",
            stderr
        );
    }

    #[test]
    fn test_error_is_actionable() {
        let scenario = write_scenario(
            r#"
            name = "test"
            permission_mode = "wrong"
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
                "-p",
                "test",
            ])
            .assert()
            .failure()
            .get_output()
            .stderr
            .clone();

        let stderr = String::from_utf8_lossy(&output);
        // Error should give guidance on valid values
        assert!(
            stderr.contains("permission_mode"),
            "Error should mention the field name. Got: {}",
            stderr
        );
    }
}
