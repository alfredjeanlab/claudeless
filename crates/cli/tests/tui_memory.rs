// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TUI /memory command tests - memory management dialog behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## /memory Command Behavior
//! - Typing /memory shows autocomplete with "View or manage conversation memory"
//! - Executing /memory shows a dialog with memory sources:
//!   - Project - .claude/CLAUDE.md (if exists)
//!   - User - ~/.claude/CLAUDE.md (if exists)
//!   - Enterprise - Organization-level instructions (if configured)
//! - Up/Down arrow keys navigate through memory sources
//! - Enter selects a memory source to view its contents
//! - Escape dismisses the dialog and shows "Memory dialog dismissed"
//! - Shows count of active memory files (e.g., "1 file" or "2 files")

mod common;

use common::{start_tui, tmux, write_scenario};

// =============================================================================
// /memory Autocomplete Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing /memory shows autocomplete dropdown with memory description
#[test]
fn test_tui_memory_command_shows_autocomplete() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-memory-autocomplete";
    let previous = start_tui(session, &scenario);

    // Type /memory
    tmux::send_keys(session, "/memory");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    assert!(
        capture.contains("/memory") && capture.contains("View or manage conversation memory"),
        "/memory should show autocomplete with description.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /memory Dialog Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /memory command shows a dialog with memory sources
#[test]
fn test_tui_memory_shows_dialog_with_sources() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-memory-dialog";
    let previous = start_tui(session, &scenario);

    // Type /memory and press Enter
    tmux::send_keys(session, "/memory");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let capture = tmux::wait_for_content(session, "Memory");

    tmux::kill_session(session);

    // Should show the memory dialog header
    assert!(
        capture.contains("Memory"),
        "Should show 'Memory' header.\nCapture:\n{}",
        capture
    );
    // Should show Project memory source
    assert!(
        capture.contains("Project"),
        "Should show Project memory source.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /memory dialog shows count of active memory files
#[test]
fn test_tui_memory_shows_active_files_count() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-memory-count";
    let previous = start_tui(session, &scenario);

    // Type /memory and press Enter
    tmux::send_keys(session, "/memory");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let capture = tmux::wait_for_content(session, "Memory");

    tmux::kill_session(session);

    // Should show count of active files
    assert!(
        capture.contains("file"),
        "Should show active files count.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /memory Navigation Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Down arrow navigates through memory entries
#[test]
fn test_tui_memory_arrow_navigation() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-memory-nav";
    let previous = start_tui(session, &scenario);

    // Open memory dialog
    tmux::send_keys(session, "/memory");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let initial = tmux::wait_for_content(session, "Memory");

    // Press Down to move to next entry
    tmux::send_keys(session, "Down");
    let after_down = tmux::wait_for_change(session, &initial);

    tmux::kill_session(session);

    // Should still show memory dialog
    assert!(
        after_down.contains("Memory"),
        "Should still show memory dialog.\nCapture:\n{}",
        after_down
    );
}

// =============================================================================
// /memory Dismiss Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Pressing Escape dismisses the memory dialog
#[test]
fn test_tui_memory_escape_dismisses_dialog() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-memory-dismiss";
    let previous = start_tui(session, &scenario);

    // Open memory dialog
    tmux::send_keys(session, "/memory");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let dialog = tmux::wait_for_content(session, "Memory");

    // Press Escape to dismiss
    tmux::send_keys(session, "Escape");
    let capture = tmux::wait_for_change(session, &dialog);

    tmux::kill_session(session);

    assert!(
        capture.contains("Memory dialog dismissed"),
        "Escape should dismiss memory dialog and show message.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /memory Selection Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Pressing Enter on a memory entry shows its details
#[test]
fn test_tui_memory_enter_shows_selected() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-memory-select";
    let previous = start_tui(session, &scenario);

    // Open memory dialog
    tmux::send_keys(session, "/memory");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let dialog = tmux::wait_for_content(session, "Memory");

    // Press Enter to select
    tmux::send_keys(session, "Enter");
    let capture = tmux::wait_for_change(session, &dialog);

    tmux::kill_session(session);

    // Should show selected entry info
    assert!(
        capture.contains("Selected"),
        "Enter should show selected entry.\nCapture:\n{}",
        capture
    );
}
