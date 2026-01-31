// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! TUI stash prompt tests - Ctrl+S to stash and restore prompts.
//!
//! Behavior observed with: claude --version 2.1.14 (Claude Code)
//!
//! ## Ctrl+S Stash Behavior
//! - When Ctrl+S is pressed with text in the input, the text is stashed
//! - The input field is cleared (returns to placeholder state)
//! - Shows "› Stashed (auto-restores after submit)" message above the input area
//! - When Ctrl+S is pressed again while a stash exists, the stashed text is restored
//! - After submitting a prompt and receiving a response, the stashed text auto-restores
//! - Ctrl+S on empty input does nothing (nothing to stash)

mod common;

use common::{simple_scenario_toml, TuiTestSession};

fn scenario() -> String {
    simple_scenario_toml("Hello!")
}

// =============================================================================
// Stash Behavior Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// When Ctrl+S is pressed with text in the input, the text is stashed
/// and a message is displayed.
#[test]
fn test_tui_ctrl_s_stashes_prompt_with_message() {
    let tui = TuiTestSession::new("stash-message", &scenario());
    let previous = tui.capture();

    // Type some text
    tui.send_keys("hello world stash test");
    let with_input = tui.wait_for_change(&previous);

    // Press Ctrl+S to stash
    tui.send_keys("C-s");
    let capture = tui.wait_for_change(&with_input);

    // Should show the stash message
    assert!(
        capture.contains("Stashed (auto-restores after submit)"),
        "Should show stash message.\nCapture:\n{}",
        capture
    );

    // Input should be cleared (shows placeholder)
    assert!(
        capture.contains("Try "),
        "Input should be cleared after stash.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// When Ctrl+S is pressed again while a stash exists, the stashed text
/// is restored to the input field.
#[test]
fn test_tui_ctrl_s_restores_stashed_prompt() {
    let tui = TuiTestSession::new("stash-restore", &scenario());
    let previous = tui.capture();

    // Type some text
    tui.send_keys("my stashed prompt");
    let _with_input = tui.wait_for_change(&previous);

    // Press Ctrl+S to stash
    tui.send_keys("C-s");
    let stashed = tui.wait_for("Stashed (auto-restores after submit)");

    // Verify stash state was captured correctly
    assert!(
        stashed.contains("Stashed (auto-restores after submit)"),
        "Stash indicator should appear after Ctrl+S.\nStashed:\n{}",
        stashed
    );

    // Press Ctrl+S again to restore
    tui.send_keys("C-s");
    let capture = tui.wait_for("my stashed prompt");

    // The stashed text should be restored
    assert!(
        capture.contains("my stashed prompt"),
        "Stashed text should be restored.\nCapture:\n{}",
        capture
    );

    // Stash message should be gone
    assert!(
        !capture.contains("Stashed (auto-restores after submit)"),
        "Stash message should disappear after restore.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Ctrl+S on empty input does nothing - there's nothing to stash.
#[test]
fn test_tui_ctrl_s_empty_input_does_nothing() {
    let tui = TuiTestSession::new("stash-empty", &scenario());
    let previous = tui.capture();

    // Press Ctrl+S on empty input
    tui.send_keys("C-s");

    // Screen should not change (nothing to stash)
    let capture = tui.assert_unchanged_ms(&previous, 300);

    // Should not show stash message
    assert!(
        !capture.contains("Stashed"),
        "Should not show stash message on empty input.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// The stash message "› Stashed (auto-restores after submit)" persists
/// until the user restores the stash or submits a prompt.
#[test]
fn test_tui_stash_message_persists() {
    let tui = TuiTestSession::new("stash-persist", &scenario());
    let previous = tui.capture();

    // Type and stash
    tui.send_keys("persistent stash");
    let with_input = tui.wait_for_change(&previous);

    tui.send_keys("C-s");
    let stashed = tui.wait_for_change(&with_input);

    // Wait and verify message persists
    std::thread::sleep(std::time::Duration::from_secs(2));
    let capture = tui.capture();

    // Stash message should still be visible
    assert!(
        capture.contains("Stashed (auto-restores after submit)"),
        "Stash message should persist.\nCapture:\n{}",
        capture
    );

    // Verify initial stash was shown correctly
    assert!(
        stashed.contains("Stashed"),
        "Stash message should have been shown.\nCapture:\n{}",
        stashed
    );
}

// =============================================================================
// Auto-Restore Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// After submitting a prompt and receiving a response, the stashed text
/// is automatically restored to the input field.
#[test]
fn test_tui_stash_auto_restores_after_submit() {
    let tui = TuiTestSession::new(
        "stash-auto-restore",
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Quick response"
        "#,
    );
    let previous = tui.capture();

    // Type and stash the main prompt
    tui.send_keys("my stashed prompt");
    let with_input = tui.wait_for_change(&previous);

    tui.send_keys("C-s");
    let stashed = tui.wait_for_change(&with_input);

    // Type and submit a different prompt
    tui.send_keys("say hello");
    let new_input = tui.wait_for_change(&stashed);

    tui.send_keys("Enter");

    // Wait for response and auto-restore
    let capture = tui.wait_for("my stashed prompt");

    // Verify new prompt was submitted
    assert!(
        new_input.contains("say hello"),
        "New prompt should have been typed.\nCapture:\n{}",
        new_input
    );

    // The stashed text should be auto-restored after the response
    assert!(
        capture.contains("my stashed prompt"),
        "Stashed text should auto-restore after submit.\nCapture:\n{}",
        capture
    );
}
