// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::super::test_helpers::execute;
use super::*;
use crate::tools::builtin::extract_str;
use serde_json::json;
use yare::parameterized;

#[test]
fn test_extract_command() {
    assert_eq!(
        extract_str(&json!({ "command": "ls -la" }), "command"),
        Some("ls -la")
    );
    assert_eq!(extract_str(&json!({}), "command"), None);
}

#[test]
#[cfg(unix)]
fn test_bash_real_execution() {
    let result = execute::<BashExecutor>(json!({ "command": "echo hello" }));
    assert!(!result.is_error);
    let text = result.text().unwrap();
    assert!(text.contains("hello"));
    assert!(text.contains("Exit code: 0"));
}

#[parameterized(
    exit_42 = { "exit 42", "Exit code: 42" },
    nonexistent_path = { "ls /nonexistent_path_abc123", "Exit code:" },
)]
#[cfg(unix)]
fn bash_failed_commands(command: &str, expected: &str) {
    let result = execute::<BashExecutor>(json!({ "command": command }));
    assert!(result.is_error);
    let text = result.text().unwrap();
    assert!(text.contains(expected));
    assert!(!text.contains("Exit code: 0"));
}

#[test]
fn test_tool_name() {
    assert_eq!(BashExecutor.tool_name(), crate::tools::ToolName::Bash);
}
