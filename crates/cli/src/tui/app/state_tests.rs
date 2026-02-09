// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use crate::state::session::SessionManager;
use crate::time::ClockHandle;
use crate::tui::app::types::{AppMode, ExitHint, TuiConfig};

fn create_test_state() -> TuiAppState {
    let sessions = SessionManager::new();
    let clock = ClockHandle::fake_at_epoch();
    let config = TuiConfig::default();
    TuiAppState::for_test(sessions, clock, config)
}

#[test]
fn idle_input_does_not_need_timer() {
    let state = create_test_state();
    // Mark session start hook as fired (default for idle state)
    state.inner.lock().session_start_hook_fired = true;
    assert!(!state.needs_timer_render());
}

#[test]
fn responding_mode_needs_timer() {
    let state = create_test_state();
    {
        let mut inner = state.inner.lock();
        inner.session_start_hook_fired = true;
        inner.mode = AppMode::Responding;
    }
    assert!(state.needs_timer_render());
}

#[test]
fn thinking_mode_needs_timer() {
    let state = create_test_state();
    {
        let mut inner = state.inner.lock();
        inner.session_start_hook_fired = true;
        inner.mode = AppMode::Thinking;
    }
    assert!(state.needs_timer_render());
}

#[test]
fn compacting_needs_timer() {
    let state = create_test_state();
    {
        let mut inner = state.inner.lock();
        inner.session_start_hook_fired = true;
        inner.is_compacting = true;
    }
    assert!(state.needs_timer_render());
}

#[test]
fn exit_hint_needs_timer() {
    let state = create_test_state();
    {
        let mut inner = state.inner.lock();
        inner.session_start_hook_fired = true;
        inner.display.show_exit_hint(ExitHint::CtrlC, 0);
    }
    assert!(state.needs_timer_render());
}

#[test]
fn pending_hook_message_needs_timer() {
    let state = create_test_state();
    {
        let mut inner = state.inner.lock();
        inner.session_start_hook_fired = true;
        inner.pending_hook_message = Some("continue".to_string());
    }
    assert!(state.needs_timer_render());
}

#[test]
fn pending_initial_prompt_needs_timer() {
    let state = create_test_state();
    {
        let mut inner = state.inner.lock();
        inner.session_start_hook_fired = true;
        inner.pending_initial_prompt = Some("hello".to_string());
    }
    assert!(state.needs_timer_render());
}

#[test]
fn session_start_hook_not_fired_needs_timer() {
    let state = create_test_state();
    // Default: session_start_hook_fired is false
    assert!(!state.inner.lock().session_start_hook_fired);
    assert!(state.needs_timer_render());
}
