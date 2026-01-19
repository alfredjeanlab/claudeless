// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Model display tests - showing model name in TUI.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::capture_tui_initial;

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude shows "Haiku 4.5" when --model haiku is used
#[test]
fn test_tui_shows_haiku_model_name() {
    let capture = capture_tui_initial("claudeless-haiku", "--model haiku");

    assert!(
        capture.to_lowercase().contains("haiku"),
        "TUI should show 'Haiku' when --model haiku is used.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude shows "Sonnet 4.5" when --model sonnet is used
#[test]
fn test_tui_shows_sonnet_model_name() {
    let capture = capture_tui_initial("claudeless-sonnet", "--model sonnet");

    assert!(
        capture.to_lowercase().contains("sonnet"),
        "TUI should show 'Sonnet' when --model sonnet is used.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude shows "Opus 4.5" when --model opus is used
#[test]
fn test_tui_shows_opus_model_name() {
    let capture = capture_tui_initial("claudeless-opus", "--model opus");

    assert!(
        capture.to_lowercase().contains("opus"),
        "TUI should show 'Opus' when --model opus is used.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Real Claude shows model in format "{Model} {Version} · {Account Type}"
/// e.g., "Haiku 4.5 · Claude Max"
#[test]
fn test_tui_model_display_format() {
    let capture = capture_tui_initial("claudeless-format", "--model haiku");

    assert!(
        capture.contains(" · ") || capture.contains(" | ") || capture.contains(" - "),
        "TUI should show model with separator (e.g., 'Model · Account').\nCapture:\n{}",
        capture
    );
}
