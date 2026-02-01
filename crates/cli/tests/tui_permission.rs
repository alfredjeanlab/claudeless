// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! Permission mode behavioral tests - shift+tab cycling behavior.
//!
//! Tests user interactions with permission mode cycling.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{
    ansi::assert_versioned_ansi_matches_fixture, assert_tui_matches_fixture, start_tui,
    start_tui_ext, tmux, write_scenario, TUI_READY_PATTERN,
};

/// Helper to capture after a sequence of shift+tab presses
fn capture_after_shift_tabs(session: &str, num_tabs: usize) -> String {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );
    let mut previous = start_tui_ext(&session, &scenario, 120, 20, TUI_READY_PATTERN);

    // Send shift+tabs, waiting for UI to update after each
    for _ in 0..num_tabs {
        tmux::send_keys(&session, "BTab");
        previous = tmux::wait_for_change(&session, &previous);
    }

    let capture = previous;

    // Cleanup: first C-c cancels operation, wait for effect, second C-c exits
    tmux::send_keys(&session, "C-c");
    let _ = tmux::wait_for_change(&session, &capture);
    tmux::send_keys(&session, "C-c");
    tmux::kill_session(&session);

    capture
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Shift+Tab cycles through permission modes:
/// - Without --dangerously-skip-permissions: default -> plan -> acceptEdits -> default (3 modes)
/// - With --dangerously-skip-permissions: default -> plan -> acceptEdits -> bypass -> default (4 modes)
#[test]
fn test_shift_tab_cycles_to_plan_mode() {
    let capture = capture_after_shift_tabs("claudeless-shift-tab-1", 1);

    assert!(
        capture.contains("plan") || capture.contains("⏸"),
        "After 1 shift+tab, should show plan mode.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
fn test_shift_tab_cycles_to_accept_edits_mode() {
    let capture = capture_after_shift_tabs("claudeless-shift-tab-2", 2);

    assert!(
        capture.to_lowercase().contains("accept") || capture.contains("⏵⏵"),
        "After 2 shift+tabs, should show accept edits mode.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Without --dangerously-skip-permissions, the cycle is 3 modes:
/// default -> plan -> acceptEdits -> default
#[test]
fn test_shift_tab_cycles_back_to_default() {
    let capture = capture_after_shift_tabs("claudeless-shift-tab-3", 3);

    assert!(
        capture.contains("?") && capture.to_lowercase().contains("shortcut"),
        "After 3 shift+tabs, should cycle back to default mode.\nCapture:\n{}",
        capture
    );
}

/// Compare default permission mode status against real Claude fixture
#[test]
fn test_permission_default_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.12"
        }
        "#,
    );

    let session = tmux::unique_session("fixture-perm-default");
    let capture = start_tui(&session, &scenario);

    tmux::kill_session(&session);

    assert_tui_matches_fixture(&capture, "permission_default.txt", None);
}

/// Compare plan mode status against real Claude fixture
// FIXME: Fixture is golden capture from real CLI v2.1.12. Claudeless incorrectly
// outputs "Use meta+t to toggle thinking" in status bar, but real CLI doesn't.
// Need to update status bar rendering to match real CLI behavior.
#[test]
#[ignore]
fn test_permission_plan_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "permission_mode": "plan",
            "claude_version": "2.1.12"
        }
        "#,
    );

    let session = tmux::unique_session("fixture-perm-plan");
    // Plan mode shows "plan mode on" instead of "? for shortcuts"
    let capture = start_tui_ext(&session, &scenario, 120, 40, "plan mode");

    tmux::kill_session(&session);

    assert_tui_matches_fixture(&capture, "permission_plan.txt", None);
}

// =============================================================================
// Category B: Ignored Tests (require unimplemented TUI features)
// =============================================================================

