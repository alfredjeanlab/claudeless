// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use serde_json::json;

use crate::config::ToolCallSpec;
use crate::tui::widgets::permission::{DiffKind, PermissionType};

use super::tool_call_to_permission_type;

#[test]
fn bash_tool_converts_to_bash_permission() {
    let call = ToolCallSpec {
        tool: "Bash".to_string(),
        input: json!({
            "command": "ls -la",
            "description": "List files"
        }),
        result: None,
    };

    let perm = tool_call_to_permission_type(&call).unwrap();
    match perm {
        PermissionType::Bash {
            command,
            description,
        } => {
            assert_eq!(command, "ls -la");
            assert_eq!(description, Some("List files".to_string()));
        }
        other => panic!("Expected Bash, got {:?}", other),
    }
}

#[test]
fn bash_tool_without_description() {
    let call = ToolCallSpec {
        tool: "Bash".to_string(),
        input: json!({ "command": "echo hi" }),
        result: None,
    };

    let perm = tool_call_to_permission_type(&call).unwrap();
    match perm {
        PermissionType::Bash {
            command,
            description,
        } => {
            assert_eq!(command, "echo hi");
            assert_eq!(description, None);
        }
        other => panic!("Expected Bash, got {:?}", other),
    }
}

#[test]
fn bash_tool_without_command_returns_none() {
    let call = ToolCallSpec {
        tool: "Bash".to_string(),
        input: json!({}),
        result: None,
    };

    assert!(tool_call_to_permission_type(&call).is_none());
}

#[test]
fn write_tool_converts_to_write_permission() {
    let call = ToolCallSpec {
        tool: "Write".to_string(),
        input: json!({
            "file_path": "/tmp/test.txt",
            "content": "line1\nline2\nline3"
        }),
        result: None,
    };

    let perm = tool_call_to_permission_type(&call).unwrap();
    match perm {
        PermissionType::Write {
            file_path,
            content_lines,
        } => {
            assert_eq!(file_path, "/tmp/test.txt");
            assert_eq!(content_lines, vec!["line1", "line2", "line3"]);
        }
        other => panic!("Expected Write, got {:?}", other),
    }
}

#[test]
fn edit_tool_converts_to_edit_permission() {
    let call = ToolCallSpec {
        tool: "Edit".to_string(),
        input: json!({
            "file_path": "src/main.rs",
            "old_string": "old line",
            "new_string": "new line"
        }),
        result: None,
    };

    let perm = tool_call_to_permission_type(&call).unwrap();
    match perm {
        PermissionType::Edit {
            file_path,
            diff_lines,
        } => {
            assert_eq!(file_path, "src/main.rs");
            // Removed + NoNewline + Added + NoNewline = 4 lines
            assert_eq!(diff_lines.len(), 4);
            assert_eq!(diff_lines[0].kind, DiffKind::Removed);
            assert_eq!(diff_lines[0].content, "old line");
            assert_eq!(diff_lines[1].kind, DiffKind::NoNewline);
            assert_eq!(diff_lines[2].kind, DiffKind::Added);
            assert_eq!(diff_lines[2].content, "new line");
            assert_eq!(diff_lines[3].kind, DiffKind::NoNewline);
        }
        other => panic!("Expected Edit, got {:?}", other),
    }
}

#[test]
fn read_completed_display_uses_read_prefix() {
    let call = ToolCallSpec {
        tool: "Read".to_string(),
        input: json!({ "file_path": "test.txt" }),
        result: Some("1 file".to_string()),
    };
    let display = super::format_completed_tool_display(&call, Some("1 file"));
    assert_eq!(display, "Read 1 file (ctrl+o to expand)");
}

#[test]
fn read_streaming_display_uses_reading_prefix() {
    let call = ToolCallSpec {
        tool: "Read".to_string(),
        input: json!({ "file_path": "test.txt" }),
        result: Some("1 file\u{2026}".to_string()),
    };
    let display = super::format_completed_tool_display(&call, Some("1 file\u{2026}"));
    assert_eq!(display, "Reading 1 file\u{2026} (ctrl+o to expand)");
}

#[test]
fn unknown_tool_returns_none() {
    let call = ToolCallSpec {
        tool: "UnknownTool".to_string(),
        input: json!({}),
        result: None,
    };

    assert!(tool_call_to_permission_type(&call).is_none());
}
