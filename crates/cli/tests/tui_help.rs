// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TUI /help command tests - help dialog behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## /help Command Behavior
//! - Typing /help shows autocomplete with "Show help and available commands"
//! - Executing /help shows a multi-tab help dialog with tabs:
//!   - general: Overview text and keyboard shortcuts
//!   - commands: Browseable list of default slash commands
//!   - custom-commands: Browseable list of custom/project commands
//! - Tab or Left/Right arrow keys cycle between tabs
//! - Up/Down arrow keys navigate within command lists
//! - Escape dismisses the dialog and shows "Help dialog dismissed"

mod common;

use common::{start_tui, tmux, write_scenario};

// =============================================================================
// /help Autocomplete Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing /help shows autocomplete dropdown with help description
// TODO(implement): requires slash command autocomplete
#[test]
fn test_tui_help_command_shows_autocomplete() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-help-autocomplete";
    let previous = start_tui(session, &scenario);

    // Type /help
    tmux::send_keys(session, "/help");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    assert!(
        capture.contains("/help") && capture.contains("Show help and available commands"),
        "/help should show autocomplete with description.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /help Dialog Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /help command shows a multi-tab help dialog with general tab active by default
// TODO(implement): requires /help dialog
#[test]
fn test_tui_help_shows_dialog_with_general_tab() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-help-general-tab";
    let previous = start_tui(session, &scenario);

    // Type /help and press Enter
    tmux::send_keys(session, "/help");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let capture = tmux::wait_for_content(session, "general");

    tmux::kill_session(session);

    // Should show the help dialog with tabs
    assert!(
        capture.contains("general") && capture.contains("commands"),
        "Should show tab headers including 'general' and 'commands'.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("(←/→ or tab to cycle)"),
        "Should show tab navigation hint.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("/ for commands"),
        "General tab should show keyboard shortcut for commands.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("For more help:"),
        "Should show help link at bottom.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Tab key cycles to commands tab showing browseable command list
// TODO(implement): requires /help dialog tab navigation
#[test]
fn test_tui_help_tab_shows_commands_tab() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-help-commands-tab";
    let previous = start_tui(session, &scenario);

    // Type /help, press Enter, then Tab to go to commands tab
    tmux::send_keys(session, "/help");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let general = tmux::wait_for_content(session, "general");
    tmux::send_keys(session, "Tab");
    let capture = tmux::wait_for_change(session, &general);

    tmux::kill_session(session);

    assert!(
        capture.contains("Browse default commands:"),
        "Commands tab should show 'Browse default commands:' header.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("/add-dir"),
        "Commands tab should show browseable commands list.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Tab cycles through all tabs: general -> commands -> custom-commands -> general
// TODO(implement): requires /help dialog tab cycling
#[test]
fn test_tui_help_tab_cycles_through_all_tabs() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-help-tab-cycle";
    let previous = start_tui(session, &scenario);

    // Type /help and press Enter
    tmux::send_keys(session, "/help");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let general = tmux::wait_for_content(session, "general");

    // Tab to commands
    tmux::send_keys(session, "Tab");
    let commands = tmux::wait_for_change(session, &general);
    assert!(
        commands.contains("Browse default commands:"),
        "First tab should show commands tab.\nCapture:\n{}",
        commands
    );

    // Tab to custom-commands
    tmux::send_keys(session, "Tab");
    let custom = tmux::wait_for_change(session, &commands);
    assert!(
        custom.contains("custom-commands") || custom.contains("Browse custom commands:"),
        "Second tab should show custom-commands tab.\nCapture:\n{}",
        custom
    );

    tmux::kill_session(session);
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Left/Right arrow keys navigate between tabs (alternative to Tab key)
#[test]
fn test_tui_help_arrow_keys_navigate_tabs() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-help-arrow-tabs";
    let previous = start_tui(session, &scenario);

    // Open help dialog
    tmux::send_keys(session, "/help");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let general = tmux::wait_for_content(session, "general");

    // Right arrow should go to commands tab
    tmux::send_keys(session, "Right");
    let commands = tmux::wait_for_change(session, &general);
    assert!(
        commands.contains("Browse default commands:"),
        "Right arrow should navigate to commands tab.\nCapture:\n{}",
        commands
    );

    // Left arrow should go back to general tab
    tmux::send_keys(session, "Left");
    let back_to_general = tmux::wait_for_change(session, &commands);

    tmux::kill_session(session);

    assert!(
        back_to_general.contains("/ for commands"),
        "Left arrow should navigate back to general tab.\nCapture:\n{}",
        back_to_general
    );
}

// =============================================================================
// /help Navigation Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Arrow keys navigate through commands in the commands tab
// TODO(implement): requires /help command list navigation
#[test]
fn test_tui_help_commands_arrow_navigation() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-help-commands-nav";
    let previous = start_tui(session, &scenario);

    // Navigate to commands tab
    tmux::send_keys(session, "/help");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let general = tmux::wait_for_content(session, "general");
    tmux::send_keys(session, "Tab");
    let commands = tmux::wait_for_change(session, &general);

    // Should start with first command selected
    assert!(
        commands.contains("❯ /add-dir"),
        "First command should be selected by default.\nCapture:\n{}",
        commands
    );

    // Press Down to move to next command
    tmux::send_keys(session, "Down");
    let after_down = tmux::wait_for_change(session, &commands);

    tmux::kill_session(session);

    // Should show next command selected (e.g., /agents)
    assert!(
        after_down.contains("❯ /agents"),
        "Down arrow should select next command.\nCapture:\n{}",
        after_down
    );
}

// =============================================================================
// /help Dismiss Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Pressing Escape dismisses the help dialog
// TODO(implement): requires /help dialog dismiss
#[test]
fn test_tui_help_escape_dismisses_dialog() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-help-dismiss";
    let previous = start_tui(session, &scenario);

    // Open help dialog
    tmux::send_keys(session, "/help");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let dialog = tmux::wait_for_content(session, "general");

    // Press Escape to dismiss
    tmux::send_keys(session, "Escape");
    let capture = tmux::wait_for_change(session, &dialog);

    tmux::kill_session(session);

    assert!(
        capture.contains("Help dialog dismissed"),
        "Escape should dismiss help dialog and show message.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// After dismissing help dialog, input is cleared and ready for new input
// TODO(implement): requires /help dialog dismiss
#[test]
fn test_tui_help_dismiss_returns_to_clean_input() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-help-dismiss-clean";
    let previous = start_tui(session, &scenario);

    // Open and dismiss help dialog
    tmux::send_keys(session, "/help");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let dialog = tmux::wait_for_content(session, "general");
    tmux::send_keys(session, "Escape");
    let capture = tmux::wait_for_change(session, &dialog);

    tmux::kill_session(session);

    // Should show empty input area (with placeholder)
    assert!(
        capture.contains("❯") && !capture.contains("general"),
        "After dismissing, should return to clean input without dialog.\nCapture:\n{}",
        capture
    );
}
