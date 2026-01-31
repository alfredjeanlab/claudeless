// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for strict scenario validation.
//!
//! Tests verify that invalid scenarios produce clear, actionable error messages.
//! Uses `#[serde(deny_unknown_fields)]` to reject typos and unknown fields.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("unknown_field") || stderr.contains("unknown field"),
            "Expected stderr to mention 'unknown_field' or 'unknown field': {}",
            stderr
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("defualt_model") || stderr.contains("unknown field"),
            "Expected stderr to mention 'defualt_model' or 'unknown field': {}",
            stderr
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("moode") || stderr.contains("unknown field"),
            "Expected stderr to mention 'moode' or 'unknown field': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("auto_aprove") || stderr.contains("unknown field"),
            "Expected stderr to mention 'auto_aprove' or 'unknown field': {}",
            stderr
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("session_id")
                && (stderr.contains("UUID") || stderr.contains("uuid")),
            "Expected stderr to mention 'session_id' and 'UUID' or 'uuid': {}",
            stderr
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("session_id"),
            "Expected stderr to mention 'session_id': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("session_id"),
            "Expected stderr to mention 'session_id': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("launch_timestamp")
                && (stderr.contains("ISO 8601") || stderr.contains("timestamp")),
            "Expected stderr to mention 'launch_timestamp' and 'ISO 8601' or 'timestamp': {}",
            stderr
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("launch_timestamp"),
            "Expected stderr to mention 'launch_timestamp': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("launch_timestamp"),
            "Expected stderr to mention 'launch_timestamp': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("permission_mode"),
            "Expected stderr to mention 'permission_mode': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("permission_mode"),
            "Expected stderr to mention 'permission_mode': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("regex") || stderr.contains("pattern"),
            "Expected stderr to mention 'regex' or 'pattern': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("glob") || stderr.contains("pattern"),
            "Expected stderr to mention 'glob' or 'pattern': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("trusted") || stderr.contains("boolean"),
            "Expected stderr to mention 'trusted' or 'boolean': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("delay_ms") || stderr.contains("integer"),
            "Expected stderr to mention 'delay_ms' or 'integer': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("session_id"),
            "Expected stderr to mention 'session_id': {}",
            stderr
        );
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("unknown_field") || stderr.contains("unknown field"),
            "Expected stderr to mention 'unknown_field' or 'unknown field': {}",
            stderr
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
        let stderr = String::from_utf8_lossy(&output.stderr);
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Error should give guidance on valid values
        assert!(
            stderr.contains("permission_mode"),
            "Error should mention the field name. Got: {}",
            stderr
        );
    }
}
