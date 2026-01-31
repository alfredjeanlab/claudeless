// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for new scenario fields.
//!
//! Tests verify that all configuration fields specified in the scenario format
//! parse correctly and affect simulator behavior as expected.

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
// Session Identity Fields
// =============================================================================

mod session_identity {
    use super::*;

    #[test]
    fn test_default_model_field_parses() {
        let scenario = write_scenario(
            r#"
            name = "test"
            default_model = "claude-opus-4-20250514"
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_claude_version_field_parses() {
        let scenario = write_scenario(
            r#"
            name = "test"
            claude_version = "2.2.0"
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_user_name_field_parses() {
        let scenario = write_scenario(
            r#"
            name = "test"
            user_name = "TestUser"
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_session_id_field_parses() {
        let scenario = write_scenario(
            r#"
            name = "test"
            session_id = "550e8400-e29b-41d4-a716-446655440000"
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_project_path_field_parses() {
        let scenario = write_scenario(
            r#"
            name = "test"
            project_path = "/Users/test/myproject"
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_session_id_with_valid_uuid_parses() {
        // Verify multiple valid UUID formats are accepted
        for uuid in [
            "550e8400-e29b-41d4-a716-446655440000",
            "00000000-0000-0000-0000-000000000001",
            "ffffffff-ffff-ffff-ffff-ffffffffffff",
        ] {
            let scenario = write_scenario(&format!(
                r#"
                name = "test"
                session_id = "{}"
                [[responses]]
                pattern = {{ type = "any" }}
                response = "ok"
                "#,
                uuid
            ));

            let output = Command::new(claudeless_bin())
                .args([
                    "--scenario",
                    scenario.path().to_str().unwrap(),
                    "-p",
                    "test",
                ])
                .output()
                .expect("Failed to run claudeless");

            assert!(
                output.status.success(),
                "UUID {} should be valid: {:?}",
                uuid,
                output
            );
        }
    }
}

// =============================================================================
// Timing Fields
// =============================================================================

mod timing {
    use super::*;

    #[test]
    fn test_launch_timestamp_field_parses() {
        let scenario = write_scenario(
            r#"
            name = "test"
            launch_timestamp = "2025-01-15T10:30:00Z"
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_launch_timestamp_with_timezone_offset() {
        let scenario = write_scenario(
            r#"
            name = "test"
            launch_timestamp = "2025-01-15T10:30:00-08:00"
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }
}

// =============================================================================
// Environment Fields
// =============================================================================

mod environment {
    use super::*;

    #[test]
    fn test_working_directory_field_parses() {
        let scenario = write_scenario(
            r#"
            name = "test"
            working_directory = "/Users/test/myproject"
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_trusted_field_parses() {
        let scenario = write_scenario(
            r#"
            name = "test"
            trusted = true
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_permission_mode_default() {
        let scenario = write_scenario(
            r#"
            name = "test"
            permission_mode = "default"
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_permission_mode_plan() {
        let scenario = write_scenario(
            r#"
            name = "test"
            permission_mode = "plan"
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_permission_mode_bypass() {
        let scenario = write_scenario(
            r#"
            name = "test"
            permission_mode = "bypass-permissions"
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

        assert!(output.status.success(), "Expected success: {:?}", output);
    }
}

// =============================================================================
// Per-Tool Configuration
// =============================================================================

mod tool_config {
    use super::*;

    #[test]
    fn test_per_tool_auto_approve_parses() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"

            [tool_execution]
            mode = "live"

            [tool_execution.tools.Bash]
            auto_approve = true
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
    }

    #[test]
    fn test_per_tool_result_parses() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"

            [tool_execution]
            mode = "mock"

            [tool_execution.tools.Read]
            result = "canned file contents"
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
    }

    #[test]
    fn test_per_tool_error_parses() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"

            [tool_execution]
            mode = "mock"

            [tool_execution.tools.Write]
            error = "Permission denied"
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
    }

    #[test]
    fn test_multiple_tool_configs() {
        let scenario = write_scenario(
            r#"
            name = "test"
            [[responses]]
            pattern = { type = "any" }
            response = "ok"

            [tool_execution]
            mode = "live"

            [tool_execution.tools.Bash]
            auto_approve = true

            [tool_execution.tools.Read]
            auto_approve = true
            result = "file contents"

            [tool_execution.tools.Write]
            auto_approve = false
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
    }
}

// =============================================================================
// Full-Featured Scenario
// =============================================================================

mod full_featured {
    use super::*;

    #[test]
    fn test_all_fields_together() {
        let scenario = write_scenario(
            r#"
            name = "full-featured-test"

            # Session identity
            default_model = "claude-sonnet-4-20250514"
            claude_version = "2.1.12"
            user_name = "Alfred"
            session_id = "550e8400-e29b-41d4-a716-446655440000"
            project_path = "/Users/test/myproject"

            # Timing
            launch_timestamp = "2025-01-15T10:30:00Z"

            # Environment
            working_directory = "/Users/test/myproject"
            trusted = true
            permission_mode = "default"

            # Default response
            [default_response]
            text = "Default response"
            delay_ms = 10

            # Responses
            [[responses]]
            pattern = { type = "contains", text = "hello" }
            response = "Hello!"

            # Tool execution
            [tool_execution]
            mode = "live"

            [tool_execution.tools.Bash]
            auto_approve = true
            "#,
        );

        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario.path().to_str().unwrap(),
                "-p",
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Hello!"),
            "Expected stdout to contain 'Hello!': {}",
            stdout
        );
    }

    #[test]
    fn test_deterministic_scenario_runs_consistently() {
        // Verify a deterministic scenario with all fixed values parses and runs
        let scenario = write_scenario(
            r#"
            name = "deterministic"
            session_id = "00000000-0000-0000-0000-000000000001"
            launch_timestamp = "2025-01-01T00:00:00Z"
            user_name = "TestUser"
            trusted = true
            [[responses]]
            pattern = { type = "any" }
            response = "Test response"
            "#,
        );

        // Run twice and verify consistent output
        let run = || -> String {
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
            String::from_utf8_lossy(&output.stdout).to_string()
        };

        let output1 = run();
        let output2 = run();

        // Both runs should produce identical text output
        assert_eq!(
            output1, output2,
            "Deterministic scenario should produce identical output"
        );
        assert!(
            output1.contains("Test response"),
            "Output should contain expected response"
        );
    }
}

// =============================================================================
// Example Scenario Files
// =============================================================================

mod example_scenarios {
    use super::*;

    fn scenarios_dir() -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        // scenarios/ is at workspace root, not crate root
        PathBuf::from(manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("scenarios")
    }

    #[test]
    fn test_full_featured_scenario_loads() {
        let scenario_path = scenarios_dir().join("full-featured.toml");
        if !scenario_path.exists() {
            panic!("Example scenario not found: {}", scenario_path.display());
        }

        let output = Command::new(claudeless_bin())
            .args(["--scenario", scenario_path.to_str().unwrap(), "-p", "hello"])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_deterministic_scenario_loads() {
        let scenario_path = scenarios_dir().join("deterministic.toml");
        if !scenario_path.exists() {
            panic!("Example scenario not found: {}", scenario_path.display());
        }

        let output = Command::new(claudeless_bin())
            .args(["--scenario", scenario_path.to_str().unwrap(), "-p", "test"])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }
}
