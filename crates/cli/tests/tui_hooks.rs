// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TUI /hooks command tests - hooks management dialog behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## /hooks Command Behavior
//! - Typing /hooks shows autocomplete with "Manage hook configurations for tool events"
//! - Executing /hooks shows a scrollable list of hook types:
//!   - PreToolUse - Before tool execution
//!   - PostToolUse - After tool execution
//!   - PostToolUseFailure - After tool execution fails
//!   - Notification - When notifications are sent
//!   - UserPromptSubmit - When the user submits a prompt
//!   - SessionStart - When a new session is started
//!   - Stop - Right before Claude concludes its response
//!   - SubagentStart - When a subagent (Task tool call) is started
//!   - SubagentStop - Right before a subagent (Task tool call) concludes its response
//!   - PreCompact - Before conversation compaction
//!   - SessionEnd - When a session is ending
//!   - PermissionRequest - When a permission dialog is displayed
//!   - Setup - Repo setup hooks for init and maintenance
//!   - Disable all hooks (special action)
//! - Up/Down arrow keys navigate through hook types
//! - Enter selects a hook type and shows its matchers
//! - Escape dismisses the dialog and shows "Hooks dialog dismissed"
//! - Shows count of active hooks (e.g., "4 hooks")

mod common;

use common::{start_tui, tmux, write_scenario};

