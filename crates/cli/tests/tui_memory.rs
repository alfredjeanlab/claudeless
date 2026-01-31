// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

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

use common::TuiTestSession;

const SCENARIO: &str = r#"
    name = "test"
    [[responses]]
    pattern = { type = "any" }
    response = "Hello!"
"#;

// =============================================================================
// /memory Autocomplete Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing /memory shows autocomplete dropdown with memory description
#[test]
fn test_tui_memory_command_shows_autocomplete() {
    let tui = TuiTestSession::new("memory-autocomplete", SCENARIO);
    let previous = tui.capture();

    // Type /memory
    tui.send_keys("/memory");
    let capture = tui.wait_for_change(&previous);

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
    let tui = TuiTestSession::new("memory-dialog", SCENARIO);
    let previous = tui.capture();

    // Type /memory and press Enter
    tui.send_keys("/memory");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let capture = tui.wait_for("Memory");

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
    let tui = TuiTestSession::new("memory-count", SCENARIO);
    let previous = tui.capture();

    // Type /memory and press Enter
    tui.send_keys("/memory");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let capture = tui.wait_for("Memory");

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
    let tui = TuiTestSession::new("memory-nav", SCENARIO);
    let previous = tui.capture();

    // Open memory dialog
    tui.send_keys("/memory");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let initial = tui.wait_for("Memory");

    // Press Down to move to next entry
    tui.send_keys("Down");
    let after_down = tui.wait_for_change(&initial);

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
    let tui = TuiTestSession::new("memory-dismiss", SCENARIO);
    let previous = tui.capture();

    // Open memory dialog
    tui.send_keys("/memory");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let dialog = tui.wait_for("Memory");

    // Press Escape to dismiss
    tui.send_keys("Escape");
    let capture = tui.wait_for_change(&dialog);

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
    let tui = TuiTestSession::new("memory-select", SCENARIO);
    let previous = tui.capture();

    // Open memory dialog
    tui.send_keys("/memory");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let dialog = tui.wait_for("Memory");

    // Press Enter to select
    tui.send_keys("Enter");
    let capture = tui.wait_for_change(&dialog);

    // Should show selected entry info
    assert!(
        capture.contains("Selected"),
        "Enter should show selected entry.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /memory Edge Case Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /memory dialog handles case where no CLAUDE.md files exist
#[test]
fn test_tui_memory_shows_no_files_gracefully() {
    let tui = TuiTestSession::new("memory-no-files", SCENARIO);
    let previous = tui.capture();

    // Type /memory and press Enter
    tui.send_keys("/memory");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");
    let capture = tui.wait_for("Memory");

    // Should still show dialog with all source types (even if inactive)
    assert!(
        capture.contains("Memory"),
        "Should show Memory dialog header.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Project") && capture.contains("User") && capture.contains("Enterprise"),
        "Should show all memory source types.\nCapture:\n{}",
        capture
    );
    // Should not crash or show error (check for error patterns, not bare "error"
    // substring which can appear in branch names like "feature/error-jsonl")
    assert!(
        !capture.contains("Error:") && !capture.contains("error:") && !capture.contains("panic"),
        "Should handle no CLAUDE.md files gracefully.\nCapture:\n{}",
        capture
    );
}
