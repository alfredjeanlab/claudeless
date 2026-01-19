// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI trust prompt behavioral tests.
//!
//! Tests user interactions with the trust prompt dialog.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{assert_tui_matches_fixture, start_tui_ext, tmux, write_scenario, TUI_READY_PATTERN};

/// Pressing Enter on Yes should proceed to main TUI
#[test]
fn test_trust_prompt_yes_proceeds() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": false
        }
        "#,
    );

    let session = "claudeless-trust-yes";
    start_tui_ext(session, &scenario, 120, 30, "trust");

    // Press Enter to accept trust
    tmux::send_keys(session, "Enter");

    // Wait for main TUI to appear (header pattern indicates ready)
    let capture = tmux::wait_for_content(session, TUI_READY_PATTERN);

    tmux::kill_session(session);

    // After accepting trust, should show main TUI (not trust prompt)
    assert!(
        !capture.to_lowercase().contains("do you trust"),
        "After accepting trust, should not show trust prompt anymore.\nCapture:\n{}",
        capture
    );
}

/// Pressing Esc should cancel/exit
#[test]
fn test_trust_prompt_escape_cancels() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": false
        }
        "#,
    );

    let session = "claudeless-trust-esc";
    start_tui_ext(session, &scenario, 120, 30, "trust");

    // Press Escape to cancel
    tmux::send_keys(session, "Escape");

    // Wait for shell prompt or exit indication
    let capture = tmux::wait_for_any(session, &["$", "❯", "%"]);

    tmux::kill_session(session);

    // After escape, should either exit or show shell prompt (not TUI)
    let has_trust_prompt = capture.to_lowercase().contains("do you trust");

    assert!(
        !has_trust_prompt,
        "After Escape, should exit or return to shell.\nCapture:\n{}",
        capture
    );
}

/// Compare trust prompt against real Claude fixture
#[test]
fn test_trust_prompt_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": false
        }
        "#,
    );

    let session = "claudeless-fixture-trust";
    let capture = start_tui_ext(session, &scenario, 120, 40, "trust");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "trust_prompt.txt", None);
}
