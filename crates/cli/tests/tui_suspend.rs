// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TUI suspend tests - Ctrl+Z process suspension behavior.
//!
//! Behavior observed with: claude --version 2.1.14 (Claude Code)
//!
//! ## Ctrl+Z Suspend Behavior
//! - When Ctrl+Z is pressed, Claude Code receives SIGTSTP and suspends
//! - Before suspending, prints: "Claude Code has been suspended. Run `fg` to bring Claude Code back."
//! - Also prints: "Note: ctrl + z now suspends Claude Code, ctrl + _ undoes input."
//! - The shell shows the standard suspend message (e.g., "zsh: suspended (signal) claude ...")
//! - Running `fg` resumes Claude Code, which redraws its TUI with state preserved
//!
//! ## Testing Notes
//! Ctrl+Z testing is complex because:
//! - It sends SIGTSTP which suspends the process
//! - The process returns control to the shell
//! - Resume requires interactive `fg` command
//! - Full suspend/resume cycle testing requires shell job control

mod common;

use common::{start_tui, tmux, write_scenario};

// =============================================================================
// Suspend Behavior Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// When Ctrl+Z is pressed, Claude Code suspends and prints a helpful message
/// telling the user how to resume.
#[test]
fn test_tui_ctrl_z_suspends_with_message() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-suspend-message";
    let _previous = start_tui(session, &scenario);

    // Send Ctrl+Z to suspend
    tmux::send_keys(session, "C-z");

    // Wait for the suspend message to appear
    // Note: After suspend, the shell prompt should appear
    let capture = tmux::wait_for_content(session, "Claude Code has been suspended");

    tmux::kill_session(session);

    // Should show the suspend message
    assert!(
        capture.contains("Claude Code has been suspended"),
        "Should show suspend message.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Run `fg` to bring Claude Code back"),
        "Should tell user how to resume.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// The suspend message includes a note about the Ctrl+Z keybinding change.
#[test]
fn test_tui_ctrl_z_shows_keybinding_note() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-suspend-note";
    let _previous = start_tui(session, &scenario);

    // Send Ctrl+Z to suspend
    tmux::send_keys(session, "C-z");

    // Wait for the suspend message
    let capture = tmux::wait_for_content(session, "ctrl + z now suspends");

    tmux::kill_session(session);

    // Should show the keybinding note
    assert!(
        capture.contains("Note: ctrl + z now suspends Claude Code, ctrl + _ undoes input"),
        "Should show keybinding note.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// After Ctrl+Z, the shell prompt appears (process is suspended).
#[test]
fn test_tui_ctrl_z_returns_to_shell() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-suspend-shell";
    let _previous = start_tui(session, &scenario);

    // Send Ctrl+Z to suspend
    tmux::send_keys(session, "C-z");

    // Wait for the shell prompt or suspend indicator
    // Note: The exact shell prompt varies, but we should see "suspended" in the output
    let capture = tmux::wait_for_any(session, &["suspended", "$", "%", "❯"]);

    tmux::kill_session(session);

    // Should have returned to shell (suspended message from shell)
    assert!(
        capture.contains("suspended"),
        "Should show shell's suspended message.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// Resume Behavior Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// After being suspended with Ctrl+Z and resumed with `fg`,
/// Claude Code redraws its TUI interface.
#[test]
fn test_tui_ctrl_z_resume_redraws_tui() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-suspend-resume";
    let _previous = start_tui(session, &scenario);

    // Send Ctrl+Z to suspend
    tmux::send_keys(session, "C-z");

    // Wait for suspend
    tmux::wait_for_content(session, "suspended");

    // Resume with fg
    tmux::send_line(session, "fg");

    // Wait for TUI to redraw
    let capture = tmux::wait_for_content(session, "? for shortcuts");

    tmux::kill_session(session);

    // Should have redrawn the TUI
    assert!(
        capture.contains("? for shortcuts"),
        "TUI should redraw after resume.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// After resume, any input text that was in the prompt is preserved.
#[test]
fn test_tui_ctrl_z_resume_preserves_input_state() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-suspend-preserve";
    let _previous = start_tui(session, &scenario);

    // Type some text
    tmux::send_keys(session, "hello world");
    tmux::wait_for_content(session, "hello world");

    // Send Ctrl+Z to suspend
    tmux::send_keys(session, "C-z");
    tmux::wait_for_content(session, "suspended");

    // Resume with fg
    tmux::send_line(session, "fg");

    // Wait for TUI to redraw
    let capture = tmux::wait_for_content(session, "hello world");

    tmux::kill_session(session);

    // Input text should be preserved
    assert!(
        capture.contains("hello world"),
        "Input text should be preserved after resume.\nCapture:\n{}",
        capture
    );
}