// =============================================================================
// /hooks Autocomplete Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing /hooks shows autocomplete dropdown with hooks description
// TODO(implement): requires slash command autocomplete
#[test]
#[ignore]
fn test_tui_hooks_command_shows_autocomplete() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-hooks-autocomplete";
    let previous = start_tui(session, &scenario);

    // Type /hooks
    tmux::send_keys(session, "/hooks");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    assert!(
        capture.contains("/hooks")
            && capture.contains("Manage hook configurations for tool events"),
        "/hooks should show autocomplete with description.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /hooks Dialog Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /hooks command shows a dialog with list of hook types
// TODO(implement): requires /hooks dialog
#[test]
#[ignore]
fn test_tui_hooks_shows_dialog_with_hook_types() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-hooks-dialog";
    let previous = start_tui(session, &scenario);

    // Type /hooks and press Enter
    tmux::send_keys(session, "/hooks");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let capture = tmux::wait_for_content(session, "Hooks");

    tmux::kill_session(session);

    // Should show the hooks dialog with hook types
    assert!(
        capture.contains("Hooks"),
        "Should show 'Hooks' header.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("PreToolUse") && capture.contains("Before tool execution"),
        "Should show PreToolUse hook type.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Enter to confirm") && capture.contains("esc to cancel"),
        "Should show navigation hints.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /hooks dialog shows count of active hooks
// TODO(implement): requires /hooks dialog
#[test]
#[ignore]
fn test_tui_hooks_shows_active_hooks_count() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-hooks-count";
    let previous = start_tui(session, &scenario);

    // Type /hooks and press Enter
    tmux::send_keys(session, "/hooks");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let capture = tmux::wait_for_content(session, "Hooks");

    tmux::kill_session(session);

    // Should show count of active hooks
    assert!(
        capture.contains("hooks"),
        "Should show active hooks count (e.g., '4 hooks').\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /hooks Navigation Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Down arrow navigates through hook types
// TODO(implement): requires /hooks dialog navigation
#[test]
#[ignore]
fn test_tui_hooks_arrow_navigation() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-hooks-nav";
    let previous = start_tui(session, &scenario);

    // Open hooks dialog
    tmux::send_keys(session, "/hooks");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let initial = tmux::wait_for_content(session, "PreToolUse");

    // Should start with first hook selected
    assert!(
        initial.contains("❯") && initial.contains("PreToolUse"),
        "First hook should be selected by default.\nCapture:\n{}",
        initial
    );

    // Press Down to move to next hook
    tmux::send_keys(session, "Down");
    let after_down = tmux::wait_for_change(session, &initial);

    tmux::kill_session(session);

    // Should show next hook selected (PostToolUse)
    assert!(
        after_down.contains("❯") && after_down.contains("PostToolUse"),
        "Down arrow should select next hook.\nCapture:\n{}",
        after_down
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Hooks list scrolls when navigating past visible items
// TODO(implement): requires /hooks dialog scrolling
#[test]
#[ignore]
fn test_tui_hooks_list_scrolls() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-hooks-scroll";
    let previous = start_tui(session, &scenario);

    // Open hooks dialog
    tmux::send_keys(session, "/hooks");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let initial = tmux::wait_for_content(session, "Hooks");

    // Navigate down multiple times to trigger scrolling
    for _ in 0..6 {
        tmux::send_keys(session, "Down");
    }
    let after_scroll = tmux::wait_for_change(session, &initial);

    tmux::kill_session(session);

    // Should show later hooks that weren't initially visible
    assert!(
        after_scroll.contains("SessionStart") || after_scroll.contains("Stop"),
        "List should scroll to show later hooks.\nCapture:\n{}",
        after_scroll
    );
}

// =============================================================================
// /hooks Selection Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Selecting a hook type shows its matchers dialog
// TODO(implement): requires /hooks matcher dialog
#[test]
#[ignore]
fn test_tui_hooks_select_shows_matchers() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-hooks-matchers";
    let previous = start_tui(session, &scenario);

    // Open hooks dialog and select PreToolUse
    tmux::send_keys(session, "/hooks");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let hooks = tmux::wait_for_content(session, "PreToolUse");
    tmux::send_keys(session, "Enter");
    let capture = tmux::wait_for_change(session, &hooks);

    tmux::kill_session(session);

    // Should show matchers dialog for PreToolUse
    assert!(
        capture.contains("PreToolUse") && capture.contains("Tool Matchers"),
        "Should show matcher dialog header.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Add new matcher"),
        "Should show option to add new matcher.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Matcher dialog shows help text about exit codes
// TODO(implement): requires /hooks matcher dialog
#[test]
#[ignore]
fn test_tui_hooks_matchers_shows_exit_code_help() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-hooks-exit-codes";
    let previous = start_tui(session, &scenario);

    // Open hooks dialog and select PreToolUse
    tmux::send_keys(session, "/hooks");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let hooks = tmux::wait_for_content(session, "PreToolUse");
    tmux::send_keys(session, "Enter");
    let capture = tmux::wait_for_change(session, &hooks);

    tmux::kill_session(session);

    // Should show exit code documentation
    assert!(
        capture.contains("Exit code 0"),
        "Should show exit code 0 documentation.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Exit code 2"),
        "Should show exit code 2 documentation.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /hooks Dismiss Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Pressing Escape dismisses the hooks dialog
// TODO(implement): requires /hooks dialog dismiss
#[test]
#[ignore]
fn test_tui_hooks_escape_dismisses_dialog() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-hooks-dismiss";
    let previous = start_tui(session, &scenario);

    // Open hooks dialog
    tmux::send_keys(session, "/hooks");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let dialog = tmux::wait_for_content(session, "Hooks");

    // Press Escape to dismiss
    tmux::send_keys(session, "Escape");
    let capture = tmux::wait_for_change(session, &dialog);

    tmux::kill_session(session);

    assert!(
        capture.contains("Hooks dialog dismissed"),
        "Escape should dismiss hooks dialog and show message.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Escape in matchers dialog returns to main hooks dialog
// TODO(implement): requires /hooks nested dialog navigation
#[test]
#[ignore]
fn test_tui_hooks_escape_from_matchers_returns_to_hooks() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-hooks-back";
    let previous = start_tui(session, &scenario);

    // Open hooks dialog and select PreToolUse
    tmux::send_keys(session, "/hooks");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let _ = tmux::wait_for_content(session, "PreToolUse");
    tmux::send_keys(session, "Enter");
    let matchers = tmux::wait_for_content(session, "Tool Matchers");

    // Press Escape to go back to hooks list
    tmux::send_keys(session, "Escape");
    let capture = tmux::wait_for_change(session, &matchers);

    tmux::kill_session(session);

    // Should return to main hooks dialog
    assert!(
        capture.contains("Hooks") && !capture.contains("Tool Matchers"),
        "Escape should return to main hooks dialog.\nCapture:\n{}",
        capture
    );
}
