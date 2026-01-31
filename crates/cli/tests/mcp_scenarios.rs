// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for MCP scenario files.
//!
//! Tests the MCP test scenarios in `scenarios/mcp-test-*.toml` to verify
//! that claudeless correctly handles MCP tool simulations.

use std::path::PathBuf;
use std::process::Command;

fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

fn scenario_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

// =============================================================================
// MCP Init Scenario Tests
// =============================================================================

mod mcp_init_scenario {
    use super::*;

    /// Test: mcp-test-init.toml lists MCP tools with correct naming convention.
    ///
    /// Real Claude CLI uses `mcp__<server>__<tool>` format for MCP tools.
    #[test]
    fn test_init_scenario_lists_mcp_tools() {
        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-init.toml").to_str().unwrap(),
                "-p",
                "list your available tools",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("mcp__filesystem__read_file"),
            "Expected stdout to contain 'mcp__filesystem__read_file': {}",
            stdout
        );
        assert!(
            stdout.contains("mcp__filesystem__write_file"),
            "Expected stdout to contain 'mcp__filesystem__write_file': {}",
            stdout
        );
        assert!(
            stdout.contains("mcp__filesystem__list_directory"),
            "Expected stdout to contain 'mcp__filesystem__list_directory': {}",
            stdout
        );
    }

    /// Test: Default response when prompt doesn't match tool listing pattern.
    #[test]
    fn test_init_scenario_default_response() {
        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-init.toml").to_str().unwrap(),
                "-p",
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("ready to help"),
            "Expected stdout to contain 'ready to help': {}",
            stdout
        );
    }
}

// =============================================================================
// MCP Read Scenario Tests
// =============================================================================

mod mcp_read_scenario {
    use super::*;

    /// Test: mcp-test-read.toml returns canned file content in mock mode.
    #[test]
    fn test_read_scenario_mock_mode() {
        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-read.toml").to_str().unwrap(),
                "--tool-mode",
                "mock",
                "--output-format",
                "stream-json",
                "-p",
                "read sample.txt",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should contain response text
        assert!(
            stdout.contains("I'll read the file for you"),
            "Should contain response text"
        );

        // Should contain tool result with canned content
        assert!(
            stdout.contains("hello world"),
            "Should contain canned file content 'hello world'"
        );

        // Should have tool_result type
        assert!(
            stdout.contains("tool_result"),
            "Should contain tool_result event"
        );
    }

    /// Test: mcp-test-read.toml response text is correct.
    #[test]
    fn test_read_scenario_response_text() {
        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-read.toml").to_str().unwrap(),
                "-p",
                "read the file",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("I'll read the file for you"),
            "Expected stdout to contain 'I'll read the file for you': {}",
            stdout
        );
    }
}

// =============================================================================
// MCP Write Scenario Tests
// =============================================================================

mod mcp_write_scenario {
    use super::*;

    /// Test: mcp-test-write.toml returns success message in mock mode.
    #[test]
    fn test_write_scenario_mock_mode() {
        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-write.toml").to_str().unwrap(),
                "--tool-mode",
                "mock",
                "--output-format",
                "stream-json",
                "-p",
                "create a file",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should contain response text
        assert!(
            stdout.contains("I'll create that file for you"),
            "Should contain response text"
        );

        // Should contain tool result with success message
        assert!(
            stdout.contains("Successfully wrote"),
            "Should contain success message"
        );
    }

    /// Test: mcp-test-write.toml response text is correct.
    #[test]
    fn test_write_scenario_response_text() {
        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-write.toml").to_str().unwrap(),
                "-p",
                "create a new file",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("I'll create that file for you"),
            "Expected stdout to contain 'I'll create that file for you': {}",
            stdout
        );
    }
}

// =============================================================================
// MCP List Directory Scenario Tests
// =============================================================================

mod mcp_list_scenario {
    use super::*;

    /// Test: mcp-test-list.toml returns file listing in mock mode.
    #[test]
    fn test_list_scenario_mock_mode() {
        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-list.toml").to_str().unwrap(),
                "--tool-mode",
                "mock",
                "--output-format",
                "stream-json",
                "-p",
                "list the files",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should contain response text
        assert!(
            stdout.contains("Here are the files"),
            "Should contain response text"
        );

        // Should contain tool result with file listing
        assert!(
            stdout.contains("sample.txt"),
            "Should contain sample.txt in listing"
        );
        assert!(
            stdout.contains("test-output.txt"),
            "Should contain test-output.txt in listing"
        );
    }

    /// Test: mcp-test-list.toml response text is correct.
    #[test]
    fn test_list_scenario_response_text() {
        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-list.toml").to_str().unwrap(),
                "-p",
                "list all files",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Here are the files"),
            "Expected stdout to contain 'Here are the files': {}",
            stdout
        );
    }
}

// =============================================================================
// MCP Live Execution Scenario Tests
// =============================================================================

mod mcp_live_scenarios {
    use super::*;

