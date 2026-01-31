// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::render::{format_header_lines, format_status_bar};
use super::types::ExitHint;
use super::*;
use crate::ansi::strip_ansi;
use crate::config::ScenarioConfig;
use crate::permission::PermissionMode;
use crate::scenario::Scenario;
use crate::state::session::SessionManager;
use crate::time::ClockHandle;

fn create_test_app() -> TuiAppState {
    let config = ScenarioConfig::default();
    let scenario = Scenario::from_config(config).unwrap();
    let sessions = SessionManager::new();
    let clock = ClockHandle::fake_at_epoch();
    let tui_config = TuiConfig::default();
    TuiAppState::new(scenario, sessions, clock, tui_config)
}

fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    let mut event = KeyEvent::new(KeyEventKind::Press, code);
    event.modifiers = modifiers;
    event
}

// =========================================================================
// Ctrl+C Exit Tests
// =========================================================================

#[test]
fn ctrl_c_on_empty_input_shows_exit_hint() {
    let state = create_test_app();

    // First Ctrl+C
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));

    let render = state.render_state();
    assert_eq!(render.display.exit_hint, Some(ExitHint::CtrlC));
    assert!(!state.should_exit());
}

#[test]
fn ctrl_c_with_text_clears_and_shows_hint() {
    let state = create_test_app();

    // Type some text
    state.handle_key_event(key_event(KeyCode::Char('h'), KeyModifiers::empty()));

    assert_eq!(state.input_buffer(), "h");

    // First Ctrl+C
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(state.input_buffer(), "");
    let render = state.render_state();
    assert_eq!(render.display.exit_hint, Some(ExitHint::CtrlC));
    assert!(!state.should_exit());
}

#[test]
fn double_ctrl_c_exits() {
    let state = create_test_app();

    // First Ctrl+C
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(!state.should_exit());

    // Second Ctrl+C (within timeout)
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert!(state.should_exit());
    assert_eq!(state.exit_reason(), Some(ExitReason::Interrupted));
}

#[test]
fn ctrl_c_hint_times_out() {
    let state = create_test_app();
    let clock = state.inner.lock().clock.as_fake().unwrap().clone();

    // First Ctrl+C
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(
        state.render_state().display.exit_hint,
        Some(ExitHint::CtrlC)
    );

    // Advance time past timeout
    clock.advance_ms(2100);
    state.check_exit_hint_timeout();

    assert_eq!(state.render_state().display.exit_hint, None);
}

// =========================================================================
// Ctrl+D Exit Tests
// =========================================================================

#[test]
fn ctrl_d_on_empty_shows_exit_hint() {
    let state = create_test_app();

    // Ctrl+D on empty input
    state.handle_key_event(key_event(KeyCode::Char('d'), KeyModifiers::CONTROL));

    let render = state.render_state();
    assert_eq!(render.display.exit_hint, Some(ExitHint::CtrlD));
    assert!(!state.should_exit());
}

#[test]
fn ctrl_d_with_text_is_ignored() {
    let state = create_test_app();

    // Type some text
    state.handle_key_event(key_event(KeyCode::Char('x'), KeyModifiers::empty()));

    // Ctrl+D with text
    state.handle_key_event(key_event(KeyCode::Char('d'), KeyModifiers::CONTROL));

    // Should be ignored - no hint, no exit
    assert_eq!(state.input_buffer(), "x");
    assert_eq!(state.render_state().display.exit_hint, None);
    assert!(!state.should_exit());
}

#[test]
fn double_ctrl_d_exits() {
    let state = create_test_app();

    // First Ctrl+D
    state.handle_key_event(key_event(KeyCode::Char('d'), KeyModifiers::CONTROL));

    // Second Ctrl+D
    state.handle_key_event(key_event(KeyCode::Char('d'), KeyModifiers::CONTROL));

    assert!(state.should_exit());
    assert_eq!(state.exit_reason(), Some(ExitReason::UserQuit));
}

// =========================================================================
// Exit Hint Behavior Tests
// =========================================================================

#[test]
fn typing_clears_exit_hint() {
    let state = create_test_app();

    // First Ctrl+C to show hint
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(
        state.render_state().display.exit_hint,
        Some(ExitHint::CtrlC)
    );

    // Type a character
    state.handle_key_event(key_event(KeyCode::Char('a'), KeyModifiers::empty()));

    // Hint should be cleared
    assert_eq!(state.render_state().display.exit_hint, None);
    assert_eq!(state.input_buffer(), "a");
}

#[test]
fn ctrl_c_after_timeout_shows_new_hint() {
    let state = create_test_app();
    let clock = state.inner.lock().clock.as_fake().unwrap().clone();

    // First Ctrl+C
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(!state.should_exit());

    // Advance time past timeout
    clock.advance_ms(2100);

    // Second Ctrl+C (after timeout - should show hint again, not exit)
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert!(!state.should_exit());
    assert_eq!(
        state.render_state().display.exit_hint,
        Some(ExitHint::CtrlC)
    );
}