// FIXME: Fixture is golden capture from real CLI. normalize_tui() doesn't strip
// shell preamble for permission dialogs that appear mid-conversation. The test
// sends input and waits for dialog, but the shell prompt lines before TUI aren't
// stripped. Need to improve preamble detection or test setup.
#[test]
#[ignore]
fn test_permission_bash_command_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "responses": [
                {
                    "pattern": { "type": "contains", "text": "run bash" },
                    "response": {
                        "text": "Sure, let me run that.",
                        "tool_calls": [
                            {
                                "tool": "Bash",
                                "input": { "command": "cat /etc/passwd | head -5", "description": "Display first 5 lines of /etc/passwd" }
                            }
                        ]
                    }
                }
            ]
        }
        "#,
    );

    let session = tmux::unique_session("fixture-perm-bash");
    start_tui(&session, &scenario);

    // Type the trigger prompt
    tmux::send_keys(&session, "run bash");
    tmux::send_keys(&session, "Enter");

    // Wait for bash permission dialog to appear
    let capture = tmux::wait_for_content(&session, "Bash command");

    tmux::kill_session(&session);

    assert_tui_matches_fixture(&capture, "permission_bash_command.txt", None);
}

// FIXME: Fixture is golden capture from real CLI. normalize_tui() doesn't strip
// shell preamble for permission dialogs that appear mid-conversation. The test
// sends input and waits for dialog, but shell prompt lines before TUI aren't stripped.
#[test]
#[ignore]
fn test_permission_edit_file_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "responses": [
                {
                    "pattern": { "type": "contains", "text": "edit file" },
                    "response": {
                        "text": "Sure, let me edit that.",
                        "tool_calls": [
                            {
                                "tool": "Edit",
                                "input": { "file_path": "hello.txt", "old_string": "Hello World", "new_string": "Hello Universe" }
                            }
                        ]
                    }
                }
            ]
        }
        "#,
    );

    let session = tmux::unique_session("fixture-perm-edit");
    start_tui(&session, &scenario);

    // Type the trigger prompt
    tmux::send_keys(&session, "edit file");
    tmux::send_keys(&session, "Enter");

    // Wait for edit permission dialog to appear
    let capture = tmux::wait_for_content(&session, "Edit file");

    tmux::kill_session(&session);

    assert_tui_matches_fixture(&capture, "permission_edit_file.txt", None);
}

// FIXME: Fixture is golden capture from real CLI. normalize_tui() doesn't strip
// shell preamble for permission dialogs that appear mid-conversation. The test
// sends input and waits for dialog, but shell prompt lines before TUI aren't stripped.
#[test]
#[ignore]
fn test_permission_write_file_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "responses": [
                {
                    "pattern": { "type": "contains", "text": "create file" },
                    "response": {
                        "text": "Sure, let me create that.",
                        "tool_calls": [
                            {
                                "tool": "Write",
                                "input": { "file_path": "hello.txt", "content": "Hello World" }
                            }
                        ]
                    }
                }
            ]
        }
        "#,
    );

    let session = tmux::unique_session("fixture-perm-write");
    start_tui(&session, &scenario);

    // Type the trigger prompt
    tmux::send_keys(&session, "create file");
    tmux::send_keys(&session, "Enter");

    // Wait for write permission dialog to appear
    let capture = tmux::wait_for_content(&session, "Create file");

    tmux::kill_session(&session);

    assert_tui_matches_fixture(&capture, "permission_write_file.txt", None);
}

// FIXME: Fixture is golden capture from real CLI. normalize_tui() doesn't strip
// shell preamble for trust folder dialog. The wait_for_content pattern may match
// before TUI is fully rendered, leaving shell prompt visible.
#[test]
#[ignore]
fn test_permission_trust_folder_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": false
        }
        "#,
    );

    let session = tmux::unique_session("fixture-perm-trust");
    // Would need trust prompt to appear
    let capture = start_tui_ext(&session, &scenario, 120, 40, "trust the files");

    tmux::kill_session(&session);

    assert_tui_matches_fixture(&capture, "permission_trust_folder.txt", None);
}

