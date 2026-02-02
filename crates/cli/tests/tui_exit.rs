// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! TUI exit tests - Ctrl+C and Ctrl+D behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## Ctrl+C Behavior
//! - First Ctrl+C: clears input (if any) AND shows "Press Ctrl-C again to exit"
//! - The exit hint message times out after ~2 seconds and returns to "? for shortcuts"
//! - Second Ctrl+C (within timeout): exits the TUI
//!
//! ## Ctrl+D Behavior
//! - With text in input: ignored (does nothing)
//! - On empty input: shows "Press Ctrl-D again to exit"
//! - The exit hint message times out after ~2 seconds and returns to "? for shortcuts"
//! - Second Ctrl+D (within timeout): exits the TUI

mod common;

use std::time::Duration;

use common::{simple_scenario_toml, TuiTestSession};

fn scenario() -> String {
    simple_scenario_toml("Hello!")
}

// =============================================================================
// Ctrl+C Exit Hint Rendering Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// First Ctrl+C on empty input shows "Press Ctrl-C again to exit" in status bar
#[test]
fn test_tui_ctrl_c_shows_exit_hint_on_empty_input() {
    let tui = TuiTestSession::new("ctrl-c-hint-empty", &scenario());
    let previous = tui.capture();

    // Press Ctrl+C on empty input
    tui.send_keys("C-c");
    let capture = tui.wait_for_change(&previous);

    assert!(
        capture.contains("Press Ctrl-C again to exit"),
        "First Ctrl+C should show exit hint.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// First Ctrl+C with text in input clears input AND shows exit hint
#[test]
fn test_tui_ctrl_c_clears_input_and_shows_exit_hint() {
    let tui = TuiTestSession::new("ctrl-c-hint-text", &scenario());
    let previous = tui.capture();

    // Type some text
    tui.send_keys("hello world test");
    let with_text = tui.wait_for_change(&previous);
    assert!(
        with_text.contains("hello world test"),
        "Text should appear in input.\nCapture:\n{}",
        with_text
    );

    // Press Ctrl+C - should clear input AND show exit hint
    tui.send_keys("C-c");
    let after_ctrl_c = tui.wait_for_change(&with_text);

    assert!(
        !after_ctrl_c.contains("hello world test"),
        "Input should be cleared after Ctrl+C.\nCapture:\n{}",
        after_ctrl_c
    );
    assert!(
        after_ctrl_c.contains("Press Ctrl-C again to exit"),
        "Should show exit hint after Ctrl+C.\nCapture:\n{}",
        after_ctrl_c
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Exit hint times out and returns to normal status bar
#[test]
fn test_tui_ctrl_c_exit_hint_times_out() {
    let tui = TuiTestSession::new("ctrl-c-timeout", &scenario());
    let previous = tui.capture();

    // Press Ctrl+C to show exit hint
    tui.send_keys("C-c");
    let with_hint = tui.wait_for_change(&previous);
    assert!(
        with_hint.contains("Press Ctrl-C again to exit"),
        "Should show exit hint.\nCapture:\n{}",
        with_hint
    );

    // Wait for timeout (~2 seconds) and check it returns to normal
    // Use 3 second timeout to allow for the ~2 second hint timeout plus buffer
    let after_timeout = tui.wait_for_timeout("? for shortcuts", Duration::from_secs(3));

    assert!(
        !after_timeout.contains("Press Ctrl-C again to exit"),
        "Exit hint should disappear after timeout.\nCapture:\n{}",
        after_timeout
    );
    assert!(
        after_timeout.contains("? for shortcuts"),
        "Should return to normal status bar.\nCapture:\n{}",
        after_timeout
    );
}

// =============================================================================
// Ctrl+C Behavioral Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Double Ctrl+C (quick succession) exits the TUI
#[test]
fn test_tui_ctrl_c_double_press_exits() {
    let tui = TuiTestSession::new("ctrl-c-exit", &scenario());
    let previous = tui.capture();

    // First Ctrl+C shows exit hint
    tui.send_keys("C-c");
    let _ = tui.wait_for_change(&previous);

    // Second Ctrl+C exits
    tui.send_keys("C-c");

    // Wait for shell prompt to appear (indicating exit)
    // Note: ❯ is starship/zsh prompt, $ is bash, % is zsh default
    let capture = tui.wait_for_any(&["$", "%", "❯"]);

    // Verify we exited (shell prompt visible, TUI elements gone or shell visible)
    assert!(
        capture.contains("$") || capture.contains("%") || capture.contains("❯"),
        "Double Ctrl+C should exit TUI and show shell prompt.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// Ctrl+D Exit Hint Rendering Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Ctrl+D on empty input shows "Press Ctrl-D again to exit" in status bar
#[test]
fn test_tui_ctrl_d_shows_exit_hint_on_empty_input() {
    let tui = TuiTestSession::new("ctrl-d-hint", &scenario());
    let previous = tui.capture();

    // Press Ctrl+D on empty input
    tui.send_keys("C-d");
    let capture = tui.wait_for_change(&previous);

    assert!(
        capture.contains("Press Ctrl-D again to exit"),
        "Ctrl+D on empty input should show exit hint.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Ctrl+D exit hint times out and returns to normal status bar
#[test]
fn test_tui_ctrl_d_exit_hint_times_out() {
    let tui = TuiTestSession::new("ctrl-d-timeout", &scenario());
    let previous = tui.capture();

    // Press Ctrl+D to show exit hint
    tui.send_keys("C-d");
    let with_hint = tui.wait_for_change(&previous);
    assert!(
        with_hint.contains("Press Ctrl-D again to exit"),
        "Should show exit hint.\nCapture:\n{}",
        with_hint
    );

    // Wait for timeout (~2 seconds) and check it returns to normal
    // Use 3 second timeout to allow for the ~2 second hint timeout plus buffer
    let after_timeout = tui.wait_for_timeout("? for shortcuts", Duration::from_secs(3));

    assert!(
        !after_timeout.contains("Press Ctrl-D again to exit"),
        "Exit hint should disappear after timeout.\nCapture:\n{}",
        after_timeout
    );
    assert!(
        after_timeout.contains("? for shortcuts"),
        "Should return to normal status bar.\nCapture:\n{}",
        after_timeout
    );
}

// =============================================================================
// Ctrl+D Behavioral Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Ctrl+D with text in input is ignored (does nothing)
#[test]
fn test_tui_ctrl_d_ignored_with_text_in_input() {
    let tui = TuiTestSession::new("ctrl-d-with-text", &scenario());
    let previous = tui.capture();

    // Type some text
    tui.send_keys("some text here");
    let with_text = tui.wait_for_change(&previous);
    assert!(
        with_text.contains("some text here"),
        "Text should appear in input.\nCapture:\n{}",
        with_text
    );

    // Press Ctrl+D - should be ignored (text remains, no exit hint)
    tui.send_keys("C-d");

    // Verify nothing changes for 200ms
    let after_ctrl_d = tui.assert_unchanged_ms(&with_text, 200);

    // Text should still be there
    assert!(
        after_ctrl_d.contains("some text here"),
        "Text should remain after Ctrl+D.\nCapture:\n{}",
        after_ctrl_d
    );
    // No exit hint should appear
    assert!(
        !after_ctrl_d.contains("Press Ctrl-D again to exit"),
        "No exit hint should appear with text in input.\nCapture:\n{}",
        after_ctrl_d
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Double Ctrl+D (quick succession) on empty input exits the TUI
#[test]
fn test_tui_ctrl_d_double_press_exits() {
    let tui = TuiTestSession::new("ctrl-d-exit", &scenario());
    let previous = tui.capture();

    // First Ctrl+D shows exit hint
    tui.send_keys("C-d");
    let _ = tui.wait_for_change(&previous);

    // Second Ctrl+D exits
    tui.send_keys("C-d");

    // Wait for shell prompt to appear (indicating exit)
    let capture = tui.wait_for_any(&["$", "%", "❯"]);

    assert!(
        capture.contains("$") || capture.contains("%") || capture.contains("❯"),
        "Double Ctrl+D should exit TUI and show shell prompt.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /exit Slash Command Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing /exit shows autocomplete dropdown with "Exit the REPL" description
#[test]
fn test_tui_exit_command_shows_autocomplete() {
    let tui = TuiTestSession::new("exit-autocomplete", &scenario());
    let previous = tui.capture();

    // Type /exit
    tui.send_keys("/exit");
    let capture = tui.wait_for_change(&previous);

    assert!(
        capture.contains("/exit") && capture.contains("Exit the REPL"),
        "/exit should show autocomplete with description.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /exit command exits the TUI and shows a farewell message
#[test]
fn test_tui_exit_command_exits_with_farewell() {
    let tui = TuiTestSession::new("exit-command", &scenario());

    // Type /exit and press Enter
    tui.send_keys("/exit");
    let _ = tui.wait_for("Exit the REPL");
    tui.send_keys("Enter");

    // Wait for shell prompt to appear (indicating exit)
    let capture = tui.wait_for_any(&["$", "%", "❯"]);

    // Should show a farewell message (could be "Goodbye!", "Bye!", "See ya!", "Catch you later!", etc.)
    // The farewell is prefixed with "⎿" like other command responses
    let has_farewell = capture.contains("Goodbye!")
        || capture.contains("Bye!")
        || capture.contains("See ya!")
        || capture.contains("Catch you later!");

    assert!(
        has_farewell,
        "/exit should display a farewell message.\nCapture:\n{}",
        capture
    );

    // Should have exited to shell
    assert!(
        capture.contains("$") || capture.contains("%") || capture.contains("❯"),
        "/exit should exit TUI and show shell prompt.\nCapture:\n{}",
        capture
    );
}
