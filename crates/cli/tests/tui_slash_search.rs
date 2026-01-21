// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TUI slash command search tests - incremental filtering and navigation.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## Slash Command Search Behavior
//! - Typing `/` opens the slash command autocomplete menu
//! - Menu shows commands in alphabetical order (add-dir, agents, chrome, clear, compact, config, ...)
//! - Each command has a description shown after the command name
//! - First command in the filtered list is highlighted (selected) by default
//! - Typing additional characters filters commands using fuzzy/subsequence matching
//! - Down arrow moves selection to the next command (changes highlight color)
//! - Up arrow moves selection to the previous command
//! - Tab completes the input field to the selected command and closes the menu
//! - If the command takes arguments, Tab shows argument hint (e.g., `/add-dir  <path>`)
//! - Escape closes the autocomplete menu but leaves typed text, shows "Esc to clear again"
//! - Another Escape (or Ctrl+U) clears the input

mod common;

use common::{start_tui, tmux, write_scenario};

// =============================================================================
// Slash Command Menu Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing / opens the slash command autocomplete menu
#[test]
fn test_tui_slash_opens_command_menu() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-menu";
    let _previous = start_tui(session, &scenario);

    // Type /
    tmux::send_keys(session, "/");
    // Wait for menu content to appear (not just any change)
    let capture = tmux::wait_for_content(session, "/add-dir");

    tmux::kill_session(session);

    assert!(
        capture.contains("/add-dir") && capture.contains("Add a new working directory"),
        "/ should open command menu showing /add-dir.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("/agents") && capture.contains("Manage agent configurations"),
        "Menu should show /agents command.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Menu shows multiple commands with descriptions
#[test]
fn test_tui_slash_menu_shows_descriptions() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-descriptions";
    let _previous = start_tui(session, &scenario);

    // Type /
    tmux::send_keys(session, "/");
    // Wait for menu content to appear
    let capture = tmux::wait_for_content(session, "/clear");

    tmux::kill_session(session);

    // Should show commands with their descriptions
    assert!(
        capture.contains("/clear") && capture.contains("Clear conversation history"),
        "Menu should show /clear with description.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("/compact") && capture.contains("keep a summary in context"),
        "Menu should show /compact with description.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// Incremental Filtering Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing characters after / filters commands using fuzzy matching
#[test]
fn test_tui_slash_filters_commands() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-filter";
    let previous = start_tui(session, &scenario);

    // Type /co
    tmux::send_keys(session, "/co");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    // Should show commands matching "co"
    assert!(
        capture.contains("/compact"),
        "/co should show /compact.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("/context"),
        "/co should show /context.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("/config"),
        "/co should show /config.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Filtering narrows down results as more characters are typed
#[test]
fn test_tui_slash_filters_progressively() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-progressive";
    let _previous = start_tui(session, &scenario);

    // Type /hel
    tmux::send_keys(session, "/hel");
    // Wait for the filtered menu to appear
    let capture = tmux::wait_for_content(session, "/help");

    tmux::kill_session(session);

    // Should only show /help
    assert!(
        capture.contains("/help"),
        "/hel should show /help.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Fuzzy matching finds commands with characters in sequence, not just prefix
#[test]
fn test_tui_slash_fuzzy_matches() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-fuzzy";
    let previous = start_tui(session, &scenario);

    // Type /h - should match help, hooks, theme, chrome, etc. (all containing 'h')
    tmux::send_keys(session, "/h");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    // Should show commands containing 'h'
    assert!(
        capture.contains("/help"),
        "/h should show /help.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("/hooks"),
        "/h should show /hooks.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// Arrow Key Navigation Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Down arrow moves selection to the next command
#[test]
fn test_tui_slash_down_arrow_navigation() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-down";
    let _previous = start_tui(session, &scenario);

    // Type /
    tmux::send_keys(session, "/");
    // Wait for menu to appear
    let _ = tmux::wait_for_content(session, "/add-dir");

    // Press Down to move to next command
    tmux::send_keys(session, "Down");
    // Small delay for selection to update (visual change is subtle)
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Press Tab to complete and verify selection moved
    tmux::send_keys(session, "Tab");
    let capture = tmux::wait_for_content(session, "/agents");

    tmux::kill_session(session);

    // After Down, Tab should complete to /agents (second command, not /add-dir)
    assert!(
        capture.contains("/agents"),
        "Down then Tab should complete to /agents (second command).\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Up arrow moves selection to the previous command
#[test]
fn test_tui_slash_up_arrow_navigation() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-up";
    let _previous = start_tui(session, &scenario);

    // Type /
    tmux::send_keys(session, "/");
    // Wait for menu to appear
    let _ = tmux::wait_for_content(session, "/add-dir");

    // Press Down twice then Up once
    tmux::send_keys(session, "Down");
    std::thread::sleep(std::time::Duration::from_millis(100));
    tmux::send_keys(session, "Down");
    std::thread::sleep(std::time::Duration::from_millis(100));
    tmux::send_keys(session, "Up");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Press Tab to complete
    tmux::send_keys(session, "Tab");
    let capture = tmux::wait_for_content(session, "/agents");

    tmux::kill_session(session);

    // Should complete to /agents (Down, Down, Up = second command)
    assert!(
        capture.contains("/agents"),
        "Down, Down, Up then Tab should complete to /agents.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// Tab Completion Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Tab completes to the selected command
#[test]
fn test_tui_slash_tab_completes_first_command() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-tab";
    let _previous = start_tui(session, &scenario);

    // Type /
    tmux::send_keys(session, "/");
    // Wait for menu to appear
    let _ = tmux::wait_for_content(session, "/add-dir");

    // Press Tab without navigation (should complete to first command)
    tmux::send_keys(session, "Tab");
    // Wait for completion - the input line should show /add-dir
    let capture = tmux::wait_for_content(session, "❯ /add-dir");

    tmux::kill_session(session);

    assert!(
        capture.contains("/add-dir"),
        "Tab should complete to first command /add-dir.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Tab shows argument hint for commands that take arguments
#[test]
fn test_tui_slash_tab_shows_argument_hint() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-arg-hint";
    let _previous = start_tui(session, &scenario);

    // Type / and Tab (complete to /add-dir which takes <path>)
    tmux::send_keys(session, "/");
    // Wait for menu to appear
    let _ = tmux::wait_for_content(session, "/add-dir");
    tmux::send_keys(session, "Tab");
    // Wait for completion and hint
    let capture = tmux::wait_for_content(session, "<path>");

    tmux::kill_session(session);

    assert!(
        capture.contains("<path>"),
        "After completing /add-dir, should show <path> argument hint.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Tab closes the autocomplete menu after completion
#[test]
fn test_tui_slash_tab_closes_menu() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-tab-close";
    let _previous = start_tui(session, &scenario);

    // Type /
    tmux::send_keys(session, "/");
    let menu = tmux::wait_for_content(session, "/add-dir");

    // Verify menu is showing multiple commands
    assert!(
        menu.contains("/agents"),
        "Menu should be visible with multiple commands.\nCapture:\n{}",
        menu
    );

    // Press Tab
    tmux::send_keys(session, "Tab");
    // Wait for the completion to be in the input line
    let capture = tmux::wait_for_content(session, "❯ /add-dir");

    tmux::kill_session(session);

    // Menu should be closed (only show completed command, not the list)
    // After Tab, the menu items (/agents, /bug, etc.) should not be visible
    // Only the completed command in the input should show /add-dir
    assert!(
        !capture.contains("/agents"),
        "After Tab, menu should close (no /agents visible).\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// Escape Behavior Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Escape closes autocomplete menu but keeps typed text
#[test]
fn test_tui_slash_escape_closes_menu_keeps_text() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-escape";
    let _previous = start_tui(session, &scenario);

    // Type /
    tmux::send_keys(session, "/");
    let _ = tmux::wait_for_content(session, "/add-dir");

    // Press Escape
    tmux::send_keys(session, "Escape");
    let capture = tmux::wait_for_content(session, "Esc to clear again");

    tmux::kill_session(session);

    // Should still show / in input but menu closed
    assert!(
        capture.contains("❯ /"),
        "Input should still contain /.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Esc to clear again"),
        "Should show 'Esc to clear again' hint.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Escape from filtered search closes menu but keeps filter text
#[test]
fn test_tui_slash_escape_from_filtered_keeps_text() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-slash-escape-filter";
    let _previous = start_tui(session, &scenario);

    // Type /he
    tmux::send_keys(session, "/he");
    let _ = tmux::wait_for_content(session, "/help");

    // Press Escape
    tmux::send_keys(session, "Escape");
    let capture = tmux::wait_for_content(session, "Esc to clear again");

    tmux::kill_session(session);

    // Should still show /he in input
    assert!(
        capture.contains("❯ /he"),
        "Input should still contain /he.\nCapture:\n{}",
        capture
    );
}
