// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! TUI interaction tests - input, response display.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{assert_tui_matches_fixture, start_tui, tmux, write_scenario};

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing in the input area should show the typed text
#[test]
fn test_tui_shows_typed_input() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = tmux::unique_session("input-test");
    start_tui(&session, &scenario);

    tmux::send_keys(&session, "Hello Claude");

    let capture = tmux::wait_for_content(&session, "Hello Claude");

    tmux::send_keys(&session, "C-c");
    tmux::send_keys(&session, "C-c");
    tmux::kill_session(&session);

    assert!(
        capture.contains("Hello Claude"),
        "TUI should show typed input.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// After submitting, the response should appear with "⏺" prefix
#[test]
fn test_tui_shows_response_with_indicator() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Test response from simulator"
        "#,
    );

    let session = tmux::unique_session("response-test");
    start_tui(&session, &scenario);

    tmux::send_line(&session, "test prompt");

    let capture = tmux::wait_for_content(&session, "Test response from simulator");

    tmux::send_keys(&session, "C-c");
    tmux::send_keys(&session, "C-c");
    tmux::kill_session(&session);

    assert!(
        capture.contains("Test response from simulator"),
        "TUI should show response.\nCapture:\n{}",
        capture
    );

    assert!(
        capture.contains("⏺") || capture.contains("●") || capture.contains("*"),
        "TUI should show response indicator (⏺ or similar).\nCapture:\n{}",
        capture
    );
}

/// Compare response format against real Claude fixture
#[test]
fn test_response_format_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello there friend.",
            "trusted": true,
            "claude_version": "2.1.12"
        }
        "#,
    );

    let session = tmux::unique_session("fixture-response");
    start_tui(&session, &scenario);

    // Send a prompt
    tmux::send_line(&session, "Hello");

    // Wait for response to appear
    let capture = tmux::wait_for_content(&session, "Hello there friend");

    tmux::kill_session(&session);

    assert_tui_matches_fixture(&capture, "after_response.txt", None);
}

/// Compare input display against real Claude fixture
#[test]
#[ignore] // TODO(slash-cleanup): Simulator shows status bar while fixture does not
fn test_input_display_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = tmux::unique_session("fixture-input");
    start_tui(&session, &scenario);

    // Type input text (without sending/Enter)
    tmux::send_keys(&session, "Say hello in exactly 3 words");

    // Wait for the typed text to appear
    let capture = tmux::wait_for_content(&session, "Say hello in exactly 3 words");

    tmux::kill_session(&session);

    assert_tui_matches_fixture(&capture, "with_input.txt", None);
}

// ============================================================================
// Double-tap Escape to clear input
// ============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// When input has text and Escape is pressed once, shows "Esc to clear again" hint
#[test]
fn test_tui_escape_shows_clear_hint_with_input() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = tmux::unique_session("escape-hint");
    let previous = start_tui(&session, &scenario);

    // Type some input
    tmux::send_keys(&session, "Some test input");
    let _ = tmux::wait_for_change(&session, &previous);

    // Press Escape once
    tmux::send_keys(&session, "Escape");
    let capture = tmux::wait_for_content(&session, "Esc to clear again");

    tmux::kill_session(&session);

    assert!(
        capture.contains("Esc to clear again"),
        "First Escape should show 'Esc to clear again' hint.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Some test input"),
        "Input should still be present after first Escape.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Double-tap Escape clears the input field
#[test]
fn test_tui_double_escape_clears_input() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = tmux::unique_session("double-escape");
    let previous = start_tui(&session, &scenario);

    // Type some input
    tmux::send_keys(&session, "Text to be cleared");
    let after_input = tmux::wait_for_change(&session, &previous);

    // Double-tap Escape quickly
    tmux::send_keys(&session, "Escape");
    std::thread::sleep(std::time::Duration::from_millis(50));
    tmux::send_keys(&session, "Escape");

    // Wait for change - input should be cleared
    let capture = tmux::wait_for_change(&session, &after_input);

    tmux::kill_session(&session);

    assert!(
        !capture.contains("Text to be cleared"),
        "Input should be cleared after double-tap Escape.\nCapture:\n{}",
        capture
    );
    // Should show placeholder again
    assert!(
        capture.contains("Try") || capture.contains("for shortcuts"),
        "Should show initial state after clearing input.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Escape on empty input does nothing (no hint shown)
#[test]
fn test_tui_escape_on_empty_input_does_nothing() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = tmux::unique_session("escape-empty");
    let initial = start_tui(&session, &scenario);

    // Press Escape on empty input
    tmux::send_keys(&session, "Escape");

    // Use assert_unchanged to verify nothing happens
    let capture = tmux::assert_unchanged_ms(&session, &initial, 200);

    tmux::kill_session(&session);

    assert!(
        !capture.contains("Esc to clear"),
        "Escape on empty input should not show clear hint.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// The "Esc to clear again" hint times out after ~2 seconds
#[test]
fn test_tui_escape_clear_hint_timeout() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = tmux::unique_session("escape-timeout");
    let previous = start_tui(&session, &scenario);

    // Type some input
    tmux::send_keys(&session, "Test input");
    let _ = tmux::wait_for_change(&session, &previous);

    // Press Escape once to show hint
    tmux::send_keys(&session, "Escape");
    let with_hint = tmux::wait_for_content(&session, "Esc to clear again");

    // Wait for timeout (~2 seconds)
    std::thread::sleep(std::time::Duration::from_millis(2500));
    let capture = tmux::capture_pane(&session);

    tmux::kill_session(&session);

    assert!(
        with_hint.contains("Esc to clear again"),
        "Hint should appear after first Escape.\nCapture:\n{}",
        with_hint
    );
    assert!(
        !capture.contains("Esc to clear again"),
        "Hint should disappear after timeout.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Test input"),
        "Input should still be present after timeout (not cleared).\nCapture:\n{}",
        capture
    );
}

