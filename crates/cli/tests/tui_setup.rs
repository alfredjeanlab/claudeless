// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TUI setup and login flow tests.
//!
//! Behavior observed with: claude --version 2.1.14 (Claude Code)
//!
//! ## Setup Flow Behavior
//! 1. Theme selection - shows 6 theme options with syntax highlighting preview
//!    - Ctrl+T toggles syntax highlighting preview on/off
//! 2. Login method - Claude subscription vs Anthropic Console API
//! 3. Browser login - shows OAuth URL and code input field
//! 4. Login success - shows email and "Press Enter to continue"
//! 5. Security notes - prompt injection warning, "Press Enter to continue"
//! 6. Terminal setup - recommended settings vs skip
//! 7. Initial state - normal TUI ready for input
//!
//! ## /logout Behavior
//! - Shows "Successfully logged out" message
//! - Exits Claude Code immediately (returns to shell prompt)
//!
//! ## Failed to Open Socket Behavior
//! - Shows "Unable to connect to Anthropic services" error
//! - Exits Claude Code immediately (returns to shell prompt)

mod common;

use common::{tmux, write_scenario};

// =============================================================================
// Setup Flow Rendering Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Theme selection shows 6 options and syntax highlighting preview.
/// Dark mode is selected by default (option 1 with ✔).
/// See fixture: setup_01_select_theme_dark.txt
#[test]
#[ignore] // DEFERRED: Requires setup flow implementation
fn test_tui_setup_theme_selection_dark_mode_default() {
    let scenario = write_scenario(
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        "#,
    );

    let session = "claudeless-setup-theme-dark";
    tmux::kill_session(session);
    tmux::new_session(session, 120, 40);

    let cmd = format!(
        "{} --scenario {} --tui",
        common::claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);

    // Wait for theme selection screen
    let capture = tmux::wait_for_content(session, "Choose the text style");

    tmux::kill_session(session);

    // Verify theme options are shown
    assert!(
        capture.contains("Dark mode"),
        "Should show Dark mode option.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Light mode"),
        "Should show Light mode option.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("colorblind-friendly"),
        "Should show colorblind-friendly options.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("ANSI colors only"),
        "Should show ANSI-only options.\nCapture:\n{}",
        capture
    );

    // Verify dark mode is selected by default
    assert!(
        capture.contains("1. Dark mode") && capture.contains("✔"),
        "Dark mode should be selected by default.\nCapture:\n{}",
        capture
    );

    // Verify syntax highlighting preview with dark theme
    assert!(
        capture.contains("function greet()"),
        "Should show syntax highlighting preview.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Monokai Extended"),
        "Should show Monokai Extended syntax theme for dark mode.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Selecting light mode (option 2) changes the logo art and syntax theme.
