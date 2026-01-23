// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TUI shell mode tests - '!' prefix shell mode handling.
//!
//! Behavior observed with: claude --version 2.1.17 (Claude Code)
//!
//! ## Shell Mode Behavior
//! - Typing '!' at the start of empty input enters shell mode
//! - Bash mode shows pink-colored separators (RGB 253, 93, 177)
//! - The prompt shows `! Try "..."` suggestion with `!` in pink (no `❯` prefix)
//! - Commands are shown as `! command` in the input (e.g., `! ls -la`)
//! - Status bar shows `! for bash mode` in pink
//! - When submitted, the prompt shows `❯ ! command` in history

mod common;

use common::{assert_tui_matches_fixture, start_tui, start_tui_ext, tmux, write_scenario};

/// Pattern for bypass mode status bar
const BYPASS_MODE_PATTERN: &str = "bypass permissions on";

// =============================================================================
// Shell Mode Entry Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.17 (Claude Code)
///
/// Typing '!' on empty input shows '!' prefix for shell mode with suggestion hint
#[test]
fn test_tui_exclamation_shows_shell_prefix() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-shell-prefix";
    let previous = start_tui(session, &scenario);

    // Press '!' to enter shell mode
    tmux::send_keys(session, "!");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    // Should show '! Try "..."' or '! for bash mode' in the UI
    assert!(
        capture.contains("! Try") || capture.contains("! for bash mode"),
        "Shell mode should show '!' prefix with suggestion.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Shell mode prefix display matches the captured fixture
#[test]
fn test_tui_shell_prefix_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.12"
        }
        "#,
    );

    let session = "claudeless-shell-prefix-fixture";
    let previous = start_tui(session, &scenario);

    // Press '!' to enter shell mode
    tmux::send_keys(session, "!");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "shell_mode_prefix.txt", None);
}

// =============================================================================
// Shell Mode Command Input Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.17 (Claude Code)
///
/// Typing a command after '!' shows '! command' in the input
#[test]
fn test_tui_shell_mode_shows_command() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-shell-command";
    let previous = start_tui(session, &scenario);

    // Enter shell mode and type a command
    tmux::send_keys(session, "!");
    tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "ls -la");
    let capture = tmux::wait_for_content(session, "ls -la");

    tmux::kill_session(session);

    // Should show '! ls -la' in input (with space after !)
    assert!(
        capture.contains("! ls -la"),
        "Shell mode should show command after '! ' prefix.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Shell mode with command matches the captured fixture
#[test]
fn test_tui_shell_command_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.12"
        }
        "#,
    );

    let session = "claudeless-shell-command-fixture";
    let previous = start_tui(session, &scenario);

    // Enter shell mode and type a command
    tmux::send_keys(session, "!");
    tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "ls -la");
    let capture = tmux::wait_for_content(session, "ls -la");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "shell_mode_command.txt", None);
}

