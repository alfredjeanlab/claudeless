// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use serde_json::json;

use crate::config::ToolCall;
use crate::tui::widgets::permission::{DiffKind, PermissionType};

use super::tool_call_to_permission_type;

#[test]
fn bash_tool_converts_to_bash_permission() {
    let call = ToolCall {
        call: "Bash".to_string(),
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
    let call = ToolCall {
        call: "Bash".to_string(),
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
    let call = ToolCall {
        call: "Bash".to_string(),
        input: json!({}),
        result: None,
    };

    assert!(tool_call_to_permission_type(&call).is_none());
}

#[test]
fn write_tool_converts_to_write_permission() {
    let call = ToolCall {
        call: "Write".to_string(),
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
    let call = ToolCall {
        call: "Edit".to_string(),
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
    let call = ToolCall {
        call: "Read".to_string(),
        input: json!({ "file_path": "test.txt" }),
        result: Some("1 file".to_string()),
    };
    let display = super::format_completed_tool_display(&call, Some("1 file"));
    assert_eq!(display, "Read 1 file (ctrl+o to expand)");
}

#[test]
fn read_streaming_display_uses_reading_prefix() {
    let call = ToolCall {
        call: "Read".to_string(),
        input: json!({ "file_path": "test.txt" }),
        result: Some("1 file\u{2026}".to_string()),
    };
    let display = super::format_completed_tool_display(&call, Some("1 file\u{2026}"));
    assert_eq!(display, "Reading 1 file\u{2026} (ctrl+o to expand)");
}

#[test]
fn unknown_tool_returns_none() {
    let call = ToolCall {
        call: "UnknownTool".to_string(),
        input: json!({}),
        result: None,
    };

    assert!(tool_call_to_permission_type(&call).is_none());
}

// =========================================================================
// handle_turn_result sets correct mode for dialog-based pending permissions
// =========================================================================

use crate::runtime::{PendingPermission, TurnResult};
use crate::state::session::SessionManager;
use crate::time::ClockHandle;
use crate::tui::app::state::TuiAppState;
use crate::tui::app::types::{AppMode, TuiConfig};

fn create_test_app() -> TuiAppState {
    let sessions = SessionManager::new();
    let clock = ClockHandle::fake_at_epoch();
    let tui_config = TuiConfig::default();
    TuiAppState::for_test(sessions, clock, tui_config)
}

#[test]
fn handle_turn_result_sets_elicitation_mode_with_response_text() {
    let state = create_test_app();
    let mut inner = state.inner.lock();

    // Simulate a turn result with response text AND a pending AskUserQuestion
    let result = TurnResult {
        say: Some("Let me ask you a question.".to_string()),
        tools: vec![ToolCall {
            call: "AskUserQuestion".to_string(),
            input: json!({
                "questions": [{
                    "question": "Which DB?",
                    "header": "DB",
                    "options": [
                        { "label": "PostgreSQL", "description": "Relational" },
                        { "label": "SQLite", "description": "Embedded" }
                    ],
                    "multiSelect": false
                }]
            }),
            result: None,
        }],
        usage: None,
        tool_results: vec![],
        hook_continuation: None,
        is_hook_continuation: false,
        pending_permission: Some(PendingPermission {
            tool_call: ToolCall {
                call: "AskUserQuestion".to_string(),
                input: json!({
                    "questions": [{
                        "question": "Which DB?",
                        "header": "DB",
                        "options": [
                            { "label": "PostgreSQL", "description": "Relational" },
                            { "label": "SQLite", "description": "Embedded" }
                        ],
                        "multiSelect": false
                    }]
                }),
                result: None,
            },
            tool_use_id: "toolu_00000000".to_string(),
        }),
    };

    let _action = super::handle_turn_result(&mut inner, result);

    // Mode must be Elicitation, NOT Responding (setup_response_display sets Responding,
    // but it should be overridden back to Elicitation)
    assert_eq!(
        inner.mode,
        AppMode::Elicitation,
        "mode should be Elicitation after handle_turn_result with pending AskUserQuestion"
    );

    // Dialog should be active
    assert!(
        inner.dialog.as_elicitation().is_some(),
        "elicitation dialog should be set"
    );

    // Response text should be captured in display
    assert!(
        inner
            .display
            .response_content
            .contains("Let me ask you a question"),
        "response text should be displayed"
    );
}

#[test]
fn handle_turn_result_sets_plan_approval_mode_with_response_text() {
    let state = create_test_app();
    let mut inner = state.inner.lock();

    let result = TurnResult {
        say: Some("Here is my plan.".to_string()),
        tools: vec![ToolCall {
            call: "ExitPlanMode".to_string(),
            input: json!({}),
            result: None,
        }],
        usage: None,
        tool_results: vec![],
        hook_continuation: None,
        is_hook_continuation: false,
        pending_permission: Some(PendingPermission {
            tool_call: ToolCall {
                call: "ExitPlanMode".to_string(),
                input: json!({}),
                result: None,
            },
            tool_use_id: "toolu_00000000".to_string(),
        }),
    };

    let _action = super::handle_turn_result(&mut inner, result);

    assert_eq!(
        inner.mode,
        AppMode::PlanApproval,
        "mode should be PlanApproval after handle_turn_result with pending ExitPlanMode"
    );
    assert!(
        inner.dialog.as_plan_approval().is_some(),
        "plan approval dialog should be set"
    );
}
