// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! Tasks tests - /tasks command behavior.
//!
//! Behavior observed with: claude --version 2.1.14 (Claude Code)
//!
//! ## Tasks Behavior
//! - `/tasks` opens a dialog showing background tasks
//! - When no tasks are running, shows "No tasks currently running"
//! - Dialog has a header "Background tasks" and footer "↑/↓ to select · Enter to view · Esc to close"
//! - Pressing Escape dismisses the dialog and shows "Background tasks dialog dismissed"
//! - The command appears in autocomplete with description "List and manage background tasks"

mod common;

use common::{assert_tui_matches_fixture, TuiTestSession};

const JSON_SCENARIO: &str = r#"
    {
        "trusted": true,
        "claude_version": "2.1.12",
        "responses": [
            { "pattern": { "type": "any" }, "response": "ok" }
        ]
    }
"#;

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// When /tasks is executed with no background tasks, it shows a dialog with "No tasks currently running".
#[test]
fn test_tasks_empty_shows_no_tasks_message() {
    let tui = TuiTestSession::new("tasks-empty", JSON_SCENARIO);

    // Execute /tasks
    tui.send_line("/tasks");

    // Wait for dialog to appear
    let capture = tui.wait_for("Background tasks");

    assert!(
        capture.contains("No tasks currently running"),
        "/tasks with no background tasks should show empty message.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// /tasks dialog shows correct header and footer controls.
#[test]
fn test_tasks_dialog_has_controls() {
    let tui = TuiTestSession::new("tasks-controls", JSON_SCENARIO);

    // Execute /tasks
    tui.send_line("/tasks");

    // Wait for dialog to appear
    let capture = tui.wait_for("Background tasks");

    // Check for header
    assert!(
        capture.contains("Background tasks"),
        "Dialog should show 'Background tasks' header.\nCapture:\n{}",
        capture
    );

    // Check for footer with controls
    assert!(
        capture.contains("Esc to close"),
        "Dialog should show 'Esc to close' in footer.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Compare /tasks empty dialog against fixture.
// FIXME: Fixture is golden capture from real CLI. normalize_tui() doesn't strip
// shell preamble when capturing mid-session (after sending /tasks command).
// Fixture only contains the dialog box, not the full TUI with header.
#[test]
#[ignore]
fn test_tasks_empty_matches_fixture() {
    let tui = TuiTestSession::new("fixture-tasks-empty", JSON_SCENARIO);

    // Execute /tasks
    tui.send_line("/tasks");

    // Wait for dialog to appear
    let capture = tui.wait_for("Background tasks");

    assert_tui_matches_fixture(&capture, "tasks_empty_dialog.txt", None);
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Pressing Escape dismisses the /tasks dialog.
#[test]
fn test_tasks_dialog_dismiss_with_escape() {
    let tui = TuiTestSession::new("tasks-dismiss", JSON_SCENARIO);

    // Execute /tasks
    tui.send_line("/tasks");

    // Wait for dialog to appear
    let _ = tui.wait_for("Background tasks");

    // Press Escape to dismiss
    tui.send_keys("Escape");

    // Wait for dialog to be dismissed
    let capture = tui.wait_for("dialog dismissed");

    assert!(
        capture.contains("Background tasks dialog dismissed"),
        "Pressing Escape should dismiss the dialog.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// /tasks appears in slash command autocomplete with correct description.
#[test]
fn test_tasks_in_autocomplete() {
    let tui = TuiTestSession::new("tasks-autocomplete", JSON_SCENARIO);

    // Type /tasks to trigger autocomplete
    tui.send_keys("/tasks");

    // Wait for autocomplete to appear
    let capture = tui.wait_for("/tasks");

    // Should show /tasks in autocomplete
    assert!(
        capture.contains("/tasks"),
        "Autocomplete should show /tasks command.\nCapture:\n{}",
        capture
    );

    // Should show description
    assert!(
        capture.contains("List and manage background tasks")
            || capture.contains("background tasks"),
        "Autocomplete should show tasks description.\nCapture:\n{}",
        capture
    );
}
