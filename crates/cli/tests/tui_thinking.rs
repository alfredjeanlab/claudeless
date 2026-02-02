// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! Thinking toggle behavioral tests - Meta+t dialog behavior.
//!
//! Tests user interactions with the thinking toggle dialog.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::capture_key_sequence;

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Meta+t opens the thinking toggle dialog
#[test]
fn test_meta_t_opens_thinking_dialog() {
    let captures = capture_key_sequence("claudeless-thinking-open", &["M-t"]);

    assert!(captures.len() >= 2, "Should have at least 2 captures");

    let after_meta_t = &captures[1];

    assert!(
        after_meta_t.contains("Toggle thinking mode"),
        "Meta+t should open thinking dialog.\nCapture:\n{}",
        after_meta_t
    );

    assert!(
        after_meta_t.contains("Enabled") && after_meta_t.contains("Disabled"),
        "Dialog should show Enabled and Disabled options.\nCapture:\n{}",
        after_meta_t
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Down arrow moves selection in thinking dialog
#[test]
fn test_thinking_dialog_down_arrow_moves_selection() {
    let captures = capture_key_sequence("claudeless-thinking-down", &["M-t", "Down"]);

    assert!(captures.len() >= 3, "Should have at least 3 captures");

    let after_meta_t = &captures[1];
    let after_down = &captures[2];

    assert!(
        after_meta_t.contains("❯ 1. Enabled") || after_meta_t.contains("❯ 1."),
        "Initially Enabled should be selected.\nCapture:\n{}",
        after_meta_t
    );

    assert!(
        after_down.contains("❯ 2. Disabled") || after_down.contains("❯ 2."),
        "After Down, Disabled should be selected.\nCapture:\n{}",
        after_down
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Enter confirms selection and shows "Thinking off" in status
#[test]
fn test_thinking_toggle_to_disabled() {
    let captures = capture_key_sequence("claudeless-thinking-off", &["M-t", "Down", "Enter"]);

    assert!(captures.len() >= 4, "Should have at least 4 captures");

    let after_confirm = &captures[3];

    assert!(
        after_confirm.contains("Thinking off"),
        "After disabling, status should show 'Thinking off'.\nCapture:\n{}",
        after_confirm
    );

    assert!(
        !after_confirm.contains("Toggle thinking mode"),
        "Dialog should be closed after confirmation.\nCapture:\n{}",
        after_confirm
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Escape closes dialog without changing setting
#[test]
fn test_thinking_dialog_escape_cancels() {
    let captures = capture_key_sequence("claudeless-thinking-esc", &["M-t", "Down", "Escape"]);

    assert!(captures.len() >= 4, "Should have at least 4 captures");

    let after_escape = &captures[3];

    assert!(
        !after_escape.contains("Toggle thinking mode"),
        "Dialog should be closed after Escape.\nCapture:\n{}",
        after_escape
    );

    assert!(
        !after_escape.contains("Thinking off"),
        "Setting should not change after Escape.\nCapture:\n{}",
        after_escape
    );
}
