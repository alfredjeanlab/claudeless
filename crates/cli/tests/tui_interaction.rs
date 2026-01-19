// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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

    let session = "claudeless-input-test";
    start_tui(session, &scenario);

    tmux::send_keys(session, "Hello Claude");

    let capture = tmux::wait_for_content(session, "Hello Claude");

    tmux::send_keys(session, "C-c");
    tmux::send_keys(session, "C-c");
    tmux::kill_session(session);

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

    let session = "claudeless-response-test";
    start_tui(session, &scenario);

    tmux::send_line(session, "test prompt");

    let capture = tmux::wait_for_content(session, "Test response from simulator");

    tmux::send_keys(session, "C-c");
    tmux::send_keys(session, "C-c");
    tmux::kill_session(session);

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
            "trusted": true
        }
        "#,
    );

    let session = "claudeless-fixture-response";
    start_tui(session, &scenario);

    // Send a prompt
    tmux::send_line(session, "Hello");

    // Wait for response to appear
    let capture = tmux::wait_for_content(session, "Hello there friend");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "after_response.txt", None);
}

/// Compare input display against real Claude fixture
#[test]
#[ignore] // TODO(implement): Simulator shows status bar while fixture does not
fn test_input_display_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = "claudeless-fixture-input";
    start_tui(session, &scenario);

    // Type input text (without sending/Enter)
    tmux::send_keys(session, "Say hello in exactly 3 words");

    // Wait for the typed text to appear
    let capture = tmux::wait_for_content(session, "Say hello in exactly 3 words");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "with_input.txt", None);
}
