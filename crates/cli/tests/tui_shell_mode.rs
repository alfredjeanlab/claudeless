// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

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

use common::{assert_tui_matches_fixture, TuiTestSession};

const SCENARIO: &str = r#"
    name = "test"
    [[responses]]
    pattern = { type = "any" }
    response = "Hello!"
"#;

const JSON_SCENARIO: &str = r#"
    {
        "default_response": "Hello!",
        "trusted": true,
        "claude_version": "2.1.12"
    }
"#;

const BYPASS_SCENARIO: &str = r#"
    name = "test"
    trusted = true
    permission_mode = "bypass-permissions"
    [[responses]]
    pattern = { type = "any" }
    response = "Command executed"
"#;

const BYPASS_DONE_SCENARIO: &str = r#"
    name = "test"
    trusted = true
    permission_mode = "bypass-permissions"
    [[responses]]
    pattern = { type = "any" }
    response = "Done"
"#;

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
    let tui = TuiTestSession::new("shell-prefix", SCENARIO);
    let previous = tui.capture();

    // Press '!' to enter shell mode
    tui.send_keys("!");
    let capture = tui.wait_for_change(&previous);

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
    let tui = TuiTestSession::new("shell-prefix-fixture", JSON_SCENARIO);
    let previous = tui.capture();

    // Press '!' to enter shell mode
    tui.send_keys("!");
    let capture = tui.wait_for_change(&previous);

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
    let tui = TuiTestSession::new("shell-command", SCENARIO);
    let previous = tui.capture();

    // Enter shell mode and type a command
    tui.send_keys("!");
    tui.wait_for_change(&previous);
    tui.send_keys("ls -la");
    let capture = tui.wait_for("ls -la");

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
    let tui = TuiTestSession::new("shell-command-fixture", JSON_SCENARIO);
    let previous = tui.capture();

    // Enter shell mode and type a command
    tui.send_keys("!");
    tui.wait_for_change(&previous);
    tui.send_keys("ls -la");
    let capture = tui.wait_for("ls -la");

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
    let tui = TuiTestSession::with_custom_wait(
        "shell-execute",
        BYPASS_SCENARIO,
        120,
        40,
        BYPASS_MODE_PATTERN,
    );
    let previous = tui.capture();

    // Enter shell mode, type command, and submit
    tui.send_keys("!");
    tui.wait_for_change(&previous);
    tui.send_keys("echo hello");
    tui.wait_for("echo hello");
    tui.send_keys("Enter");

    // Wait for command execution (Bash output)
    let capture = tui.wait_for("Bash");

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
    let tui = TuiTestSession::with_custom_wait(
        "shell-history",
        BYPASS_DONE_SCENARIO,
        120,
        40,
        BYPASS_MODE_PATTERN,
    );
    let previous = tui.capture();

    // Enter shell mode, type command, and submit
    tui.send_keys("!");
    tui.wait_for_change(&previous);
    tui.send_keys("pwd");
    tui.wait_for("pwd");
    tui.send_keys("Enter");

    // Wait for response
    let capture = tui.wait_for("Done");

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
    let tui = TuiTestSession::new("shell-backspace", SCENARIO);
    let previous = tui.capture();

    // Enter shell mode
    tui.send_keys("!");
    let with_prefix = tui.wait_for_change(&previous);

    // Verify we're in shell mode (shows '! for bash mode' in status)
    assert!(
        with_prefix.contains("! for bash mode") || with_prefix.contains("! Try"),
        "Should be in shell mode.\nCapture:\n{}",
        with_prefix
    );

    // Backspace to exit shell mode
    tui.send_keys("BSpace");
    let capture = tui.wait_for_change(&with_prefix);

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
    let tui = TuiTestSession::new("shell-pipe", SCENARIO);
    let previous = tui.capture();

    // Enter shell mode and type a command with pipe
    tui.send_keys("!");
    tui.wait_for_change(&previous);
    tui.send_keys("ls | head");
    let capture = tui.wait_for("ls | head");

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
    let tui = TuiTestSession::new("shell-quotes", SCENARIO);
    let previous = tui.capture();

    // Enter shell mode and type a command with quotes
    tui.send_keys("!");
    tui.wait_for_change(&previous);
    // Note: We use single quotes to avoid tmux key interpretation issues
    tui.send_keys("echo 'hello world'");
    let capture = tui.wait_for("echo");

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
    let tui = TuiTestSession::new("shell-env", SCENARIO);
    let previous = tui.capture();

    // Enter shell mode and type a command with env variable
    tui.send_keys("!");
    tui.wait_for_change(&previous);
    tui.send_keys("echo $HOME");
    let capture = tui.wait_for("$HOME");

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
    use common::tmux;

    let tui = TuiTestSession::new(
        "shell-prefix-ansi",
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.17"
        }
        "#,
    );
    let previous = tui.capture();

    // Press '!' to enter shell mode
    tui.send_keys("!");
    let capture = tmux::wait_for_change_ansi(tui.name(), &previous);

    assert_versioned_ansi_matches_fixture(&capture, "v2.1.17", "shell_mode_prefix_ansi.txt", None);
}
