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

use common::{assert_tui_matches_fixture, start_tui, tmux, write_scenario};

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// When /fork is executed with no conversation, it shows an error message.
#[test]
fn test_fork_no_conversation_shows_error() {
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

    let session = tmux::unique_session("fork-no-conversation");
    start_tui(&session, &scenario);

    // Execute /fork with no conversation
    tmux::send_line(&session, "/fork");

    // Wait for error message
    let capture = tmux::wait_for_content(&session, "Failed to fork");

    tmux::kill_session(&session);

    assert!(
        capture.contains("Failed to fork conversation: No conversation to fork"),
        "/fork with no conversation should show error.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Compare /fork error state against fixture when no conversation exists.
#[test]
fn test_fork_no_conversation_matches_fixture() {
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

    let session = tmux::unique_session("fixture-fork-no-conv");
    start_tui(&session, &scenario);

    // Execute /fork with no conversation
    tmux::send_line(&session, "/fork");

    // Wait for error message
    let capture = tmux::wait_for_content(&session, "Failed to fork");

    tmux::kill_session(&session);

    assert_tui_matches_fixture(&capture, "fork_no_conversation.txt", None);
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /fork appears in slash command autocomplete with correct description.
#[test]
fn test_fork_in_autocomplete() {
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

    let session = tmux::unique_session("fork-autocomplete");
    start_tui(&session, &scenario);

    // Type /fork to trigger autocomplete
    tmux::send_keys(&session, "/fork");

    // Wait for autocomplete to appear
    let capture = tmux::wait_for_content(&session, "/fork");

    tmux::kill_session(&session);

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
    let scenario = write_scenario(
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

    let session = tmux::unique_session("fork-success");
    start_tui(&session, &scenario);

    // Build up a conversation first
    tmux::send_line(&session, "hello");
    tmux::wait_for_content(&session, "How can I help you");

    // Execute /fork with existing conversation
    tmux::send_line(&session, "/fork");

    // Wait for success message
    let capture = tmux::wait_for_content(&session, "Conversation forked");

    tmux::kill_session(&session);

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