#[test]
fn status_bar_shows_exit_hint() {
    let state = create_test_app();

    // First Ctrl+C to show hint
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));

    let render = state.render_state();
    let status = format_status_bar(&render, render.display.terminal_width as usize);
    assert!(status.contains("Press Ctrl-C again to exit"));
}

#[test]
fn status_bar_shows_ctrl_d_hint() {
    let state = create_test_app();

    // First Ctrl+D to show hint
    state.handle_key_event(key_event(KeyCode::Char('d'), KeyModifiers::CONTROL));

    let render = state.render_state();
    let status = format_status_bar(&render, render.display.terminal_width as usize);
    assert!(status.contains("Press Ctrl-D again to exit"));
}

// =========================================================================
// Version Display Tests
// =========================================================================

#[test]
fn tui_config_default_has_no_claude_version() {
    let config = TuiConfig::default();
    assert!(config.claude_version.is_none());
}

#[test]
fn header_shows_claudeless_when_no_version_specified() {
    let state = create_test_app();
    let render = state.render_state();

    assert!(render.claude_version.is_none());

    let (line1, _, _) = format_header_lines(&render);
    // Strip ANSI codes for text content checks (line may have color styling)
    let line1_plain = strip_ansi(&line1);
    assert!(line1_plain.contains("Claudeless"));
    assert!(!line1_plain.contains("Claude Code"));
}

#[test]
fn header_shows_claude_code_when_version_specified() {
    let config = ScenarioConfig {
        claude_version: Some("2.1.12".to_string()),
        ..Default::default()
    };
    let scenario = Scenario::from_config(config).unwrap();
    let sessions = SessionManager::new();
    let clock = ClockHandle::fake_at_epoch();

    let tui_config = TuiConfig::from_scenario(
        scenario.config(),
        None,
        &PermissionMode::Default,
        false,
        None,
        false,
    );
    let state = TuiAppState::new(scenario, sessions, clock, tui_config);
    let render = state.render_state();

    assert_eq!(render.claude_version, Some("2.1.12".to_string()));

    let (line1, _, _) = format_header_lines(&render);
    // Strip ANSI codes for text content checks (line has color styling)
    let line1_plain = strip_ansi(&line1);
    assert!(line1_plain.contains("Claude Code v2.1.12"));
    assert!(!line1_plain.contains("Claudeless"));
}

#[test]
fn cli_version_overrides_scenario() {
    let scenario_config = ScenarioConfig {
        claude_version: Some("1.0.0".to_string()),
        ..Default::default()
    };

    let tui_config = TuiConfig::from_scenario(
        &scenario_config,
        None,
        &PermissionMode::Default,
        false,
        Some("2.0.0"), // CLI override
        false,
    );

    assert_eq!(tui_config.claude_version, Some("2.0.0".to_string()));
}

// =========================================================================
// Escape to Clear Input Tests
// =========================================================================

#[test]
fn escape_with_text_shows_clear_hint() {
    let state = create_test_app();

    // Type some text
    state.handle_key_event(key_event(KeyCode::Char('x'), KeyModifiers::empty()));

    // Press Escape
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));

    let render = state.render_state();
    assert_eq!(render.display.exit_hint, Some(ExitHint::Escape));
    assert_eq!(render.input.buffer, "x"); // Input still present
}

#[test]
fn double_escape_clears_input() {
    let state = create_test_app();

    // Type some text
    state.handle_key_event(key_event(KeyCode::Char('x'), KeyModifiers::empty()));

    // Double-tap Escape
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));

    let render = state.render_state();
    assert!(render.input.buffer.is_empty());
    assert_eq!(render.display.exit_hint, None);
}

#[test]
fn escape_on_empty_input_does_nothing() {
    let state = create_test_app();

    // Escape on empty input
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));

    let render = state.render_state();
    assert_eq!(render.display.exit_hint, None);
    assert!(render.input.buffer.is_empty());
}

#[test]
fn escape_clear_hint_times_out() {
    let state = create_test_app();
    let clock = state.inner.lock().clock.as_fake().unwrap().clone();

    // Type text and press Escape
    state.handle_key_event(key_event(KeyCode::Char('x'), KeyModifiers::empty()));
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));

    assert_eq!(
        state.render_state().display.exit_hint,
        Some(ExitHint::Escape)
    );

    // Advance time past timeout
    clock.advance_ms(2100);
    state.check_exit_hint_timeout();

    assert_eq!(state.render_state().display.exit_hint, None);
    assert_eq!(state.render_state().input.buffer, "x"); // Input not cleared
}

#[test]
fn escape_after_timeout_shows_hint_again() {
    let state = create_test_app();
    let clock = state.inner.lock().clock.as_fake().unwrap().clone();

    // Type text, press Escape, wait for timeout
    state.handle_key_event(key_event(KeyCode::Char('x'), KeyModifiers::empty()));
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));
    clock.advance_ms(2100);
    state.check_exit_hint_timeout();

    // Press Escape again - should show hint (not clear)
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));

    let render = state.render_state();
    assert_eq!(render.display.exit_hint, Some(ExitHint::Escape));
    assert_eq!(render.input.buffer, "x"); // Still present
}
