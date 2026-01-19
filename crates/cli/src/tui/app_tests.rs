// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::config::ScenarioConfig;
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

#[test]
fn ctrl_c_on_empty_input_shows_exit_hint() {
    let state = create_test_app();

    // First Ctrl+C
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));

    let render = state.render_state();
    assert_eq!(render.exit_hint, Some(ExitHint::CtrlC));
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
    assert_eq!(render.exit_hint, Some(ExitHint::CtrlC));
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

    assert_eq!(state.render_state().exit_hint, Some(ExitHint::CtrlC));

    // Advance time past timeout
    clock.advance_ms(2100);
    state.check_exit_hint_timeout();

    assert_eq!(state.render_state().exit_hint, None);
}

#[test]
fn ctrl_d_on_empty_shows_exit_hint() {
    let state = create_test_app();

    // Ctrl+D on empty input
    state.handle_key_event(key_event(KeyCode::Char('d'), KeyModifiers::CONTROL));

    let render = state.render_state();
    assert_eq!(render.exit_hint, Some(ExitHint::CtrlD));
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
    assert_eq!(state.render_state().exit_hint, None);
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

#[test]
fn typing_clears_exit_hint() {
    let state = create_test_app();

    // First Ctrl+C to show hint
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(state.render_state().exit_hint, Some(ExitHint::CtrlC));

    // Type a character
    state.handle_key_event(key_event(KeyCode::Char('a'), KeyModifiers::empty()));

    // Hint should be cleared
    assert_eq!(state.render_state().exit_hint, None);
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
    assert_eq!(state.render_state().exit_hint, Some(ExitHint::CtrlC));
}

#[test]
fn status_bar_shows_exit_hint() {
    let state = create_test_app();

    // First Ctrl+C to show hint
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));

    let render = state.render_state();
    let status = format_status_bar(&render);
    assert!(status.contains("Press Ctrl-C again to exit"));
}

#[test]
fn status_bar_shows_ctrl_d_hint() {
    let state = create_test_app();

    // First Ctrl+D to show hint
    state.handle_key_event(key_event(KeyCode::Char('d'), KeyModifiers::CONTROL));

    let render = state.render_state();
    let status = format_status_bar(&render);
    assert!(status.contains("Press Ctrl-D again to exit"));
}
