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

use common::TuiTestSession;

const SCENARIO: &str = r#"
    name = "test"
    [[responses]]
    pattern = { type = "any" }
    response = "Hello!"
"#;

// =============================================================================
// /export Autocomplete Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing /export shows autocomplete dropdown with export description
#[test]
#[ignore] // DEFERRED: Requires slash command autocomplete implementation
fn test_tui_export_command_shows_autocomplete() {
    let tui = TuiTestSession::new("export-autocomplete", SCENARIO);
    let previous = tui.capture();

    // Type /export
    tui.send_keys("/export");
    let capture = tui.wait_for_change(&previous);

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
    let tui = TuiTestSession::new("export-method-dialog", SCENARIO);
    let previous = tui.capture();

    // First, create a conversation by sending a message
    tui.send_keys("test message");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let after_message = tui.wait_for("Hello!");

    // Type /export and press Enter
    tui.send_keys("/export");
    let _ = tui.wait_for_change(&after_message);
    tui.send_keys("Enter");
    let capture = tui.wait_for("Export Conversation");

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
#[ignore] // TODO(flaky): Timing-sensitive tmux test that fails intermittently on CI
fn test_tui_export_clipboard_shows_confirmation() {
    let tui = TuiTestSession::new("export-clipboard", SCENARIO);
    let previous = tui.capture();

    // First, create a conversation by sending a message
    tui.send_keys("test message");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let after_message = tui.wait_for("Hello!");

    // Type /export, press Enter to open dialog, then Enter again to select clipboard
    tui.send_keys("/export");
    let _ = tui.wait_for_change(&after_message);
    tui.send_keys("Enter");
    let dialog = tui.wait_for("Export Conversation");
    tui.send_keys("Enter");
    let capture = tui.wait_for_change(&dialog);

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
#[ignore] // TODO(flaky): Timing-sensitive tmux test that fails intermittently on CI
fn test_tui_export_file_shows_filename_dialog() {
    let tui = TuiTestSession::new("export-filename-dialog", SCENARIO);
    let previous = tui.capture();

    // First, create a conversation by sending a message
    tui.send_keys("test message");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let after_message = tui.wait_for("Hello!");

    // Type /export, press Enter to open dialog, Down to select file, Enter
    tui.send_keys("/export");
    let _ = tui.wait_for_change(&after_message);
    tui.send_keys("Enter");
    let dialog = tui.wait_for("Export Conversation");
    tui.send_keys("Down");
    let _ = tui.wait_for_change(&dialog);
    tui.send_keys("Enter");
    let capture = tui.wait_for("Enter filename:");

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
    let tui = TuiTestSession::new("export-file-save", SCENARIO);
    let previous = tui.capture();

    // First, create a conversation by sending a message
    tui.send_keys("test message");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let after_message = tui.wait_for("Hello!");

    // Type /export, open dialog, select file, press Enter to save with default name
    tui.send_keys("/export");
    let _ = tui.wait_for_change(&after_message);
    tui.send_keys("Enter");
    let dialog = tui.wait_for("Export Conversation");
    tui.send_keys("Down");
    let _ = tui.wait_for_change(&dialog);
    tui.send_keys("Enter");
    let filename_dialog = tui.wait_for("Enter filename:");
    tui.send_keys("Enter");
    let capture = tui.wait_for_change(&filename_dialog);

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
    let tui = TuiTestSession::new("export-cancel", SCENARIO);
    let previous = tui.capture();

    // First, create a conversation by sending a message
    tui.send_keys("test message");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let after_message = tui.wait_for("Hello!");

    // Type /export, open dialog, then cancel with Escape
    tui.send_keys("/export");
    let _ = tui.wait_for_change(&after_message);
    tui.send_keys("Enter");
    let dialog = tui.wait_for("Export Conversation");
    tui.send_keys("Escape");
    let capture = tui.wait_for_change(&dialog);

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
#[ignore] // TODO(flaky): Timing-sensitive tmux test that fails intermittently on CI
fn test_tui_export_filename_escape_returns_to_method() {
    let tui = TuiTestSession::new("export-filename-back", SCENARIO);
    let previous = tui.capture();

    // First, create a conversation by sending a message
    tui.send_keys("test message");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let after_message = tui.wait_for("Hello!");

    // Navigate to filename dialog, then press Escape
    tui.send_keys("/export");
    let _ = tui.wait_for_change(&after_message);
    tui.send_keys("Enter");
    let dialog = tui.wait_for("Export Conversation");
    tui.send_keys("Down");
    let _ = tui.wait_for_change(&dialog);
    tui.send_keys("Enter");
    let filename_dialog = tui.wait_for("Enter filename:");
    tui.send_keys("Escape");
    let capture = tui.wait_for_change(&filename_dialog);

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
#[ignore] // TODO(flaky): Timing-sensitive tmux test that fails intermittently on CI
fn test_tui_export_arrow_navigation() {
    let tui = TuiTestSession::new("export-navigation", SCENARIO);
    let previous = tui.capture();

    // First, create a conversation by sending a message
    tui.send_keys("test message");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let after_message = tui.wait_for("Hello!");

    // Open export dialog
    tui.send_keys("/export");
    let _ = tui.wait_for_change(&after_message);
    tui.send_keys("Enter");
    let initial = tui.wait_for("Export Conversation");

    // Default should have cursor on first option
    assert!(
        initial.contains("❯ 1. Copy to clipboard"),
        "First option should be selected by default.\nCapture:\n{}",
        initial
    );

    // Press Down to move to second option
    tui.send_keys("Down");
    let after_down = tui.wait_for_change(&initial);

    assert!(
        after_down.contains("❯ 2. Save to file"),
        "Down arrow should select second option.\nCapture:\n{}",
        after_down
    );
}
