// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! TUI /export command tests - conversation export behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## /export Command Behavior
//! - Typing /export shows autocomplete with "Export the current conversation to a file or clipboard"
//! - Executing /export shows a dialog with two options:
//!   1. Copy to clipboard - copies conversation to system clipboard
//!   2. Save to file - prompts for filename, saves to current directory
//! - Escape cancels the export and shows "Export cancelled"
//! - Escape from filename input returns to method selection

mod common;

use common::{start_tui, tmux, write_scenario};

// =============================================================================
// /export Autocomplete Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing /export shows autocomplete dropdown with export description
#[test]
#[ignore] // DEFERRED: Requires slash command autocomplete implementation
fn test_tui_export_command_shows_autocomplete() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = tmux::unique_session("export-autocomplete");
    let previous = start_tui(&session, &scenario);

    // Type /export
    tmux::send_keys(&session, "/export");
    let capture = tmux::wait_for_change(&session, &previous);

    tmux::kill_session(&session);

    assert!(
        capture.contains("/export")
            && capture.contains("Export the current conversation to a file or clipboard"),
        "/export should show autocomplete with description.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /export Dialog Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /export command shows a dialog with export method options
#[test]
fn test_tui_export_shows_method_dialog() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = tmux::unique_session("export-method-dialog");
    let previous = start_tui(&session, &scenario);

    // First, create a conversation by sending a message
    tmux::send_keys(&session, "test message");
    let _ = tmux::wait_for_change(&session, &previous);
    tmux::send_keys(&session, "Enter");
    let after_message = tmux::wait_for_content(&session, "Hello!");

    // Type /export and press Enter
    tmux::send_keys(&session, "/export");
    let _ = tmux::wait_for_change(&session, &after_message);
    tmux::send_keys(&session, "Enter");
    let capture = tmux::wait_for_content(&session, "Export Conversation");

    tmux::kill_session(&session);

    // Should show the export dialog
    assert!(
        capture.contains("Export Conversation"),
        "Should show Export Conversation dialog header.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Select export method:"),
        "Should show method selection prompt.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Copy to clipboard"),
        "Should show clipboard option.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Save to file"),
        "Should show file option.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Selecting clipboard option copies conversation and shows confirmation
