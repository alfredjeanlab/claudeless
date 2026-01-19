// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Permission mode behavioral tests - shift+tab cycling behavior.
//!
//! Tests user interactions with permission mode cycling.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{
    assert_tui_matches_fixture, start_tui, start_tui_ext, tmux, write_scenario, TUI_READY_PATTERN,
};

/// Helper to capture after a sequence of shift+tab presses
fn capture_after_shift_tabs(session: &str, num_tabs: usize) -> String {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );
    let mut previous = start_tui_ext(session, &scenario, 120, 20, TUI_READY_PATTERN);

    // Send shift+tabs, waiting for UI to update after each
    for _ in 0..num_tabs {
        tmux::send_keys(session, "BTab");
        previous = tmux::wait_for_change(session, &previous);
    }

    let capture = previous;

    // Cleanup: first C-c cancels operation, wait for effect, second C-c exits
    tmux::send_keys(session, "C-c");
    let _ = tmux::wait_for_change(session, &capture);
    tmux::send_keys(session, "C-c");
    tmux::kill_session(session);

    capture
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Shift+Tab cycles through permission modes: default -> plan -> acceptEdits -> bypass -> default
#[test]
fn test_shift_tab_cycles_to_plan_mode() {
    let capture = capture_after_shift_tabs("claudeless-shift-tab-1", 1);

    assert!(
        capture.contains("plan") || capture.contains("⏸"),
        "After 1 shift+tab, should show plan mode.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_shift_tab_cycles_to_accept_edits_mode() {
    let capture = capture_after_shift_tabs("claudeless-shift-tab-2", 2);

    assert!(
        capture.to_lowercase().contains("accept") || capture.contains("⏵⏵"),
        "After 2 shift+tabs, should show accept edits mode.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_shift_tab_cycles_back_to_default() {
    let capture = capture_after_shift_tabs("claudeless-shift-tab-4", 4);

    assert!(
        capture.contains("?") && capture.to_lowercase().contains("shortcut"),
        "After 4 shift+tabs, should cycle back to default mode.\nCapture:\n{}",
        capture
    );
}

/// Compare default permission mode status against real Claude fixture
#[test]
fn test_permission_default_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = "claudeless-fixture-perm-default";
    let capture = start_tui(session, &scenario);

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "permission_default.txt", None);
}

/// Compare plan mode status against real Claude fixture
#[test]
fn test_permission_plan_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "permission_mode": "plan"
        }
        "#,
    );

    let session = "claudeless-fixture-perm-plan";
    // Plan mode shows "plan mode on" instead of "? for shortcuts"
    let capture = start_tui_ext(session, &scenario, 120, 40, "plan mode");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "permission_plan.txt", None);
}

// =============================================================================
// Category B: Ignored Tests (require unimplemented TUI features)
// =============================================================================

#[test]
fn test_permission_bash_command_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = "claudeless-fixture-perm-bash";
    start_tui(session, &scenario);

    // Type the trigger prompt
    tmux::send_keys(session, "test bash permission");
    tmux::send_keys(session, "Enter");

    // Wait for bash permission dialog to appear
    let capture = tmux::wait_for_content(session, "Bash command");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "permission_bash_command.txt", None);
}

#[test]
fn test_permission_edit_file_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = "claudeless-fixture-perm-edit";
    start_tui(session, &scenario);

    // Type the trigger prompt
    tmux::send_keys(session, "test edit permission");
    tmux::send_keys(session, "Enter");

    // Wait for edit permission dialog to appear
    let capture = tmux::wait_for_content(session, "Edit file");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "permission_edit_file.txt", None);
}

#[test]
fn test_permission_write_file_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = "claudeless-fixture-perm-write";
    start_tui(session, &scenario);

    // Type the trigger prompt
    tmux::send_keys(session, "test write permission");
    tmux::send_keys(session, "Enter");

    // Wait for write permission dialog to appear
    let capture = tmux::wait_for_content(session, "Create file");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "permission_write_file.txt", None);
}

#[test]
fn test_permission_trust_folder_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": false
        }
        "#,
    );

    let session = "claudeless-fixture-perm-trust";
    // Would need trust prompt to appear
    let capture = start_tui_ext(session, &scenario, 120, 40, "trust the files");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "permission_trust_folder.txt", None);
}

#[test]
fn test_status_bar_extended_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "permission_mode": "accept-edits"
        }
        "#,
    );

    let session = "claudeless-fixture-status-extended";
    // Accept edits mode should show extended status bar
    let capture = start_tui_ext(session, &scenario, 120, 40, "accept edits");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "status_bar_extended.txt", None);
}
