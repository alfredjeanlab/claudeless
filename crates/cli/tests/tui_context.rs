// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Context command tests - /context slash command behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## /context Behavior
//! - `/context` visualizes current context usage as a colored grid
//! - Shows a 10x9 grid with symbols representing different context categories
//! - Displays "Estimated usage by category" with token counts and percentages
//! - Categories include: System prompt, System tools, Memory files, Messages, Free space, Autocompact buffer
//! - Shows "Memory files · /memory" section listing loaded CLAUDE.md files
//! - The command appears in autocomplete with description "Visualize current context usage as a colored grid"

mod common;

use common::{start_tui, tmux, write_scenario};

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// When /context is executed, it shows a context usage grid with token estimates.
#[test]
fn test_context_shows_usage_grid() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-context-usage-grid";
    start_tui(session, &scenario);

    // Execute /context command
    tmux::send_line(session, "/context");

    // Wait for the context grid to appear
    let capture = tmux::wait_for_content(session, "Estimated usage by category");

    tmux::kill_session(session);

    // Should show the grid header
    assert!(
        capture.contains("Estimated usage by category"),
        "/context should show usage categories.\nCapture:\n{}",
        capture
    );

    // Should show category breakdowns
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

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /context shows memory files section listing loaded CLAUDE.md files.
#[test]
fn test_context_shows_memory_files() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-context-memory-files";
    start_tui(session, &scenario);

    // Execute /context command
    tmux::send_line(session, "/context");

    // Wait for the memory files section
    let capture = tmux::wait_for_content(session, "Memory files");

    tmux::kill_session(session);

    // Should show memory files section with /memory reference
    assert!(
        capture.contains("Memory files") && capture.contains("/memory"),
        "/context should show Memory files section.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /context appears in slash command autocomplete with correct description.
#[test]
fn test_context_in_autocomplete() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-context-autocomplete";
    start_tui(session, &scenario);

    // Type /context to trigger autocomplete
    tmux::send_keys(session, "/context");

    // Wait for autocomplete to appear
    let capture = tmux::wait_for_content(session, "/context");

    tmux::kill_session(session);

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

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /context displays a visual grid using Unicode symbols.
#[test]
fn test_context_shows_visual_grid() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-context-visual-grid";
    start_tui(session, &scenario);

    // Execute /context command
    tmux::send_line(session, "/context");

    // Wait for the grid symbols to appear
    let capture = tmux::wait_for_content(session, "Estimated usage");

    tmux::kill_session(session);

    // Should contain grid symbols (⛶ for free space, ⛝ for autocompact buffer)
    let has_grid_symbols = capture.contains('⛶') || capture.contains('⛝') || capture.contains('⛁');
    assert!(
        has_grid_symbols,
        "/context should show visual grid with Unicode symbols.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /context shows token counts with percentages for each category.
#[test]
fn test_context_shows_token_percentages() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "claude_version": "2.1.12",
            "responses": [
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-context-percentages";
    start_tui(session, &scenario);

    // Execute /context command
    tmux::send_line(session, "/context");

    // Wait for percentage to appear
    let capture = tmux::wait_for_content(session, "tokens");

    tmux::kill_session(session);

    // Should show token counts with percentages (e.g., "2.8k tokens (1.4%)")
    assert!(
        capture.contains("tokens") && capture.contains('%'),
        "/context should show token counts with percentages.\nCapture:\n{}",
        capture
    );
}