// ============================================================================
// Ctrl+_ to undo input
// ============================================================================

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Pressing Ctrl+_ (undo) when input has text removes the last typed "word" or segment.
/// When multiple words are typed with pauses, each undo removes the most recent segment.
/// Example: "first second third" → Ctrl+_ → "first second" → Ctrl+_ → empty
///
/// NOTE: This test is ignored due to tmux key encoding issues with Ctrl+_.
/// The functionality is verified by unit tests in app_tests.rs.
#[test]
#[ignore = "tmux cannot reliably send Ctrl+_ - unit tests verify this behavior"]
fn test_tui_ctrl_underscore_undoes_last_word() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = tmux::unique_session("ctrl-underscore-word");
    let previous = start_tui(&session, &scenario);

    // Type words with pauses
    tmux::send_keys(&session, "first");
    std::thread::sleep(std::time::Duration::from_millis(200));
    tmux::send_keys(&session, " second");
    std::thread::sleep(std::time::Duration::from_millis(200));
    tmux::send_keys(&session, " third");
    let _ = tmux::wait_for_change(&session, &previous);

    // Press Ctrl+_ (via Ctrl+/ which produces the same ASCII 31 character)
    tmux::send_keys(&session, "C-/");
    let after_first_undo = tmux::wait_for_content(&session, "first second");

    tmux::kill_session(&session);

    // Should have removed "third"
    assert!(
        after_first_undo.contains("first second"),
        "First undo should keep 'first second'.\nCapture:\n{}",
        after_first_undo
    );
    assert!(
        !after_first_undo.contains("third"),
        "First undo should remove 'third'.\nCapture:\n{}",
        after_first_undo
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Pressing Ctrl+_ repeatedly undoes all input, returning to empty state
///
/// NOTE: This test is ignored due to tmux key encoding issues with Ctrl+_.
/// The functionality is verified by unit tests in app_tests.rs.
#[test]
#[ignore = "tmux cannot reliably send Ctrl+_ - unit tests verify this behavior"]
fn test_tui_ctrl_underscore_clears_all_input() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = tmux::unique_session("ctrl-underscore-clear");
    let previous = start_tui(&session, &scenario);

    // Type some text
    tmux::send_keys(&session, "Hello world");
    let _ = tmux::wait_for_change(&session, &previous);

    // Press Ctrl+_ multiple times to clear all (via Ctrl+/)
    tmux::send_keys(&session, "C-/");
    std::thread::sleep(std::time::Duration::from_millis(100));
    tmux::send_keys(&session, "C-/");
    let capture = tmux::wait_for_content(&session, "? for shortcuts");

    tmux::kill_session(&session);

    // Input should be cleared
    assert!(
        !capture.contains("Hello world"),
        "All input should be cleared after multiple Ctrl+_.\nCapture:\n{}",
        capture
    );
    // Should show placeholder
    assert!(
        capture.contains("Try") || capture.contains("? for shortcuts"),
        "Should show initial placeholder after clearing.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Pressing Ctrl+_ on empty input does nothing
#[test]
fn test_tui_ctrl_underscore_on_empty_input_does_nothing() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = tmux::unique_session("ctrl-underscore-empty");
    let initial = start_tui(&session, &scenario);

    // Press Ctrl+_ on empty input (via Ctrl+/)
    tmux::send_keys(&session, "C-/");

    // Use assert_unchanged to verify nothing happens
    let capture = tmux::assert_unchanged_ms(&session, &initial, 200);

    tmux::kill_session(&session);

    // Should still show initial state
    assert!(
        capture.contains("? for shortcuts") || capture.contains("Try"),
        "Empty input should remain unchanged after Ctrl+_.\nCapture:\n{}",
        capture
    );
}
