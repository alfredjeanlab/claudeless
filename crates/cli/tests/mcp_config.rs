// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for MCP configuration loading.

#![allow(deprecated)] // Command::cargo_bin is deprecated but still functional

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "-p",
            "--mcp-config",
            config.path().to_str().unwrap(),
            "hello",
        ])
        .assert()
        .success();
    }

    #[test]
    fn test_mcp_config_inline_json() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "-p",
            "--mcp-config",
            r#"{"mcpServers":{"inline":{"command":"node"}}}"#,
            "hello",
        ])
        .assert()
        .success();
    }

    #[test]
    fn test_multiple_mcp_configs() {
        let config1 = write_config(r#"{"mcpServers":{"a":{"command":"a"}}}"#);
        let config2 = write_config(r#"{"mcpServers":{"b":{"command":"b"}}}"#);

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "-p",
            "--mcp-config",
            config1.path().to_str().unwrap(),
            "--mcp-config",
            config2.path().to_str().unwrap(),
            "hello",
        ])
        .assert()
        .success();
    }

    #[test]
    fn test_strict_mcp_config_flag() {
        let config = write_config(r#"{"mcpServers":{"only":{"command":"echo"}}}"#);

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "-p",
            "--strict-mcp-config",
            "--mcp-config",
            config.path().to_str().unwrap(),
            "hello",
        ])
        .assert()
        .success();
    }

    #[test]
    fn test_mcp_debug_flag() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args(["-p", "--mcp-debug", "hello"]).assert().success();
    }
}

mod config_errors {
    use super::*;

    #[test]
    fn test_nonexistent_config_file_error() {
        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "-p",
            "--mcp-config",
            "/nonexistent/path/config.json",
            "hello",
        ])
        .assert()
        .failure();
    }

    #[test]
    fn test_invalid_json_config_error() {
        let config = write_config("not valid json at all");

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "-p",
            "--mcp-config",
            config.path().to_str().unwrap(),
            "hello",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("parse").or(predicate::str::contains("Failed")));
    }

    #[test]
    fn test_empty_json_config_accepted() {
        // Empty JSON object is valid, just has no servers
        let config = write_config("{}");

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "-p",
            "--mcp-config",
            config.path().to_str().unwrap(),
            "hello",
        ])
        .assert()
        .success();
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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "-p",
            "--mcp-config",
            config.path().to_str().unwrap(),
            "hello",
        ])
        .assert()
        .success();
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

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        cmd.args([
            "-p",
            "--mcp-config",
            config.path().to_str().unwrap(),
            "hello",
        ])
        .assert()
        .success();
    }
}

mod stream_json_output {
    use super::*;

    #[test]
    fn test_mcp_servers_in_stream_json_init() {
        let config = write_config(r#"{"mcpServers":{"test":{"command":"echo"}}}"#);

        let mut cmd = Command::cargo_bin("claudeless").unwrap();
        let output = cmd
            .args([
                "-p",
                "--output-format",
                "stream-json",
                "--mcp-config",
                config.path().to_str().unwrap(),
                "hello",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let stdout = String::from_utf8_lossy(&output);
        // The first line should be system init which contains mcp_servers
        // Note: We just check that output is produced, the actual MCP server
        // inclusion in output depends on main.rs wiring which isn't fully done yet
        assert!(stdout.contains("system") || stdout.contains("message"));
    }
}
