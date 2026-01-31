// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! Clear tests - /clear command behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{assert_tui_matches_fixture, TuiTestSession};

/// Compare conversation state before /clear against fixture
#[test]
fn test_clear_before_matches_fixture() {
    let tui = TuiTestSession::new(
        "fixture-clear-before",
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

    // Build up conversation
    tui.send_line("what is 2 + 2?");
    tui.wait_for("2 + 2 = 4");

    tui.send_line("and 3 + 3?");
    let capture = tui.wait_for("3 + 3 = 6");

    assert_tui_matches_fixture(&capture, "clear_before.txt", None);
}

/// Compare conversation state after /clear against fixture
#[test]
fn test_clear_after_matches_fixture() {
    let tui = TuiTestSession::new(
        "fixture-clear-after",
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

    // Build up some conversation
    tui.send_line("what is 2 + 2?");
    tui.wait_for("2 + 2 = 4");

    // Trigger clear
    tui.send_line("/clear");

    // Wait for clear to complete (shows "(no content)")
    let capture = tui.wait_for("no content");

    assert_tui_matches_fixture(&capture, "clear_after.txt", None);
}

/// Verify /clear succeeds when session is already empty
#[test]
fn test_clear_empty_session_succeeds() {
    let tui = TuiTestSession::new(
        "clear-empty",
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

    // Clear without any conversation
    tui.send_line("/clear");

    // Should show "(no content)" without error
    let capture = tui.wait_for("no content");

    // Verify no error message appears (check for error patterns, not bare "error"
    // substring which can appear in branch names like "feature/error-jsonl")
    assert!(!capture.contains("Error:"));
    assert!(!capture.contains("error:"));
    assert!(!capture.contains("Failed"));
}