// FIXME: Fixture status_bar_extended.txt is cropped to just the status bar area
// (no TUI header/logo). Either fixture needs recapturing with full TUI, or test
// needs special handling for status-bar-only comparison.
#[test]
#[ignore]
fn test_status_bar_extended_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "permission_mode": "accept-edits",
            "claude_version": "2.1.12"
        }
        "#,
    );

    let session = tmux::unique_session("fixture-status-extended");
    // Accept edits mode should show extended status bar
    let capture = start_tui_ext(&session, &scenario, 120, 40, "accept edits");

    tmux::kill_session(&session);

    assert_tui_matches_fixture(&capture, "status_bar_extended.txt", None);
}

// =============================================================================
// ANSI Color Fixture Tests (v2.1.15)
// =============================================================================

/// Helper to start TUI and wait for content, then capture with ANSI sequences.
fn start_tui_ansi(
    session: &str,
    scenario: &tempfile::NamedTempFile,
    width: u16,
    height: u16,
    wait_for: &str,
) -> String {
    use common::claudeless_bin;

    tmux::kill_session(&session);
    tmux::new_session(&session, width, height);

    let cmd = format!(
        "{} --scenario {}",
        claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(&session, &cmd);

    // Wait for pattern, then capture with ANSI sequences
    tmux::wait_for_content_ansi(&session, wait_for)
}

/// Compare default permission mode ANSI output against v2.1.15 fixture
#[test]
fn test_permission_default_ansi_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.15"
        }
        "#,
    );

    let session = tmux::unique_session("fixture-perm-default-ansi");
    let capture = start_tui_ansi(&session, &scenario, 120, 40, TUI_READY_PATTERN);

    tmux::kill_session(&session);

    assert_versioned_ansi_matches_fixture(&capture, "v2.1.15", "permission_default_ansi.txt", None);
}

/// Compare plan mode ANSI output against v2.1.15 fixture
// FIXME: Fixture is golden capture from real CLI v2.1.15. Claudeless incorrectly
// outputs "Use meta+t to toggle thinking" in status bar, but real CLI doesn't.
// Need to update status bar rendering to match real CLI behavior.
#[test]
#[ignore]
fn test_permission_plan_ansi_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "permission_mode": "plan",
            "claude_version": "2.1.15"
        }
        "#,
    );

    let session = tmux::unique_session("fixture-perm-plan-ansi");
    let capture = start_tui_ansi(&session, &scenario, 120, 40, "plan mode");

    tmux::kill_session(&session);

    assert_versioned_ansi_matches_fixture(&capture, "v2.1.15", "permission_plan_ansi.txt", None);
}

/// Compare accept edits mode ANSI output against v2.1.15 fixture
// FIXME: Fixture is golden capture from real CLI v2.1.15. Claudeless incorrectly
// outputs "Use meta+t to toggle thinking" in status bar, but real CLI doesn't.
// Need to update status bar rendering to match real CLI behavior.
#[test]
#[ignore]
fn test_permission_accept_edits_ansi_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "permission_mode": "accept-edits",
            "claude_version": "2.1.15"
        }
        "#,
    );

    let session = tmux::unique_session("fixture-perm-accept-edits-ansi");
    let capture = start_tui_ansi(&session, &scenario, 120, 40, "accept edits");

    tmux::kill_session(&session);

    assert_versioned_ansi_matches_fixture(
        &capture,
        "v2.1.15",
        "permission_accept_edits_ansi.txt",
        None,
    );
}

/// Compare bypass permissions mode ANSI output against v2.1.15 fixture
// FIXME: Fixture is golden capture from real CLI v2.1.15. Claudeless incorrectly
// outputs "Use meta+t to toggle thinking" in status bar, but real CLI doesn't.
// Need to update status bar rendering to match real CLI behavior.
#[test]
#[ignore]
fn test_permission_bypass_ansi_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "permission_mode": "bypass-permissions",
            "claude_version": "2.1.15"
        }
        "#,
    );

    let session = tmux::unique_session("fixture-perm-bypass-ansi");
    let capture = start_tui_ansi(&session, &scenario, 120, 40, "bypass permissions");

    tmux::kill_session(&session);

    assert_versioned_ansi_matches_fixture(&capture, "v2.1.15", "permission_bypass_ansi.txt", None);
}
