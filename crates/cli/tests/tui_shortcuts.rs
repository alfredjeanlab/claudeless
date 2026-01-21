// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TUI shortcuts tests - keyboard shortcut display and handling.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## '?' Shortcut Behavior
//! - When input is empty, pressing '?' shows the shortcuts panel
//! - The shortcuts panel displays keyboard shortcuts in columns:
//!   - ! for bash mode
//!   - / for commands
//!   - @ for file paths
//!   - & for background
//!   - double tap esc to clear input
//!   - shift + tab to auto-accept edits
//!   - ctrl + o for verbose output
//!   - ctrl + t to show todos
//!   - backslash (\) + return for newline
//!   - ctrl + _ to undo
//!   - ctrl + z to suspend
//!   - cmd + v to paste images
//!   - meta + p to switch model
//!   - ctrl + s to stash prompt
//! - Pressing Escape dismisses the shortcuts panel
//! - When input is NOT empty, '?' types a literal '?' character

mod common;

use common::{assert_tui_matches_fixture, start_tui, tmux, write_scenario};

// =============================================================================
// Shortcuts Display Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Pressing '?' on empty input shows the shortcuts panel with all available shortcuts
// TODO(implement): requires shortcuts panel display
#[test]
#[ignore]
fn test_tui_question_mark_shows_shortcuts_on_empty_input() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-shortcuts-empty";
    let previous = start_tui(session, &scenario);

    // Press '?' to show shortcuts
    tmux::send_keys(session, "?");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    // Should show shortcuts panel with key bindings
    assert!(
        capture.contains("! for bash mode"),
        "Shortcuts panel should show '! for bash mode'.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("/ for commands"),
        "Shortcuts panel should show '/ for commands'.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("@ for file paths"),
        "Shortcuts panel should show '@ for file paths'.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Shortcuts panel matches the captured fixture
// TODO(implement): requires shortcuts panel display
#[test]
#[ignore]
fn test_tui_shortcuts_display_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.12"
        }
        "#,
    );

    let session = "claudeless-shortcuts-fixture";
    let previous = start_tui(session, &scenario);

    // Press '?' to show shortcuts
    tmux::send_keys(session, "?");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "shortcuts_display.txt", None);
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Pressing Escape dismisses the shortcuts panel
// TODO(implement): requires shortcuts panel display and dismiss
#[test]
#[ignore]
fn test_tui_escape_dismisses_shortcuts_panel() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-shortcuts-dismiss";
    let previous = start_tui(session, &scenario);

    // Press '?' to show shortcuts
    tmux::send_keys(session, "?");
    let after_question = tmux::wait_for_change(session, &previous);

    // Verify shortcuts are shown
    assert!(
        after_question.contains("! for bash mode"),
        "Shortcuts should be visible.\nCapture:\n{}",
        after_question
    );

    // Press Escape to dismiss
    tmux::send_keys(session, "Escape");
    let after_escape = tmux::wait_for_change(session, &after_question);

    tmux::kill_session(session);

    // Should be back to normal state without shortcuts panel
    assert!(
        !after_escape.contains("! for bash mode"),
        "Shortcuts panel should be dismissed after Escape.\nCapture:\n{}",
        after_escape
    );
    assert!(
        after_escape.contains("? for shortcuts"),
        "Should show '? for shortcuts' hint after dismissing panel.\nCapture:\n{}",
        after_escape
    );
}

// =============================================================================
// '?' Literal Input Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// When input is not empty, '?' types a literal '?' instead of showing shortcuts
// TODO(implement): requires conditional '?' behavior based on input state
#[test]
#[ignore]
fn test_tui_question_mark_types_literal_when_input_present() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-shortcuts-literal";
    let _previous = start_tui(session, &scenario);

    // Type some text first
    tmux::send_keys(session, "Hello");
    let _after_hello = tmux::wait_for_content(session, "Hello");

    // Now press '?' - should type literal '?', not show shortcuts
    tmux::send_keys(session, "?");
    let capture = tmux::wait_for_content(session, "Hello?");

    tmux::kill_session(session);

    // Should show "Hello?" in input, NOT the shortcuts panel
    assert!(
        capture.contains("Hello?"),
        "Should show 'Hello?' in input when '?' is typed with existing text.\nCapture:\n{}",
        capture
    );
    assert!(
        !capture.contains("! for bash mode"),
        "Should NOT show shortcuts panel when input is not empty.\nCapture:\n{}",
        capture
    );
}