#[test]
fn test_tui_export_clipboard_shows_confirmation() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = tmux::unique_session("export-clipboard");
    let previous = start_tui(&session, &scenario);

    // First, create a conversation by sending a message
    tmux::send_keys(&session, "test message");
    let _ = tmux::wait_for_change(&session, &previous);
    tmux::send_keys(&session, "Enter");
    let after_message = tmux::wait_for_content(&session, "Hello!");

    // Type /export, press Enter to open dialog, then Enter again to select clipboard
    tmux::send_keys(&session, "/export");
    let _ = tmux::wait_for_change(&session, &after_message);
    tmux::send_keys(&session, "Enter");
    let dialog = tmux::wait_for_content(&session, "Export Conversation");
    tmux::send_keys(&session, "Enter");
    let capture = tmux::wait_for_change(&session, &dialog);

    tmux::kill_session(&session);

    // Clipboard may not be available in CI/headless environments
    assert!(
        capture.contains("Conversation copied to clipboard")
            || capture.contains("Failed to access clipboard"),
        "Should show clipboard confirmation or error message.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Selecting file option shows filename input dialog
#[test]
fn test_tui_export_file_shows_filename_dialog() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = tmux::unique_session("export-filename-dialog");
    let previous = start_tui(&session, &scenario);

    // First, create a conversation by sending a message
    tmux::send_keys(&session, "test message");
    let _ = tmux::wait_for_change(&session, &previous);
    tmux::send_keys(&session, "Enter");
    let after_message = tmux::wait_for_content(&session, "Hello!");

    // Type /export, press Enter to open dialog, Down to select file, Enter
    tmux::send_keys(&session, "/export");
    let _ = tmux::wait_for_change(&session, &after_message);
    tmux::send_keys(&session, "Enter");
    let dialog = tmux::wait_for_content(&session, "Export Conversation");
    tmux::send_keys(&session, "Down");
    let _ = tmux::wait_for_change(&session, &dialog);
    tmux::send_keys(&session, "Enter");
    let capture = tmux::wait_for_content(&session, "Enter filename:");

    tmux::kill_session(&session);

    assert!(
        capture.contains("Enter filename:"),
        "Should show filename prompt.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains(".txt"),
        "Should show default filename with .txt extension.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Enter to save"),
        "Should show save instruction.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("esc to go back"),
        "Should show back instruction.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Saving to file shows confirmation with filename
#[test]
fn test_tui_export_file_shows_save_confirmation() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = tmux::unique_session("export-file-save");
    let previous = start_tui(&session, &scenario);

    // First, create a conversation by sending a message
    tmux::send_keys(&session, "test message");
    let _ = tmux::wait_for_change(&session, &previous);
    tmux::send_keys(&session, "Enter");
    let after_message = tmux::wait_for_content(&session, "Hello!");

    // Type /export, open dialog, select file, press Enter to save with default name
    tmux::send_keys(&session, "/export");
    let _ = tmux::wait_for_change(&session, &after_message);
    tmux::send_keys(&session, "Enter");
    let dialog = tmux::wait_for_content(&session, "Export Conversation");
    tmux::send_keys(&session, "Down");
    let _ = tmux::wait_for_change(&session, &dialog);
    tmux::send_keys(&session, "Enter");
    let filename_dialog = tmux::wait_for_content(&session, "Enter filename:");
    tmux::send_keys(&session, "Enter");
    let capture = tmux::wait_for_change(&session, &filename_dialog);

    tmux::kill_session(&session);

    assert!(
        capture.contains("Conversation exported to:"),
        "Should show export confirmation with filename.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /export Cancel Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Pressing Escape in method selection cancels export
#[test]
fn test_tui_export_escape_cancels() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = tmux::unique_session("export-cancel");
    let previous = start_tui(&session, &scenario);

    // First, create a conversation by sending a message
    tmux::send_keys(&session, "test message");
    let _ = tmux::wait_for_change(&session, &previous);
    tmux::send_keys(&session, "Enter");
    let after_message = tmux::wait_for_content(&session, "Hello!");

    // Type /export, open dialog, then cancel with Escape
    tmux::send_keys(&session, "/export");
    let _ = tmux::wait_for_change(&session, &after_message);
    tmux::send_keys(&session, "Enter");
    let dialog = tmux::wait_for_content(&session, "Export Conversation");
    tmux::send_keys(&session, "Escape");
    let capture = tmux::wait_for_change(&session, &dialog);

    tmux::kill_session(&session);

    assert!(
        capture.contains("Export cancelled"),
        "Escape should cancel export and show message.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Pressing Escape in filename dialog returns to method selection
#[test]
fn test_tui_export_filename_escape_returns_to_method() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = tmux::unique_session("export-filename-back");
    let previous = start_tui(&session, &scenario);

    // First, create a conversation by sending a message
    tmux::send_keys(&session, "test message");
    let _ = tmux::wait_for_change(&session, &previous);
    tmux::send_keys(&session, "Enter");
    let after_message = tmux::wait_for_content(&session, "Hello!");

    // Navigate to filename dialog, then press Escape
    tmux::send_keys(&session, "/export");
    let _ = tmux::wait_for_change(&session, &after_message);
    tmux::send_keys(&session, "Enter");
    let dialog = tmux::wait_for_content(&session, "Export Conversation");
    tmux::send_keys(&session, "Down");
    let _ = tmux::wait_for_change(&session, &dialog);
    tmux::send_keys(&session, "Enter");
    let filename_dialog = tmux::wait_for_content(&session, "Enter filename:");
    tmux::send_keys(&session, "Escape");
    let capture = tmux::wait_for_change(&session, &filename_dialog);

    tmux::kill_session(&session);

    // Should return to method selection
    assert!(
        capture.contains("Select export method:"),
        "Escape from filename should return to method selection.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Copy to clipboard"),
        "Should show clipboard option again.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /export Navigation Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Arrow keys navigate between export method options
#[test]
fn test_tui_export_arrow_navigation() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = tmux::unique_session("export-navigation");
    let previous = start_tui(&session, &scenario);

    // First, create a conversation by sending a message
    tmux::send_keys(&session, "test message");
    let _ = tmux::wait_for_change(&session, &previous);
    tmux::send_keys(&session, "Enter");
    let after_message = tmux::wait_for_content(&session, "Hello!");

    // Open export dialog
    tmux::send_keys(&session, "/export");
    let _ = tmux::wait_for_change(&session, &after_message);
    tmux::send_keys(&session, "Enter");
    let initial = tmux::wait_for_content(&session, "Export Conversation");

    // Default should have cursor on first option
    assert!(
        initial.contains("❯ 1. Copy to clipboard"),
        "First option should be selected by default.\nCapture:\n{}",
        initial
    );

    // Press Down to move to second option
    tmux::send_keys(&session, "Down");
    let after_down = tmux::wait_for_change(&session, &initial);

    tmux::kill_session(&session);

    assert!(
        after_down.contains("❯ 2. Save to file"),
        "Down arrow should select second option.\nCapture:\n{}",
        after_down
    );
}
