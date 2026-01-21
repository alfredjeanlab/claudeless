// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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

use common::{start_tui, tmux, write_scenario};

// =============================================================================
// Stash Behavior Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// When Ctrl+S is pressed with text in the input, the text is stashed
/// and a message is displayed.
// TODO(implement): requires Ctrl+S stash handler
#[test]
#[ignore]
fn test_tui_ctrl_s_stashes_prompt_with_message() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-stash-message";
    let previous = start_tui(session, &scenario);

    // Type some text
    tmux::send_keys(session, "hello world stash test");
    let with_input = tmux::wait_for_change(session, &previous);

    // Press Ctrl+S to stash
    tmux::send_keys(session, "C-s");
    let capture = tmux::wait_for_change(session, &with_input);

    tmux::kill_session(session);

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
// TODO(implement): requires Ctrl+S stash/restore toggle
#[test]
#[ignore]
fn test_tui_ctrl_s_restores_stashed_prompt() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-stash-restore";
    let previous = start_tui(session, &scenario);

    // Type some text
    tmux::send_keys(session, "my stashed prompt");
    let with_input = tmux::wait_for_change(session, &previous);

    // Press Ctrl+S to stash
    tmux::send_keys(session, "C-s");
    let stashed = tmux::wait_for_change(session, &with_input);

    // Press Ctrl+S again to restore
    tmux::send_keys(session, "C-s");
    let capture = tmux::wait_for_change(session, &stashed);

    tmux::kill_session(session);

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
// TODO(implement): requires Ctrl+S handler to check for empty input
#[test]
#[ignore]
fn test_tui_ctrl_s_empty_input_does_nothing() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-stash-empty";
    let previous = start_tui(session, &scenario);

    // Press Ctrl+S on empty input
    tmux::send_keys(session, "C-s");

    // Screen should not change (nothing to stash)
    let capture = tmux::assert_unchanged_ms(session, &previous, 300);

    tmux::kill_session(session);

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
// TODO(implement): requires persistent stash indicator
#[test]
#[ignore]
fn test_tui_stash_message_persists() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-stash-persist";
    let previous = start_tui(session, &scenario);

    // Type and stash
    tmux::send_keys(session, "persistent stash");
    let with_input = tmux::wait_for_change(session, &previous);

    tmux::send_keys(session, "C-s");
    let stashed = tmux::wait_for_change(session, &with_input);

    // Wait and verify message persists
    std::thread::sleep(std::time::Duration::from_secs(2));
    let capture = tmux::capture_pane(session);

    tmux::kill_session(session);

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
// TODO(implement): requires stash auto-restore after response
#[test]
#[ignore]
fn test_tui_stash_auto_restores_after_submit() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Quick response"
        "#,
    );

    let session = "claudeless-stash-auto-restore";
    let previous = start_tui(session, &scenario);

    // Type and stash the main prompt
    tmux::send_keys(session, "my stashed prompt");
    let with_input = tmux::wait_for_change(session, &previous);

    tmux::send_keys(session, "C-s");
    let stashed = tmux::wait_for_change(session, &with_input);

    // Type and submit a different prompt
    tmux::send_keys(session, "say hello");
    let new_input = tmux::wait_for_change(session, &stashed);

    tmux::send_keys(session, "Enter");

    // Wait for response and auto-restore
    let capture = tmux::wait_for_content(session, "my stashed prompt");

    tmux::kill_session(session);

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
