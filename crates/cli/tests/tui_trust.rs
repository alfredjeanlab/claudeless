// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! TUI trust prompt behavioral tests.
//!
//! Tests user interactions with the trust prompt dialog.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{assert_tui_matches_fixture, TuiTestSession, TUI_READY_PATTERN};

const UNTRUSTED_SCENARIO: &str = r#"
    {
        "default_response": "Hello!",
        "trusted": false
    }
"#;

/// Pressing Enter on Yes should proceed to main TUI
#[test]
fn test_trust_prompt_yes_proceeds() {
    let tui = TuiTestSession::with_custom_wait("trust-yes", UNTRUSTED_SCENARIO, 120, 30, "trust");

    // Press Enter to accept trust
    tui.send_keys("Enter");

    // Wait for main TUI to appear (header pattern indicates ready)
    let capture = tui.wait_for(TUI_READY_PATTERN);

    // After accepting trust, should show main TUI (not trust prompt)
    assert!(
        !capture.to_lowercase().contains("do you trust"),
        "After accepting trust, should not show trust prompt anymore.\nCapture:\n{}",
        capture
    );
}

/// Pressing Esc should cancel/exit
// FIXME: Fixture is golden capture from real CLI. Claudeless doesn't dismiss
// trust prompt on Escape press. Real CLI exits when user presses Escape on
// the trust dialog. Need to implement Escape handling for trust dialog.
#[test]
#[ignore]
fn test_trust_prompt_escape_cancels() {
    let tui = TuiTestSession::with_custom_wait("trust-esc", UNTRUSTED_SCENARIO, 120, 30, "trust");

    // Press Escape to cancel
    tui.send_keys("Escape");

    // Wait for shell prompt or exit indication
    let capture = tui.wait_for_any(&["$", "❯", "%"]);

    // After escape, should either exit or show shell prompt (not TUI)
    let has_trust_prompt = capture.to_lowercase().contains("do you trust");

    assert!(
        !has_trust_prompt,
        "After Escape, should exit or return to shell.\nCapture:\n{}",
        capture
    );
}

/// Compare trust prompt against real Claude fixture
// FIXME: Fixture is golden capture from real CLI. normalize_tui() doesn't strip
// shell preamble when capturing. Fixture only contains the trust dialog box,
// not the full terminal output with shell prompt.
#[test]
#[ignore]
fn test_trust_prompt_matches_fixture() {
    let tui =
        TuiTestSession::with_custom_wait("fixture-trust", UNTRUSTED_SCENARIO, 120, 40, "trust");
    let capture = tui.capture();

    assert_tui_matches_fixture(&capture, "trust_prompt.txt", None);
}
