// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! TUI interaction tests - input, response display.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{simple_scenario_toml, TuiTestSession};

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing in the input area should show the typed text
#[test]
fn test_tui_shows_typed_input() {
    let tui = TuiTestSession::new("input-test", &simple_scenario_toml("Hello!"));

    tui.send_keys("Hello Claude");

    let capture = tui.wait_for("Hello Claude");

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
    let tui = TuiTestSession::new(
        "response-test",
        &simple_scenario_toml("Test response from simulator"),
    );

    tui.send_line("test prompt");

    let capture = tui.wait_for("Test response from simulator");

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

// ============================================================================
// Double-tap Escape to clear input
// ============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// When input has text and Escape is pressed once, shows "Esc to clear again" hint
#[test]
fn test_tui_escape_shows_clear_hint_with_input() {
    let tui = TuiTestSession::new(
        "escape-hint",
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );
    let previous = tui.capture();

    // Type some input
    tui.send_keys("Some test input");
    let _ = tui.wait_for_change(&previous);

    // Press Escape once
    tui.send_keys("Escape");
    let capture = tui.wait_for("Esc to clear again");

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
    let tui = TuiTestSession::new(
        "double-escape",
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );
    let previous = tui.capture();

    // Type some input
    tui.send_keys("Text to be cleared");
    let after_input = tui.wait_for_change(&previous);

    // Double-tap Escape quickly
    tui.send_keys("Escape");
    std::thread::sleep(std::time::Duration::from_millis(50));
    tui.send_keys("Escape");

    // Wait for change - input should be cleared
    let capture = tui.wait_for_change(&after_input);

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
    let tui = TuiTestSession::new(
        "escape-empty",
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );
    let initial = tui.capture();

    // Press Escape on empty input
    tui.send_keys("Escape");

    // Use assert_unchanged to verify nothing happens
    let capture = tui.assert_unchanged_ms(&initial, 200);

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
    let tui = TuiTestSession::new(
        "escape-timeout",
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );
    let previous = tui.capture();

    // Type some input
    tui.send_keys("Test input");
    let _ = tui.wait_for_change(&previous);

    // Press Escape once to show hint
    tui.send_keys("Escape");
    let with_hint = tui.wait_for("Esc to clear again");

    // Wait for timeout (~2 seconds)
    std::thread::sleep(std::time::Duration::from_millis(2500));
    let capture = tui.capture();

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
    let tui = TuiTestSession::new(
        "ctrl-underscore-word",
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );
    let previous = tui.capture();

    // Type words with pauses
    tui.send_keys("first");
    std::thread::sleep(std::time::Duration::from_millis(200));
    tui.send_keys(" second");
    std::thread::sleep(std::time::Duration::from_millis(200));
    tui.send_keys(" third");
    let _ = tui.wait_for_change(&previous);

    // Press Ctrl+_ (via Ctrl+/ which produces the same ASCII 31 character)
    tui.send_keys("C-/");
    let after_first_undo = tui.wait_for("first second");

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
    let tui = TuiTestSession::new(
        "ctrl-underscore-clear",
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );
    let previous = tui.capture();

    // Type some text
    tui.send_keys("Hello world");
    let _ = tui.wait_for_change(&previous);

    // Press Ctrl+_ multiple times to clear all (via Ctrl+/)
    tui.send_keys("C-/");
    std::thread::sleep(std::time::Duration::from_millis(100));
    tui.send_keys("C-/");
    let capture = tui.wait_for("? for shortcuts");

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
    let tui = TuiTestSession::new(
        "ctrl-underscore-empty",
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );
    let initial = tui.capture();

    // Press Ctrl+_ on empty input (via Ctrl+/)
    tui.send_keys("C-/");

    // Use assert_unchanged to verify nothing happens
    let capture = tui.assert_unchanged_ms(&initial, 200);

    // Should still show initial state
    assert!(
        capture.contains("? for shortcuts") || capture.contains("Try"),
        "Empty input should remain unchanged after Ctrl+_.\nCapture:\n{}",
        capture
    );
}