    /// Test: mcp-test-read-live.toml (qualified tool name) routes to MCP executor.
    ///
    /// Verifies that qualified MCP tool names (mcp__filesystem__read_file) are
    /// correctly routed to MCP servers instead of failing with "Unknown built-in tool".
    ///
    /// Note: This test requires an MCP config to have an MCP executor available.
    /// Without MCP config, qualified names fall through to builtin (expected behavior).
    #[test]
    fn test_read_live_scenario_qualified_name_routes_to_mcp() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a minimal MCP config that won't actually start a server
        // but will register the MCP executor
        let mut mcp_config = NamedTempFile::new().unwrap();
        mcp_config
            .write_all(br#"{"mcpServers":{"filesystem":{"command":"echo","args":["test"]}}}"#)
            .unwrap();
        mcp_config.flush().unwrap();

        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-read-live.toml").to_str().unwrap(),
                "--mcp-config",
                mcp_config.path().to_str().unwrap(),
                "--tool-mode",
                "live",
                "--output-format",
                "stream-json",
                "-p",
                "read the file",
            ])
            .output()
            .expect("Failed to run claudeless");

        // Command succeeds but tool call may return error
        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should NOT contain "Unknown built-in tool" error - that was the old bug
        assert!(
            !stdout.contains("Unknown built-in tool"),
            "Qualified MCP tool name should route to MCP, not fail as unknown builtin. Got: {}",
            stdout
        );

        // Should contain a tool_result (success or MCP-level error)
        assert!(
            stdout.contains("tool_result"),
            "Should produce tool_result event"
        );
    }

    /// Test: mcp-test-read-raw.toml (raw tool name) loads correctly.
    ///
    /// Note: This test doesn't verify actual MCP execution because that would
    /// require a running MCP server. It just verifies the scenario loads.
    #[test]
    fn test_read_raw_scenario_loads() {
        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-read-raw.toml").to_str().unwrap(),
                "--tool-mode",
                "disabled", // Don't try to execute
                "-p",
                "read the file",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("I'll read the file for you"),
            "Expected stdout to contain 'I'll read the file for you': {}",
            stdout
        );
    }
}

// =============================================================================
// Tool Mode Behavior Tests
// =============================================================================

mod tool_mode_behavior {
    use super::*;

    /// Test: --tool-mode disabled returns response without tool results.
    #[test]
    fn test_disabled_mode_no_tool_results() {
        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-read.toml").to_str().unwrap(),
                "--tool-mode",
                "disabled",
                "--output-format",
                "stream-json",
                "-p",
                "read file",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should contain response text
        assert!(stdout.contains("I'll read the file"));

        // Should NOT contain tool_result (disabled mode)
        assert!(
            !stdout.contains("tool_result"),
            "Disabled mode should not produce tool_result events"
        );
    }

    /// Test: --tool-mode mock returns tool results from scenario.
    #[test]
    fn test_mock_mode_returns_canned_results() {
        let output = Command::new(claudeless_bin())
            .args([
                "--scenario",
                scenario_path("mcp-test-read.toml").to_str().unwrap(),
                "--tool-mode",
                "mock",
                "--output-format",
                "stream-json",
                "-p",
                "read file",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should contain tool_result with canned content
        assert!(
            stdout.contains("tool_result"),
            "Mock mode should produce tool_result events"
        );
        assert!(
            stdout.contains("hello world"),
            "Mock mode should return canned result"
        );
    }
}

// =============================================================================
// Scenario File Validation Tests
// =============================================================================

mod scenario_validation {
    use super::*;

    /// Test: All MCP scenarios load without parse errors.
    #[test]
    fn test_all_mcp_scenarios_load() {
        let scenarios = [
            "mcp-test-init.toml",
            "mcp-test-read.toml",
            "mcp-test-write.toml",
            "mcp-test-list.toml",
            "mcp-test-read-live.toml",
            "mcp-test-read-raw.toml",
        ];

        for scenario in scenarios {
            let output = Command::new(claudeless_bin())
                .args([
                    "--scenario",
                    scenario_path(scenario).to_str().unwrap(),
                    "-p",
                    "test",
                ])
                .output()
                .expect("Failed to run claudeless");

            assert!(
                output.status.success(),
                "Scenario {} should load successfully: {:?}",
                scenario,
                output
            );
        }
    }

    /// Test: MCP scenarios contain valid tool call definitions.
    #[test]
    fn test_mcp_scenarios_have_tool_calls() {
        // Scenarios with tool_calls should work with mock mode
        let scenarios_with_tools = [
            ("mcp-test-read.toml", "read"),
            ("mcp-test-write.toml", "create"),
            ("mcp-test-list.toml", "list"),
        ];

        for (scenario, trigger) in scenarios_with_tools {
            let output = Command::new(claudeless_bin())
                .args([
                    "--scenario",
                    scenario_path(scenario).to_str().unwrap(),
                    "--tool-mode",
                    "mock",
                    "--output-format",
                    "stream-json",
                    "-p",
                    trigger,
                ])
                .output()
                .expect("Failed to run claudeless");

            assert!(
                output.status.success(),
                "Scenario {} should succeed: {:?}",
                scenario,
                output
            );
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains("tool_result"),
                "Scenario {} should produce tool_result for trigger '{}'",
                scenario,
                trigger
            );
        }
    }
}
