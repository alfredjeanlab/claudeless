// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! Fork tests - /fork command behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## Fork Behavior
//! - `/fork` creates a fork of the current conversation at the current point
//! - When executed with no conversation, shows error "Failed to fork conversation: No conversation to fork"
//! - The command appears in autocomplete with description "Create a fork of the current conversation at this point"

mod common;

use common::TuiTestSession;

const SCENARIO: &str = r#"
    {
        "trusted": true,
        "claude_version": "2.1.12",
        "responses": [
            { "pattern": { "type": "any" }, "response": "ok" }
        ]
    }
"#;

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// When /fork is executed with no conversation, it shows an error message.
#[test]
fn test_fork_no_conversation_shows_error() {
    let tui = TuiTestSession::new("fork-no-conversation", SCENARIO);

    // Execute /fork with no conversation
    tui.send_line("/fork");

    // Wait for error message
    let capture = tui.wait_for("Failed to fork");

    assert!(
        capture.contains("Failed to fork conversation: No conversation to fork"),
        "/fork with no conversation should show error.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /fork appears in slash command autocomplete with correct description.
#[test]
fn test_fork_in_autocomplete() {
    let tui = TuiTestSession::new("fork-autocomplete", SCENARIO);

    // Type /fork to trigger autocomplete
    tui.send_keys("/fork");

    // Wait for autocomplete to appear
    let capture = tui.wait_for("/fork");

    // Should show /fork in autocomplete
    assert!(
        capture.contains("/fork"),
        "Autocomplete should show /fork command.\nCapture:\n{}",
        capture
    );

    // Should show description
    assert!(
        capture.contains("Create a fork") || capture.contains("fork of the current conversation"),
        "Autocomplete should show fork description.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /fork succeeds when there is an existing conversation.
#[test]
#[ignore] // TODO(flaky): Timing-sensitive tmux test that fails intermittently on CI
fn test_fork_success_with_conversation() {
    let tui = TuiTestSession::new(
        "fork-success",
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "contains", "text": "hello" }, "response": "Hello! How can I help you today?" },
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    // Build up a conversation first
    tui.send_line("hello");
    tui.wait_for("How can I help you");

    // Execute /fork with existing conversation
    tui.send_line("/fork");

    // Wait for success message
    let capture = tui.wait_for("Conversation forked");

    // Should show success message (not error)
    assert!(
        capture.contains("Conversation forked"),
        "/fork with conversation should succeed.\nCapture:\n{}",
        capture
    );
    assert!(
        !capture.contains("Failed"),
        "/fork with conversation should not show error.\nCapture:\n{}",
        capture
    );
}
