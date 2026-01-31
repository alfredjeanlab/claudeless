// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tests for permission module, focusing on tool JSONL recording.

use super::*;

#[test]
fn test_build_tool_use_content_bash() {
    let (id, content) = build_tool_use_content(&PermissionType::Bash {
        command: "ls -la".to_string(),
        description: Some("List files".to_string()),
    });

    assert!(id.starts_with("toolu_"));
    assert_eq!(content.len(), 1);
    match &content[0] {
        ContentBlock::ToolUse { name, input, .. } => {
            assert_eq!(name, "Bash");
            assert_eq!(input["command"], "ls -la");
            assert_eq!(input["description"], "List files");
        }
        _ => panic!("Expected ToolUse content block"),
    }
}

#[test]
fn test_build_tool_use_content_bash_no_description() {
    let (id, content) = build_tool_use_content(&PermissionType::Bash {
        command: "npm test".to_string(),
        description: None,
    });

    assert!(id.starts_with("toolu_"));
    match &content[0] {
        ContentBlock::ToolUse { name, input, .. } => {
            assert_eq!(name, "Bash");
            assert_eq!(input["command"], "npm test");
            assert!(input.get("description").is_none());
        }
        _ => panic!("Expected ToolUse content block"),
    }
}

#[test]
fn test_build_tool_use_content_edit() {
    use crate::tui::widgets::permission::DiffKind;

    let (id, content) = build_tool_use_content(&PermissionType::Edit {
        file_path: "src/main.rs".to_string(),
        diff_lines: vec![
            DiffLine {
                line_num: Some(1),
                kind: DiffKind::Removed,
                content: "old".to_string(),
            },
            DiffLine {
                line_num: Some(2),
                kind: DiffKind::Added,
                content: "new".to_string(),
            },
        ],
    });

    assert!(id.starts_with("toolu_"));
    match &content[0] {
        ContentBlock::ToolUse { name, input, .. } => {
            assert_eq!(name, "Edit");
            assert_eq!(input["file_path"], "src/main.rs");
            assert_eq!(input["changes"], 2);
        }
        _ => panic!("Expected ToolUse content block"),
    }
}

#[test]
fn test_build_tool_use_content_write() {
    let (id, content) = build_tool_use_content(&PermissionType::Write {
        file_path: "hello.txt".to_string(),
        content_lines: vec!["Hello".to_string(), "World".to_string()],
    });

    assert!(id.starts_with("toolu_"));
    match &content[0] {
        ContentBlock::ToolUse { name, input, .. } => {
            assert_eq!(name, "Write");
            assert_eq!(input["file_path"], "hello.txt");
            assert_eq!(input["content"], "Hello\nWorld");
        }
        _ => panic!("Expected ToolUse content block"),
    }
}

#[test]
fn test_tool_use_id_format() {
    // Tool use IDs should be "toolu_" followed by a 32-char hex UUID (no hyphens)
    let (id, _) = build_tool_use_content(&PermissionType::Bash {
        command: "test".to_string(),
        description: None,
    });

    assert!(id.starts_with("toolu_"));
    let uuid_part = &id[6..]; // Skip "toolu_"
    assert_eq!(uuid_part.len(), 32);
    assert!(uuid_part.chars().all(|c| c.is_ascii_hexdigit()));
}
