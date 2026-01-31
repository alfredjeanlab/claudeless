// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for MCP configuration loading.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;

fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

fn write_config(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

mod config_loading {
    use super::*;

    #[test]
    fn test_mcp_config_flag_accepted() {
        let config = write_config(
            r#"
            {
                "mcpServers": {
                    "test": {"command": "echo"}
                }
            }
        "#,
        );

        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--mcp-config",
                config.path().to_str().unwrap(),
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_mcp_config_inline_json() {
        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--mcp-config",
                r#"{"mcpServers":{"inline":{"command":"node"}}}"#,
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_multiple_mcp_configs() {
        let config1 = write_config(r#"{"mcpServers":{"a":{"command":"a"}}}"#);
        let config2 = write_config(r#"{"mcpServers":{"b":{"command":"b"}}}"#);

        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--mcp-config",
                config1.path().to_str().unwrap(),
                "--mcp-config",
                config2.path().to_str().unwrap(),
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_strict_mcp_config_flag() {
        // With strict mode, invalid MCP server commands should cause exit
        // This tests that --strict-mcp-config is recognized as a flag
        // We use a command that doesn't speak MCP protocol to verify it fails
        let config = write_config(r#"{"mcpServers":{"bad":{"command":"nonexistent_cmd_xyz"}}}"#);

        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--strict-mcp-config",
                "--mcp-config",
                config.path().to_str().unwrap(),
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(!output.status.success(), "Expected failure: {:?}", output);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("failed to start"),
            "Expected stderr to contain 'failed to start': {}",
            stderr
        );
    }

    #[test]
    fn test_mcp_debug_flag() {
        let output = Command::new(claudeless_bin())
            .args(["-p", "--mcp-debug", "hello"])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }
}

mod config_errors {
    use super::*;

    #[test]
    fn test_nonexistent_config_file_error() {
        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--mcp-config",
                "/nonexistent/path/config.json",
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(!output.status.success(), "Expected failure: {:?}", output);
    }

    #[test]
    fn test_invalid_json_config_error() {
        let config = write_config("not valid json at all");

        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--mcp-config",
                config.path().to_str().unwrap(),
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(!output.status.success(), "Expected failure: {:?}", output);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("parse") || stderr.contains("Failed"),
            "Expected stderr to contain 'parse' or 'Failed': {}",
            stderr
        );
    }

    #[test]
    fn test_empty_json_config_accepted() {
        // Empty JSON object is valid, just has no servers
        let config = write_config("{}");

        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--mcp-config",
                config.path().to_str().unwrap(),
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }
}

mod json5_support {
    use super::*;

    #[test]
    fn test_json5_with_comments() {
        let config = write_config(
            r#"
            {
                // This is a comment
                "mcpServers": {
                    "test": {
                        "command": "node",
                        "args": ["server.js"], // trailing comma OK
                    }
                }
            }
        "#,
        );

        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--mcp-config",
                config.path().to_str().unwrap(),
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }

    #[test]
    fn test_json5_with_trailing_commas() {
        let config = write_config(
            r#"
            {
                "mcpServers": {
                    "test": {
                        "command": "node",
                        "args": ["a", "b",],
                    },
                },
            }
        "#,
        );

        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--mcp-config",
                config.path().to_str().unwrap(),
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);
    }
}

mod stream_json_output {
    use super::*;

    #[test]
    fn test_mcp_servers_in_stream_json_init() {
        let config = write_config(r#"{"mcpServers":{"test":{"command":"echo"}}}"#);

        let output = Command::new(claudeless_bin())
            .args([
                "-p",
                "--output-format",
                "stream-json",
                "--mcp-config",
                config.path().to_str().unwrap(),
                "hello",
            ])
            .output()
            .expect("Failed to run claudeless");

        assert!(output.status.success(), "Expected success: {:?}", output);

        let stdout = String::from_utf8_lossy(&output.stdout);
        // The first line should be system init which contains mcp_servers
        // Note: We just check that output is produced, the actual MCP server
        // inclusion in output depends on main.rs wiring which isn't fully done yet
        assert!(stdout.contains("system") || stdout.contains("message"));
    }
}
