// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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

use common::{start_tui, tmux, write_scenario};

// =============================================================================
// Ctrl+C Exit Hint Rendering Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// First Ctrl+C on empty input shows "Press Ctrl-C again to exit" in status bar
#[test]
fn test_tui_ctrl_c_shows_exit_hint_on_empty_input() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-ctrl-c-hint-empty";
    let previous = start_tui(session, &scenario);

    // Press Ctrl+C on empty input
    tmux::send_keys(session, "C-c");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

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
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-ctrl-c-hint-text";
    let previous = start_tui(session, &scenario);

    // Type some text
    tmux::send_keys(session, "hello world test");
    let with_text = tmux::wait_for_change(session, &previous);
    assert!(
        with_text.contains("hello world test"),
        "Text should appear in input.\nCapture:\n{}",
        with_text
    );

    // Press Ctrl+C - should clear input AND show exit hint
    tmux::send_keys(session, "C-c");
    let after_ctrl_c = tmux::wait_for_change(session, &with_text);

    tmux::kill_session(session);

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
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-ctrl-c-timeout";
    let previous = start_tui(session, &scenario);

    // Press Ctrl+C to show exit hint
    tmux::send_keys(session, "C-c");
    let with_hint = tmux::wait_for_change(session, &previous);
    assert!(
        with_hint.contains("Press Ctrl-C again to exit"),
        "Should show exit hint.\nCapture:\n{}",
        with_hint
    );

    // Wait for timeout (~2 seconds) and check it returns to normal
    // Use 3 second timeout to allow for the ~2 second hint timeout plus buffer
    let after_timeout =
        tmux::wait_for_content_timeout(session, "? for shortcuts", Duration::from_secs(3));

    tmux::kill_session(session);

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
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-ctrl-c-exit";
    let previous = start_tui(session, &scenario);

    // First Ctrl+C shows exit hint
    tmux::send_keys(session, "C-c");
    let _ = tmux::wait_for_change(session, &previous);

    // Second Ctrl+C exits
    tmux::send_keys(session, "C-c");

    // Wait for shell prompt to appear (indicating exit)
    // Note: ❯ is starship/zsh prompt, $ is bash, % is zsh default
    let capture = tmux::wait_for_any(session, &["$", "%", "❯"]);

    tmux::kill_session(session);

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
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-ctrl-d-hint";
    let previous = start_tui(session, &scenario);

    // Press Ctrl+D on empty input
    tmux::send_keys(session, "C-d");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

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
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-ctrl-d-timeout";
    let previous = start_tui(session, &scenario);

    // Press Ctrl+D to show exit hint
    tmux::send_keys(session, "C-d");
    let with_hint = tmux::wait_for_change(session, &previous);
    assert!(
        with_hint.contains("Press Ctrl-D again to exit"),
        "Should show exit hint.\nCapture:\n{}",
        with_hint
    );

    // Wait for timeout (~2 seconds) and check it returns to normal
    // Use 3 second timeout to allow for the ~2 second hint timeout plus buffer
    let after_timeout =
        tmux::wait_for_content_timeout(session, "? for shortcuts", Duration::from_secs(3));

    tmux::kill_session(session);

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
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-ctrl-d-with-text";
    let previous = start_tui(session, &scenario);

    // Type some text
    tmux::send_keys(session, "some text here");
    let with_text = tmux::wait_for_change(session, &previous);
    assert!(
        with_text.contains("some text here"),
        "Text should appear in input.\nCapture:\n{}",
        with_text
    );

    // Press Ctrl+D - should be ignored (text remains, no exit hint)
    tmux::send_keys(session, "C-d");

    // Verify nothing changes for 200ms
    let after_ctrl_d = tmux::assert_unchanged_ms(session, &with_text, 200);

    tmux::kill_session(session);

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
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-ctrl-d-exit";
    let previous = start_tui(session, &scenario);

    // First Ctrl+D shows exit hint
    tmux::send_keys(session, "C-d");
    let _ = tmux::wait_for_change(session, &previous);

    // Second Ctrl+D exits
    tmux::send_keys(session, "C-d");

    // Wait for shell prompt to appear (indicating exit)
    let capture = tmux::wait_for_any(session, &["$", "%", "❯"]);

    tmux::kill_session(session);

    assert!(
        capture.contains("$") || capture.contains("%") || capture.contains("❯"),
        "Double Ctrl+D should exit TUI and show shell prompt.\nCapture:\n{}",
        capture
    );
}