// =============================================================================
// Shell Mode Execution Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Submitting a shell command executes it via Bash
#[test]
fn test_tui_shell_mode_executes_command() {
    let scenario = write_scenario(
        r#"
        name = "test"
        trusted = true
        permission_mode = "bypass-permissions"
        [[responses]]
        pattern = { type = "any" }
        response = "Command executed"
        "#,
    );

    let session = "claudeless-shell-execute";
    let previous = start_tui_ext(session, &scenario, 120, 40, BYPASS_MODE_PATTERN);

    // Enter shell mode, type command, and submit
    tmux::send_keys(session, "!");
    tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "echo hello");
    tmux::wait_for_content(session, "echo hello");
    tmux::send_keys(session, "Enter");

    // Wait for command execution (Bash output)
    let capture = tmux::wait_for_content(session, "Bash");

    tmux::kill_session(session);

    // Should show the command was executed as Bash
    assert!(
        capture.contains("Bash"),
        "Shell mode should execute command via Bash.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.17 (Claude Code)
///
/// Shell command shows as '! command' in conversation history
#[test]
fn test_tui_shell_mode_shows_prefixed_prompt_in_history() {
    let scenario = write_scenario(
        r#"
        name = "test"
        trusted = true
        permission_mode = "bypass-permissions"
        [[responses]]
        pattern = { type = "any" }
        response = "Done"
        "#,
    );

    let session = "claudeless-shell-history";
    let previous = start_tui_ext(session, &scenario, 120, 40, BYPASS_MODE_PATTERN);

    // Enter shell mode, type command, and submit
    tmux::send_keys(session, "!");
    tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "pwd");
    tmux::wait_for_content(session, "pwd");
    tmux::send_keys(session, "Enter");

    // Wait for response
    let capture = tmux::wait_for_content(session, "Done");

    tmux::kill_session(session);

    // Should show the prompt with shell prefix in history
    assert!(
        capture.contains("❯ ! pwd") || capture.contains("! pwd"),
        "Shell mode should show '! command' in conversation history.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// Shell Mode Escape Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.17 (Claude Code)
///
/// Backspace on shell mode prefix '!' exits shell mode and shows placeholder again
#[test]
fn test_tui_shell_mode_backspace_exits_shell_mode() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-shell-backspace";
    let previous = start_tui(session, &scenario);

    // Enter shell mode
    tmux::send_keys(session, "!");
    let with_prefix = tmux::wait_for_change(session, &previous);

    // Verify we're in shell mode (shows '! for bash mode' in status)
    assert!(
        with_prefix.contains("! for bash mode") || with_prefix.contains("! Try"),
        "Should be in shell mode.\nCapture:\n{}",
        with_prefix
    );

    // Backspace to exit shell mode
    tmux::send_keys(session, "BSpace");
    let capture = tmux::wait_for_change(session, &with_prefix);

    tmux::kill_session(session);

    // Should no longer show bash mode indicator and should show normal placeholder
    assert!(
        !capture.contains("! for bash mode"),
        "Backspace should exit shell mode.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Try \"") || capture.contains("? for shortcuts"),
        "Should show placeholder or shortcuts hint after exiting shell mode.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// Shell Mode Special Characters Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.17 (Claude Code)
///
/// Shell mode handles commands with special characters (pipes, redirects)
#[test]
fn test_tui_shell_mode_with_pipe_command() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-shell-pipe";
    let previous = start_tui(session, &scenario);

    // Enter shell mode and type a command with pipe
    tmux::send_keys(session, "!");
    tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "ls | head");
    let capture = tmux::wait_for_content(session, "ls | head");

    tmux::kill_session(session);

    // Should show the full command with pipe
    assert!(
        capture.contains("! ls | head"),
        "Shell mode should handle pipe characters.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.17 (Claude Code)
///
/// Shell mode handles commands with quoted strings
#[test]
fn test_tui_shell_mode_with_quoted_string() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-shell-quotes";
    let previous = start_tui(session, &scenario);

    // Enter shell mode and type a command with quotes
    tmux::send_keys(session, "!");
    tmux::wait_for_change(session, &previous);
    // Note: We use single quotes to avoid tmux key interpretation issues
    tmux::send_keys(session, "echo 'hello world'");
    let capture = tmux::wait_for_content(session, "echo");

    tmux::kill_session(session);

    // Should show the command with quotes
    assert!(
        capture.contains("echo") && capture.contains("hello world"),
        "Shell mode should handle quoted strings.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.17 (Claude Code)
///
/// Shell mode handles commands with environment variables
#[test]
fn test_tui_shell_mode_with_env_variable() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-shell-env";
    let previous = start_tui(session, &scenario);

    // Enter shell mode and type a command with env variable
    tmux::send_keys(session, "!");
    tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "echo $HOME");
    let capture = tmux::wait_for_content(session, "$HOME");

    tmux::kill_session(session);

    // Should show the command with env variable
    assert!(
        capture.contains("! echo $HOME"),
        "Shell mode should handle environment variables.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// Shell Mode ANSI Color Tests (v2.1.17)
// =============================================================================

/// Behavior observed with: claude --version 2.1.17 (Claude Code)
///
/// Shell mode prefix ANSI output matches v2.1.17 fixture.
/// In bash mode, the CLI shows:
/// - Pink separators (RGB 253, 93, 177) instead of gray
/// - `! Try "..."` prompt with `!` in pink (no `❯` prefix)
/// - Status bar shows `! for bash mode` in pink
///
#[test]
#[ignore] // TODO(implement): Requires bash mode pink styling and `!` prefix
fn test_tui_shell_prefix_ansi_matches_fixture_v2117() {
    use common::ansi::assert_versioned_ansi_matches_fixture;

    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.17"
        }
        "#,
    );

    let session = "claudeless-shell-prefix-ansi";
    let previous = start_tui(session, &scenario);

    // Press '!' to enter shell mode
    tmux::send_keys(session, "!");
    let capture = tmux::wait_for_change_ansi(session, &previous);

    tmux::kill_session(session);

    assert_versioned_ansi_matches_fixture(&capture, "v2.1.17", "shell_mode_prefix_ansi.txt", None);
}
