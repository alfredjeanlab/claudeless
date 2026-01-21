// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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

use common::{assert_tui_matches_fixture, start_tui, tmux, write_scenario};

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// When /tasks is executed with no background tasks, it shows a dialog with "No tasks currently running".
// TODO(implement): requires /tasks slash command implementation
#[test]
#[ignore]
fn test_tasks_empty_shows_no_tasks_message() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-tasks-empty";
    start_tui(session, &scenario);

    // Execute /tasks
    tmux::send_line(session, "/tasks");

    // Wait for dialog to appear
    let capture = tmux::wait_for_content(session, "Background tasks");

    tmux::kill_session(session);

    assert!(
        capture.contains("No tasks currently running"),
        "/tasks with no background tasks should show empty message.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// /tasks dialog shows correct header and footer controls.
// TODO(implement): requires /tasks slash command implementation
#[test]
#[ignore]
fn test_tasks_dialog_has_controls() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-tasks-controls";
    start_tui(session, &scenario);

    // Execute /tasks
    tmux::send_line(session, "/tasks");

    // Wait for dialog to appear
    let capture = tmux::wait_for_content(session, "Background tasks");

    tmux::kill_session(session);

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
// TODO(implement): requires /tasks slash command implementation
#[test]
#[ignore]
fn test_tasks_empty_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-fixture-tasks-empty";
    start_tui(session, &scenario);

    // Execute /tasks
    tmux::send_line(session, "/tasks");

    // Wait for dialog to appear
    let capture = tmux::wait_for_content(session, "Background tasks");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "tasks_empty_dialog.txt", None);
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Pressing Escape dismisses the /tasks dialog.
// TODO(implement): requires /tasks slash command implementation
#[test]
#[ignore]
fn test_tasks_dialog_dismiss_with_escape() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-tasks-dismiss";
    start_tui(session, &scenario);

    // Execute /tasks
    tmux::send_line(session, "/tasks");

    // Wait for dialog to appear
    let _ = tmux::wait_for_content(session, "Background tasks");

    // Press Escape to dismiss
    tmux::send_keys(session, "Escape");

    // Wait for dialog to be dismissed
    let capture = tmux::wait_for_content(session, "dialog dismissed");

    tmux::kill_session(session);

    assert!(
        capture.contains("Background tasks dialog dismissed"),
        "Pressing Escape should dismiss the dialog.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// /tasks appears in slash command autocomplete with correct description.
// TODO(implement): requires slash command autocomplete for /tasks
#[test]
#[ignore]
fn test_tasks_in_autocomplete() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-tasks-autocomplete";
    start_tui(session, &scenario);

    // Type /tasks to trigger autocomplete
    tmux::send_keys(session, "/tasks");

    // Wait for autocomplete to appear
    let capture = tmux::wait_for_content(session, "/tasks");

    tmux::kill_session(session);

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
