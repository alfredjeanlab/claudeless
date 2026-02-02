// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! Permission mode behavioral tests - shift+tab cycling behavior.
//!
//! Tests user interactions with permission mode cycling.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{start_tui_ext, tmux, write_scenario, TUI_READY_PATTERN};

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
    let mut previous = start_tui_ext(&session, &scenario, 120, 20, TUI_READY_PATTERN);

    // Send shift+tabs, waiting for UI to update after each
    for _ in 0..num_tabs {
        tmux::send_keys(&session, "BTab");
        previous = tmux::wait_for_change(&session, &previous);
    }

    let capture = previous;

    // Cleanup: first C-c cancels operation, wait for effect, second C-c exits
    tmux::send_keys(&session, "C-c");
    let _ = tmux::wait_for_change(&session, &capture);
    tmux::send_keys(&session, "C-c");
    tmux::kill_session(&session);

    capture
}

/// Behavior observed with: claude --version 2.1.29 (Claude Code)
///
/// Shift+Tab cycles through permission modes:
/// - Without --dangerously-skip-permissions: default -> acceptEdits -> plan -> default (3 modes)
/// - With --dangerously-skip-permissions: default -> acceptEdits -> plan -> bypass -> default (4 modes)
#[test]
fn test_shift_tab_cycles_to_accept_edits_mode() {
    let capture = capture_after_shift_tabs("claudeless-shift-tab-1", 1);

    assert!(
        capture.to_lowercase().contains("accept") || capture.contains("⏵⏵"),
        "After 1 shift+tab, should show accept edits mode.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.29 (Claude Code)
#[test]
fn test_shift_tab_cycles_to_plan_mode() {
    let capture = capture_after_shift_tabs("claudeless-shift-tab-2", 2);

    assert!(
        capture.contains("⏸") || capture.contains("plan mode"),
        "After 2 shift+tabs, should show plan mode.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.29 (Claude Code)
///
/// Without --dangerously-skip-permissions, the cycle is 3 modes:
/// default -> acceptEdits -> plan -> default
#[test]
fn test_shift_tab_cycles_back_to_default() {
    let capture = capture_after_shift_tabs("claudeless-shift-tab-3", 3);

    assert!(
        capture.contains("?") && capture.to_lowercase().contains("shortcut"),
        "After 3 shift+tabs, should cycle back to default mode.\nCapture:\n{}",
        capture
    );
}
