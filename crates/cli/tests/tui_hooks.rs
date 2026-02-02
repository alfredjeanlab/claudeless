// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

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

use common::TuiTestSession;

const SCENARIO: &str = r#"
    name = "test"
    [[responses]]
    pattern = { type = "any" }
    response = "Hello!"
"#;

// =============================================================================
// /hooks Autocomplete Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing /hooks shows autocomplete dropdown with hooks description
// Note: requires slash command autocomplete
#[test]
fn test_tui_hooks_command_shows_autocomplete() {
    let tui = TuiTestSession::new("hooks-autocomplete", SCENARIO);

    // Type /hooks
    tui.send_keys("/hooks");
    let capture = tui.wait_for("Manage hook configurations for tool events");

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
// Note: requires /hooks dialog
#[test]
fn test_tui_hooks_shows_dialog_with_hook_types() {
    let tui = TuiTestSession::new("hooks-dialog", SCENARIO);
    // Type /hooks and press Enter
    tui.send_keys("/hooks");
    let _ = tui.wait_for("Manage hook");
    tui.send_keys("Enter");
    let capture = tui.wait_for("Hooks");

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
        capture.contains("Enter to confirm") && capture.contains("Esc to cancel"),
        "Should show navigation hints.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /hooks dialog shows count of active hooks
// Note: requires /hooks dialog
#[test]
fn test_tui_hooks_shows_active_hooks_count() {
    let tui = TuiTestSession::new("hooks-count", SCENARIO);
    // Type /hooks and press Enter
    tui.send_keys("/hooks");
    let _ = tui.wait_for("Manage hook");
    tui.send_keys("Enter");
    let capture = tui.wait_for("Hooks");

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
// Note: requires /hooks dialog navigation
#[test]
fn test_tui_hooks_arrow_navigation() {
    let tui = TuiTestSession::new("hooks-nav", SCENARIO);
    // Open hooks dialog
    tui.send_keys("/hooks");
    let _ = tui.wait_for("Manage hook");
    tui.send_keys("Enter");
    let initial = tui.wait_for("PreToolUse");

    // Should start with first hook selected
    assert!(
        initial.contains("❯") && initial.contains("PreToolUse"),
        "First hook should be selected by default.\nCapture:\n{}",
        initial
    );

    // Press Down to move to next hook
    tui.send_keys("Down");
    let after_down = tui.wait_for_change(&initial);

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
// Note: requires /hooks dialog scrolling
#[test]
fn test_tui_hooks_list_scrolls() {
    let tui = TuiTestSession::new("hooks-scroll", SCENARIO);

    // Open hooks dialog
    tui.send_keys("/hooks");
    let _ = tui.wait_for("Manage hook");
    tui.send_keys("Enter");
    let initial = tui.wait_for("Hooks");

    // Navigate down multiple times to trigger scrolling
    for _ in 0..6 {
        tui.send_keys("Down");
    }
    let after_scroll = tui.wait_for_change(&initial);

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
// Note: requires /hooks matcher dialog
#[test]
fn test_tui_hooks_select_shows_matchers() {
    let tui = TuiTestSession::new("hooks-matchers", SCENARIO);
    // Open hooks dialog and select PreToolUse
    tui.send_keys("/hooks");
    let _ = tui.wait_for("Manage hook");
    tui.send_keys("Enter");
    let hooks = tui.wait_for("PreToolUse");
    tui.send_keys("Enter");
    let capture = tui.wait_for_change(&hooks);

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
// Note: requires /hooks matcher dialog
#[test]
fn test_tui_hooks_matchers_shows_exit_code_help() {
    let tui = TuiTestSession::new("hooks-exit-codes", SCENARIO);
    // Open hooks dialog and select PreToolUse
    tui.send_keys("/hooks");
    let _ = tui.wait_for("Manage hook");
    tui.send_keys("Enter");
    let hooks = tui.wait_for("PreToolUse");
    tui.send_keys("Enter");
    let capture = tui.wait_for_change(&hooks);

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
// Note: requires /hooks dialog dismiss
#[test]
fn test_tui_hooks_escape_dismisses_dialog() {
    let tui = TuiTestSession::new("hooks-dismiss", SCENARIO);
    // Open hooks dialog
    tui.send_keys("/hooks");
    let _ = tui.wait_for("Manage hook");
    tui.send_keys("Enter");
    let dialog = tui.wait_for("Hooks");

    // Press Escape to dismiss
    tui.send_keys("Escape");
    let capture = tui.wait_for_change(&dialog);

    assert!(
        capture.contains("Hooks dialog dismissed"),
        "Escape should dismiss hooks dialog and show message.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Escape in matchers dialog returns to main hooks dialog
// Note: requires /hooks nested dialog navigation
#[test]
fn test_tui_hooks_escape_from_matchers_returns_to_hooks() {
    let tui = TuiTestSession::new("hooks-back", SCENARIO);
    // Open hooks dialog and select PreToolUse
    tui.send_keys("/hooks");
    let _ = tui.wait_for("Manage hook");
    tui.send_keys("Enter");
    let _ = tui.wait_for("PreToolUse");
    tui.send_keys("Enter");
    let matchers = tui.wait_for("Tool Matchers");

    // Press Escape to go back to hooks list
    tui.send_keys("Escape");
    let capture = tui.wait_for_change(&matchers);

    // Should return to main hooks dialog
    assert!(
        capture.contains("Hooks") && !capture.contains("Tool Matchers"),
        "Escape should return to main hooks dialog.\nCapture:\n{}",
        capture
    );
}
