// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

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

use common::TuiTestSession;

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
    let tui = TuiTestSession::with_custom_wait(
        "setup-theme-dark",
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        "#,
        120,
        40,
        "Choose the text style",
    );

    let capture = tui.capture();

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
    let tui = TuiTestSession::with_custom_wait(
        "setup-theme-light",
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        "#,
        120,
        40,
        "Choose the text style",
    );

    // Navigate to Light mode (press Down once)
    tui.send_keys("Down");
    let capture = tui.wait_for("GitHub");

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
    let tui = TuiTestSession::with_custom_wait(
        "setup-theme-ansi",
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        "#,
        120,
        40,
        "Choose the text style",
    );

    // Navigate to Dark mode ANSI (option 5 - press Down 4 times)
    tui.send_keys("Down");
    tui.send_keys("Down");
    tui.send_keys("Down");
    tui.send_keys("Down");
    let capture = tui.wait_for("Syntax theme: ansi");

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
    let tui = TuiTestSession::with_custom_wait(
        "setup-theme-toggle",
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        "#,
        120,
        40,
        "Syntax theme:",
    );

    // Capture initial state with syntax enabled
    let enabled = tui.capture();

    // Press Ctrl+T to disable
    tui.send_keys("C-t");
    let disabled = tui.wait_for("Syntax highlighting disabled");

    // Press Ctrl+T to re-enable
    tui.send_keys("C-t");
    let re_enabled = tui.wait_for("Syntax theme:");

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
    let tui = TuiTestSession::with_custom_wait(
        "setup-login-method",
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        "#,
        120,
        40,
        "Choose the text style",
    );

    // Proceed past theme selection
    tui.send_keys("Enter");

    // Wait for login method selection
    let capture = tui.wait_for("Select login method");

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
    let tui = TuiTestSession::with_custom_wait(
        "setup-full-flow",
        r#"
        name = "setup-test"
        [setup]
        enabled = true
        auto_login = "user@example.com"
        "#,
        120,
        40,
        "Choose the text style",
    );

    // Step 1: Theme selection
    let theme_screen = tui.capture();
    assert!(
        theme_screen.contains("Dark mode"),
        "Should show theme options.\nCapture:\n{}",
        theme_screen
    );
    tui.send_keys("Enter");

    // Step 2: Login method selection
    let login_method = tui.wait_for("Select login method");
    assert!(
        login_method.contains("Claude account"),
        "Should show login method.\nCapture:\n{}",
        login_method
    );
    tui.send_keys("Enter");

    // Step 3: Login success (auto-login bypasses browser step)
    let login_success = tui.wait_for("Login successful");
    assert!(
        login_success.contains("Logged in as"),
        "Should show logged in user.\nCapture:\n{}",
        login_success
    );
    tui.send_keys("Enter");

    // Step 4: Security notes
    let security = tui.wait_for("Security notes");
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
    tui.send_keys("Enter");

    // Step 5: Terminal setup
    let terminal = tui.wait_for("terminal setup");
    assert!(
        terminal.contains("recommended settings"),
        "Should show terminal setup options.\nCapture:\n{}",
        terminal
    );
    tui.send_keys("Escape"); // Skip terminal setup

    // Step 6: Initial state
    let initial = tui.wait_for("? for shortcuts");

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
    let tui = TuiTestSession::new(
        "logout",
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    // Type /logout and submit
    let previous = tui.capture();
    tui.send_keys("/logout");
    let _ = tui.wait_for_change(&previous);
    tui.send_keys("Enter");

    // Wait for logout message and shell prompt
    let capture = tui.wait_for_any(&["$", "%", "❯"]);

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
    let tui = TuiTestSession::with_custom_wait(
        "socket-fail",
        r#"
        name = "no-internet-test"
        [connection]
        simulate_failure = "FailedToOpenSocket"
        "#,
        120,
        40,
        "FailedToOpenSocket",
    );

    // Wait for shell prompt (process should have exited)
    let capture = tui.wait_for_any(&["$", "%", "❯"]);

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
    let tui = TuiTestSession::with_custom_wait(
        "socket-fail-msg",
        r#"
        name = "no-internet-test"
        [connection]
        simulate_failure = "FailedToOpenSocket"
        "#,
        120,
        40,
        "FailedToOpenSocket",
    );

    let capture = tui.capture();

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
