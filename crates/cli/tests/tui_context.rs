// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! Context command tests - /context slash command behavior.
//!
//! Behavior observed with: claude --version 2.1.29 (Claude Code)
//!
//! ## /context Behavior
//! - `/context` visualizes current context usage as a colored grid
//! - Shows a 10x10 grid with symbols representing different context categories
//! - Displays "Estimated usage by category" with token counts and percentages
//! - Categories: System prompt, System tools, Messages, Free space, Autocompact buffer
//! - Row 0 shows model info: model name + used/total tokens
//! - The command appears in autocomplete with description "Visualize current context usage as a colored grid"

mod common;

use common::TuiTestSession;

const SCENARIO: &str = r#"
    {
        "trusted": true,
        "claude_version": "2.1.29",
        "responses": [
            { "pattern": { "type": "any" }, "response": "ok" }
        ]
    }
"#;

/// Behavior observed with: claude --version 2.1.29 (Claude Code)
///
/// When /context is executed, it shows a context usage grid with token estimates.
#[test]
fn test_context_shows_usage_grid() {
    let tui = TuiTestSession::new("context-usage-grid", SCENARIO);

    // Execute /context command
    tui.send_line("/context");

    // Wait for the context grid to appear
    let capture = tui.wait_for("Estimated usage by category");

    // Should show the grid header
    assert!(
        capture.contains("Estimated usage by category"),
        "/context should show usage categories.\nCapture:\n{}",
        capture
    );

    // Should show category breakdowns (no Memory files category in v2.1.29)
    assert!(
        capture.contains("System prompt"),
        "/context should show System prompt category.\nCapture:\n{}",
        capture
    );

    assert!(
        capture.contains("System tools"),
        "/context should show System tools category.\nCapture:\n{}",
        capture
    );

    assert!(
        capture.contains("Free space"),
        "/context should show Free space category.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.29 (Claude Code)
///
/// /context shows model info on the first grid row.
#[test]
fn test_context_shows_model_info() {
    let tui = TuiTestSession::new("context-model-info", SCENARIO);

    // Execute /context command
    tui.send_line("/context");

    // Wait for model info to appear
    let capture = tui.wait_for("tokens");

    // Should show model name and token summary on row 0
    assert!(
        capture.contains("19k/200k tokens"),
        "/context should show used/total token summary.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.29 (Claude Code)
///
/// /context appears in slash command autocomplete with correct description.
#[test]
fn test_context_in_autocomplete() {
    let tui = TuiTestSession::new("context-autocomplete", SCENARIO);

    // Type /context to trigger autocomplete
    tui.send_keys("/context");

    // Wait for autocomplete to appear
    let capture = tui.wait_for("/context");

    // Should show /context in autocomplete
    assert!(
        capture.contains("/context"),
        "Autocomplete should show /context command.\nCapture:\n{}",
        capture
    );

    // Should show description
    assert!(
        capture.contains("Visualize current context usage")
            || capture.contains("context usage as a colored grid"),
        "Autocomplete should show context description.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.29 (Claude Code)
///
/// /context displays a visual grid using Unicode symbols including ⛁ for used cells.
#[test]
fn test_context_shows_visual_grid() {
    let tui = TuiTestSession::new("context-visual-grid", SCENARIO);

    // Execute /context command
    tui.send_line("/context");

    // Wait for the grid symbols to appear
    let capture = tui.wait_for("Estimated usage");

    // Should contain ⛁ (used cells) in the grid
    assert!(
        capture.contains('\u{26C1}'),
        "/context should show \u{26C1} symbols for used cells.\nCapture:\n{}",
        capture
    );

    // Should contain ⛶ (free space) and ⛝ (autocompact buffer)
    let has_grid_symbols = capture.contains('\u{26F6}') || capture.contains('\u{26DD}');
    assert!(
        has_grid_symbols,
        "/context should show visual grid with Unicode symbols.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.29 (Claude Code)
///
/// /context shows token counts with percentages for each category.
#[test]
fn test_context_shows_token_percentages() {
    let tui = TuiTestSession::new("context-percentages", SCENARIO);

    // Execute /context command
    tui.send_line("/context");

    // Wait for percentage to appear
    let capture = tui.wait_for("tokens");

    // Should show token counts with percentages (e.g., "2.3k tokens (1.1%)")
    assert!(
        capture.contains("tokens") && capture.contains('%'),
        "/context should show token counts with percentages.\nCapture:\n{}",
        capture
    );
}
