// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! Model display tests - showing model name in TUI.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## Model Display Behavior
//! - Shows model name in header (e.g., "Haiku 4.5 · Claude Max")
//! - Different models: Haiku, Sonnet, Opus
//!
//! ## Model Picker Behavior (Meta+P)
//! - Meta+P (Option+P on macOS) opens model picker dialog
//! - Shows "Select model" header
//! - Lists available models:
//!   - Default (recommended) - Opus 4.5
//!   - Sonnet - Sonnet 4.5
//!   - Haiku - Haiku 4.5
//! - Arrow (❯) indicates cursor position
//! - Checkmark (✔) indicates currently active model
//! - Up/Down arrows navigate between options
//! - Enter confirms selection (changes model for session)
//! - Escape closes picker without changes

mod common;

use common::{capture_tui_initial, start_tui, tmux, write_scenario};

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

// =============================================================================
// Meta+P Model Picker Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Meta+P (Option+P on macOS) opens a model picker dialog showing available models.
#[test]
fn test_tui_meta_p_opens_model_picker() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let session = tmux::unique_session("meta-p-picker");
    let previous = start_tui(&session, &scenario);

    // Press Meta+P to open model picker
    tmux::send_keys(&session, "M-p");
    let capture = tmux::wait_for_change(&session, &previous);

    tmux::kill_session(&session);

    assert!(
        capture.contains("Select model"),
        "Meta+P should open model picker dialog with 'Select model' header.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Model picker shows available models: Default (Opus), Sonnet, and Haiku.
#[test]
fn test_tui_model_picker_shows_available_models() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let session = tmux::unique_session("picker-models");
    let previous = start_tui(&session, &scenario);

    tmux::send_keys(&session, "M-p");
    let capture = tmux::wait_for_change(&session, &previous);

    tmux::kill_session(&session);

    assert!(
        capture.contains("Default") && capture.contains("Sonnet") && capture.contains("Haiku"),
        "Model picker should show Default, Sonnet, and Haiku options.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Model picker shows checkmark (✔) next to currently active model.
#[test]
fn test_tui_model_picker_shows_active_model_checkmark() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let session = tmux::unique_session("picker-checkmark");
    let previous = start_tui(&session, &scenario);

    tmux::send_keys(&session, "M-p");
    let capture = tmux::wait_for_change(&session, &previous);

    tmux::kill_session(&session);

    assert!(
        capture.contains("✔"),
        "Model picker should show checkmark next to active model.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Model picker can be navigated with Up/Down arrow keys.
#[test]
fn test_tui_model_picker_arrow_navigation() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let session = tmux::unique_session("picker-nav");
    let previous = start_tui(&session, &scenario);

    // Open model picker
    tmux::send_keys(&session, "M-p");
    let after_open = tmux::wait_for_change(&session, &previous);

    // Navigate up
    tmux::send_keys(&session, "Up");
    let after_up = tmux::wait_for_change(&session, &after_open);

    tmux::kill_session(&session);

    // The cursor position (❯) should have moved
    assert!(
        after_open != after_up,
        "Arrow keys should navigate model picker (screen should change).\nBefore:\n{}\n\nAfter:\n{}",
        after_open,
        after_up
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Pressing Escape closes the model picker without changing the model.
#[test]
fn test_tui_model_picker_escape_closes() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let session = tmux::unique_session("picker-escape");
    let previous = start_tui(&session, &scenario);

    // Open model picker
    tmux::send_keys(&session, "M-p");
    let _ = tmux::wait_for_change(&session, &previous);

    // Press Escape to close
    tmux::send_keys(&session, "Escape");
    let after_escape = tmux::wait_for_content(&session, "? for shortcuts");

    tmux::kill_session(&session);

    assert!(
        !after_escape.contains("Select model"),
        "Escape should close the model picker.\nCapture:\n{}",
        after_escape
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Model picker footer shows "Enter to confirm · esc to exit".
#[test]
fn test_tui_model_picker_shows_footer_hints() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    let session = tmux::unique_session("picker-footer");
    let previous = start_tui(&session, &scenario);

    tmux::send_keys(&session, "M-p");
    let capture = tmux::wait_for_change(&session, &previous);

    tmux::kill_session(&session);

    assert!(
        capture.contains("Enter to confirm") && capture.contains("esc to exit"),
        "Model picker should show 'Enter to confirm · esc to exit' footer.\nCapture:\n{}",
        capture
    );
}