/// Light mode uses "GitHub" syntax theme instead of "Monokai Extended".
/// See fixture: setup_01_select_theme_light.txt
#[test]
#[ignore] // DEFERRED: Requires setup flow implementation
fn test_tui_setup_theme_selection_light_mode() {
    let scenario = write_scenario(
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        "#,
    );

    let session = "claudeless-setup-theme-light";
    tmux::kill_session(session);
    tmux::new_session(session, 120, 40);

    let cmd = format!(
        "{} --scenario {} --tui",
        common::claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);

    // Wait for theme selection screen
    let _ = tmux::wait_for_content(session, "Choose the text style");

    // Navigate to Light mode (press Down once)
    tmux::send_keys(session, "Down");
    let capture = tmux::wait_for_content(session, "GitHub");

    tmux::kill_session(session);

    // Verify light mode is now selected (cursor on option 2)
    assert!(
        capture.contains("❯ 2. Light mode"),
        "Light mode should be selected.\nCapture:\n{}",
        capture
    );

    // Verify syntax theme changes to GitHub for light mode
    assert!(
        capture.contains("GitHub"),
        "Should show GitHub syntax theme for light mode.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Selecting ANSI-only mode (option 5 or 6) uses "ansi" syntax theme.
/// ANSI modes have limited colors for terminal compatibility.
#[test]
#[ignore] // DEFERRED: Requires setup flow implementation
fn test_tui_setup_theme_selection_ansi_mode() {
    let scenario = write_scenario(
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        "#,
    );

    let session = "claudeless-setup-theme-ansi";
    tmux::kill_session(session);
    tmux::new_session(session, 120, 40);

    let cmd = format!(
        "{} --scenario {} --tui",
        common::claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);

    // Wait for theme selection screen
    let _ = tmux::wait_for_content(session, "Choose the text style");

    // Navigate to Dark mode ANSI (option 5 - press Down 4 times)
    tmux::send_keys(session, "Down");
    tmux::send_keys(session, "Down");
    tmux::send_keys(session, "Down");
    tmux::send_keys(session, "Down");
    let capture = tmux::wait_for_content(session, "Syntax theme: ansi");

    tmux::kill_session(session);

    // Verify ANSI mode is selected
    assert!(
        capture.contains("5. Dark mode (ANSI colors only)"),
        "ANSI dark mode should be highlighted.\nCapture:\n{}",
        capture
    );

    // Verify syntax theme is "ansi"
    assert!(
        capture.contains("Syntax theme: ansi"),
        "Should show ansi syntax theme for ANSI mode.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Ctrl+T on theme selection toggles syntax highlighting preview
#[test]
#[ignore] // DEFERRED: Requires setup flow implementation
fn test_tui_setup_theme_ctrl_t_toggles_syntax_highlighting() {
    let scenario = write_scenario(
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        "#,
    );

    let session = "claudeless-setup-theme-toggle";
    tmux::kill_session(session);
    tmux::new_session(session, 120, 40);

    let cmd = format!(
        "{} --scenario {} --tui",
        common::claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);

    // Wait for theme selection with syntax enabled
    let enabled = tmux::wait_for_content(session, "Syntax theme:");

    // Press Ctrl+T to disable
    tmux::send_keys(session, "C-t");
    let disabled = tmux::wait_for_content(session, "Syntax highlighting disabled");

    // Press Ctrl+T to re-enable
    tmux::send_keys(session, "C-t");
    let re_enabled = tmux::wait_for_content(session, "Syntax theme:");

    tmux::kill_session(session);

    assert!(
        enabled.contains("Syntax theme:"),
        "Initial state should show syntax theme.\nCapture:\n{}",
        enabled
    );
    assert!(
        disabled.contains("Syntax highlighting disabled"),
        "After Ctrl+T should show disabled.\nCapture:\n{}",
        disabled
    );
    assert!(
        re_enabled.contains("Syntax theme:"),
        "After second Ctrl+T should show enabled.\nCapture:\n{}",
        re_enabled
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Login method selection shows Claude subscription and API options
#[test]
#[ignore] // DEFERRED: Requires setup flow implementation
fn test_tui_setup_login_method_shows_options() {
    let scenario = write_scenario(
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        "#,
    );

    let session = "claudeless-setup-login-method";
    tmux::kill_session(session);
    tmux::new_session(session, 120, 40);

    let cmd = format!(
        "{} --scenario {} --tui",
        common::claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);

    // Wait for theme selection then proceed
    let _ = tmux::wait_for_content(session, "Choose the text style");
    tmux::send_keys(session, "Enter");

    // Wait for login method selection
    let capture = tmux::wait_for_content(session, "Select login method");

    tmux::kill_session(session);

    assert!(
        capture.contains("Claude account with subscription"),
        "Should show Claude subscription option.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Pro, Max, Team, or Enterprise"),
        "Should show subscription types.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("Anthropic Console account"),
        "Should show Console option.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("API usage billing"),
        "Should show API billing note.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// Full Login Flow Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Complete setup flow from theme selection to initial state:
/// 1. Theme selection (Enter to accept default)
/// 2. Login method (Enter to accept default)
/// 3. Browser login (simulated with auto-login)
/// 4. Security notes (Enter to continue)
/// 5. Terminal setup (Enter to accept or Esc to skip)
/// 6. Initial state with "? for shortcuts"
#[test]
#[ignore] // DEFERRED: Requires full setup flow implementation
fn test_tui_setup_full_login_flow_to_initial_state() {
    let scenario = write_scenario(
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        auto_login = "user@example.com"
        "#,
    );

    let session = "claudeless-setup-full-flow";
    tmux::kill_session(session);
    tmux::new_session(session, 120, 40);

    let cmd = format!(
        "{} --scenario {} --tui",
        common::claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);

    // Step 1: Theme selection
    let theme_screen = tmux::wait_for_content(session, "Choose the text style");
    assert!(
        theme_screen.contains("Dark mode"),
        "Should show theme options.\nCapture:\n{}",
        theme_screen
    );
    tmux::send_keys(session, "Enter");

    // Step 2: Login method selection
    let login_method = tmux::wait_for_content(session, "Select login method");
    assert!(
        login_method.contains("Claude account"),
        "Should show login method.\nCapture:\n{}",
        login_method
    );
    tmux::send_keys(session, "Enter");

    // Step 3: Login success (auto-login bypasses browser step)
    let login_success = tmux::wait_for_content(session, "Login successful");
    assert!(
        login_success.contains("Logged in as"),
        "Should show logged in user.\nCapture:\n{}",
        login_success
    );
    tmux::send_keys(session, "Enter");

    // Step 4: Security notes
    let security = tmux::wait_for_content(session, "Security notes");
    assert!(
        security.contains("Claude can make mistakes"),
        "Should show security warning.\nCapture:\n{}",
        security
    );
    assert!(
        security.contains("prompt injection"),
        "Should mention prompt injection.\nCapture:\n{}",
        security
    );
    tmux::send_keys(session, "Enter");

    // Step 5: Terminal setup
    let terminal = tmux::wait_for_content(session, "terminal setup");
    assert!(
        terminal.contains("recommended settings"),
        "Should show terminal setup options.\nCapture:\n{}",
        terminal
    );
    tmux::send_keys(session, "Escape"); // Skip terminal setup

    // Step 6: Initial state
    let initial = tmux::wait_for_content(session, "? for shortcuts");

    tmux::kill_session(session);

    // Verify we reached the normal TUI state
    assert!(
        initial.contains("? for shortcuts"),
        "Should reach initial state.\nCapture:\n{}",
        initial
    );
    // Check header elements
    assert!(
        initial.contains("Claude Code"),
        "Should show Claude Code in header.\nCapture:\n{}",
        initial
    );
}

// =============================================================================
// /logout Command Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// /logout command shows success message and exits Claude Code.
/// After /logout, the shell prompt should be visible.
#[test]
#[ignore] // DEFERRED: Requires /logout command implementation
fn test_tui_slash_logout_exits_to_shell() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-logout";
    let previous = common::start_tui(session, &scenario);

    // Type /logout and submit
    tmux::send_keys(session, "/logout");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");

    // Wait for logout message and shell prompt
    let capture = tmux::wait_for_any(session, &["$", "%", "❯"]);

    tmux::kill_session(session);

    // Should show logout success message
    assert!(
        capture.contains("Successfully logged out"),
        "/logout should show success message.\nCapture:\n{}",
        capture
    );

    // Should have exited to shell (tmux prompt visible)
    assert!(
        capture.contains("$") || capture.contains("%") || capture.contains("❯"),
        "/logout should exit to shell prompt.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// Connection Error Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// When started with no internet connection, shows "Unable to connect" error
/// and exits immediately to shell.
#[test]
#[ignore] // DEFERRED: Requires connection error handling
fn test_tui_failed_to_open_socket_exits() {
    let scenario = write_scenario(
        r#"
        name = "no-internet-test"
        [connection]
        simulate_failure = "FailedToOpenSocket"
        "#,
    );

    let session = "claudeless-socket-fail";
    tmux::kill_session(session);
    tmux::new_session(session, 120, 40);

    let cmd = format!(
        "{} --scenario {} --tui",
        common::claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);

    // Wait for error message and shell prompt
    let capture = tmux::wait_for_any(session, &["$", "%", "❯"]);

    tmux::kill_session(session);

    // Should show connection error
    assert!(
        capture.contains("Unable to connect to Anthropic services"),
        "Should show connection error.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("FailedToOpenSocket"),
        "Should show socket error.\nCapture:\n{}",
        capture
    );

    // Should have exited to shell
    assert!(
        capture.contains("$") || capture.contains("%") || capture.contains("❯"),
        "Connection error should exit to shell prompt.\nCapture:\n{}",
        capture
    );
}

/// Behavior observed with: claude --version 2.1.14 (Claude Code)
///
/// Connection error message includes helpful troubleshooting hints.
#[test]
#[ignore] // DEFERRED: Requires connection error handling
fn test_tui_failed_to_open_socket_shows_helpful_message() {
    let scenario = write_scenario(
        r#"
        name = "no-internet-test"
        [connection]
        simulate_failure = "FailedToOpenSocket"
        "#,
    );

    let session = "claudeless-socket-fail-msg";
    tmux::kill_session(session);
    tmux::new_session(session, 120, 40);

    let cmd = format!(
        "{} --scenario {} --tui",
        common::claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);

    // Wait for error to appear
    let capture = tmux::wait_for_content(session, "FailedToOpenSocket");

    tmux::kill_session(session);

    // Should show helpful troubleshooting information
    assert!(
        capture.contains("check your internet connection"),
        "Should suggest checking internet.\nCapture:\n{}",
        capture
    );
    assert!(
        capture.contains("supported countries") || capture.contains("anthropic.com"),
        "Should mention country availability.\nCapture:\n{}",
        capture
    );
}
