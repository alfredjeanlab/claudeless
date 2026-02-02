// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

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

use common::{simple_scenario_toml, TuiTestSession};

fn scenario() -> String {
    simple_scenario_toml("Hello!")
}

// =============================================================================
// Shortcuts Display Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Pressing '?' on empty input shows the shortcuts panel with all available shortcuts
#[test]
fn test_tui_question_mark_shows_shortcuts_on_empty_input() {
    let tui = TuiTestSession::new("shortcuts-empty", &scenario());
    let previous = tui.capture();

    // Press '?' to show shortcuts
    tui.send_keys("?");
    let capture = tui.wait_for_change(&previous);

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
/// Pressing Escape dismisses the shortcuts panel
#[test]
fn test_tui_escape_dismisses_shortcuts_panel() {
    let tui = TuiTestSession::new("shortcuts-dismiss", &scenario());
    let previous = tui.capture();

    // Press '?' to show shortcuts
    tui.send_keys("?");
    let after_question = tui.wait_for_change(&previous);

    // Verify shortcuts are shown
    assert!(
        after_question.contains("! for bash mode"),
        "Shortcuts should be visible.\nCapture:\n{}",
        after_question
    );

    // Press Escape to dismiss
    tui.send_keys("Escape");
    let after_escape = tui.wait_for_change(&after_question);

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
#[test]
fn test_tui_question_mark_types_literal_when_input_present() {
    let tui = TuiTestSession::new("shortcuts-literal", &scenario());

    // Type some text first
    tui.send_keys("Hello");
    let _after_hello = tui.wait_for("Hello");

    // Now press '?' - should type literal '?', not show shortcuts
    tui.send_keys("?");
    let capture = tui.wait_for("Hello?");

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
