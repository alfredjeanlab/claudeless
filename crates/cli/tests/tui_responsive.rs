// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! Integration tests for responsive terminal width rendering.
//!
//! These tests verify claudeless adapts to different terminal widths,
//! independent of the fixture comparison tests (which use 120 chars).

mod common;

use common::{start_tui_ext, tmux, write_scenario, TUI_READY_PATTERN};

/// Helper to find separator lines in output
fn find_separator_line(capture: &str) -> Option<&str> {
    capture
        .lines()
        .find(|line| line.chars().all(|c| c == '─') && line.chars().count() > 50)
}

/// Separator should span full terminal width at 80 columns
#[test]
fn test_separator_width_80() {
    let scenario = write_scenario(r#"{ "default_response": "ok", "trusted": true }"#);
    let session = tmux::unique_session("responsive-80");

    let capture = start_tui_ext(&session, &scenario, 80, 24, TUI_READY_PATTERN);
    tmux::kill_session(&session);

    // Find separator line and verify width
    let separator_line = find_separator_line(&capture).expect("Should have separator line");

    assert_eq!(
        separator_line.chars().count(),
        80,
        "Separator should be 80 chars at 80-column terminal"
    );
}

/// Separator should span full terminal width at 100 columns
#[test]
fn test_separator_width_100() {
    let scenario = write_scenario(r#"{ "default_response": "ok", "trusted": true }"#);
    let session = tmux::unique_session("responsive-100");

    let capture = start_tui_ext(&session, &scenario, 100, 24, TUI_READY_PATTERN);
    tmux::kill_session(&session);

    let separator_line = find_separator_line(&capture).expect("Should have separator line");

    assert_eq!(
        separator_line.chars().count(),
        100,
        "Separator should be 100 chars at 100-column terminal"
    );
}

/// Separator should span full terminal width at 150 columns
#[test]
fn test_separator_width_150() {
    let scenario = write_scenario(r#"{ "default_response": "ok", "trusted": true }"#);
    let session = tmux::unique_session("responsive-150");

    let capture = start_tui_ext(&session, &scenario, 150, 24, TUI_READY_PATTERN);
    tmux::kill_session(&session);

    let separator_line = find_separator_line(&capture).expect("Should have separator line");

    assert_eq!(
        separator_line.chars().count(),
        150,
        "Separator should be 150 chars at 150-column terminal"
    );
}

/// Compact separator should span full width after /compact
#[test]
fn test_compact_separator_width() {
    let scenario = write_scenario(
        r#"{
        "default_response": "ok",
        "trusted": true,
        "compact_delay_ms": 100
    }"#,
    );
    let session = tmux::unique_session("compact-width");

    let _ = start_tui_ext(&session, &scenario, 100, 24, TUI_READY_PATTERN);

    // Type a message and wait for response
    tmux::send_line(&session, "hello");
    let _ = tmux::wait_for_content(&session, "ok");

    // Type /compact
    tmux::send_line(&session, "/compact");
    let capture = tmux::wait_for_content(&session, "compacted");

    tmux::kill_session(&session);

    // Find compact separator line (uses ═ character)
    let compact_line = capture
        .lines()
        .find(|line| line.contains("compacted") && line.contains('═'))
        .expect("Should have compact separator");

    assert_eq!(
        compact_line.chars().count(),
        100,
        "Compact separator should be 100 chars at 100-column terminal, got: '{}'",
        compact_line
    );
}
