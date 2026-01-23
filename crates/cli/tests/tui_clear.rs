// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Clear tests - /clear command behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{assert_tui_matches_fixture, start_tui, tmux, write_scenario};

/// Compare conversation state before /clear against fixture
#[test]
fn test_clear_before_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "contains", "text": "2 + 2" }, "response": "2 + 2 = 4" },
                { "pattern": { "type": "contains", "text": "3 + 3" }, "response": "3 + 3 = 6" },
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-fixture-clear-before";
    start_tui(session, &scenario);

    // Build up conversation
    tmux::send_line(session, "what is 2 + 2?");
    tmux::wait_for_content(session, "2 + 2 = 4");

    tmux::send_line(session, "and 3 + 3?");
    let capture = tmux::wait_for_content(session, "3 + 3 = 6");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "clear_before.txt", None);
}

/// Compare conversation state after /clear against fixture
#[test]
fn test_clear_after_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "contains", "text": "2 + 2" }, "response": "2 + 2 = 4" },
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-fixture-clear-after";
    start_tui(session, &scenario);

    // Build up some conversation
    tmux::send_line(session, "what is 2 + 2?");
    tmux::wait_for_content(session, "2 + 2 = 4");

    // Trigger clear
    tmux::send_line(session, "/clear");

    // Wait for clear to complete (shows "(no content)")
    let capture = tmux::wait_for_content(session, "no content");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "clear_after.txt", None);
}

/// Verify /clear succeeds when session is already empty
#[test]
fn test_clear_empty_session_succeeds() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-clear-empty";
    start_tui(session, &scenario);

    // Clear without any conversation
    tmux::send_line(session, "/clear");

    // Should show "(no content)" without error
    let capture = tmux::wait_for_content(session, "no content");

    tmux::kill_session(session);

    // Verify no error message appears
    assert!(!capture.contains("error"));
    assert!(!capture.contains("Failed"));
}
