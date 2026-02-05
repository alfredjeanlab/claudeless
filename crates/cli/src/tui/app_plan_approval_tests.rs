// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tests for ExitPlanMode plan approval dialog keyboard interaction.
//!
//! Validates behavior matching real Claude Code 2.1.31:
//! - Arrow keys navigate 4 options
//! - Number keys 1-3 select and immediately submit
//! - Free-text typing on option 4
//! - Escape cancels with "User rejected tool use"
//! - Enter submits the highlighted option

use super::*;
use crate::state::session::SessionManager;
use crate::time::ClockHandle;
use crate::tui::app::dialogs::DialogState;
use crate::tui::app::types::AppMode;
use crate::tui::widgets::plan_approval::PlanApprovalState;
use serde_json::json;

fn create_test_app() -> TuiAppState {
    let sessions = SessionManager::new();
    let clock = ClockHandle::fake_at_epoch();
    let tui_config = TuiConfig::default();
    TuiAppState::for_test(sessions, clock, tui_config)
}

fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    let mut event = KeyEvent::new(KeyEventKind::Press, code);
    event.modifiers = modifiers;
    event
}

fn setup_plan_approval(state: &TuiAppState) {
    let input = json!({
        "plan": "# Plan\n\n1. Step one\n2. Step two\n3. Step three"
    });
    let plan_approval = PlanApprovalState::from_tool_input(
        &input,
        "toolu_test".to_string(),
        "~/.claude/plans/happy-yellow-dragon.md".to_string(),
    );
    let mut inner = state.inner.lock();
    inner.dialog = DialogState::PlanApproval(plan_approval);
    inner.mode = AppMode::PlanApproval;
}

// =========================================================================
// Arrow Key Navigation
// =========================================================================

#[test]
fn test_arrow_down_moves_cursor() {
    let state = create_test_app();
    setup_plan_approval(&state);

    // Initially at option 0
    {
        let inner = state.inner.lock();
        assert_eq!(inner.dialog.as_plan_approval().unwrap().cursor, 0);
    }

    // Press Down
    state.handle_plan_approval_key(key_event(KeyCode::Down, KeyModifiers::NONE));

    {
        let inner = state.inner.lock();
        assert_eq!(inner.dialog.as_plan_approval().unwrap().cursor, 1);
    }
}

#[test]
fn test_arrow_up_moves_cursor() {
    let state = create_test_app();
    setup_plan_approval(&state);

    // Move down first, then up
    state.handle_plan_approval_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    state.handle_plan_approval_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    state.handle_plan_approval_key(key_event(KeyCode::Up, KeyModifiers::NONE));

    {
        let inner = state.inner.lock();
        assert_eq!(inner.dialog.as_plan_approval().unwrap().cursor, 1);
    }
}

#[test]
fn test_arrow_up_wraps_to_free_text() {
    let state = create_test_app();
    setup_plan_approval(&state);

    // Up from position 0 wraps to free-text (index 3)
    state.handle_plan_approval_key(key_event(KeyCode::Up, KeyModifiers::NONE));
    {
        let inner = state.inner.lock();
        assert_eq!(inner.dialog.as_plan_approval().unwrap().cursor, 3);
    }
}

#[test]
fn test_arrow_down_clamps_at_free_text() {
    let state = create_test_app();
    setup_plan_approval(&state);

    // 4 options (indices 0..3), move past all then press Down
    for _ in 0..5 {
        state.handle_plan_approval_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    }
    {
        let inner = state.inner.lock();
        assert_eq!(inner.dialog.as_plan_approval().unwrap().cursor, 3);
    }
}

// =========================================================================
// Escape Cancels
// =========================================================================

#[test]
fn test_escape_cancels_plan_approval() {
    let state = create_test_app();
    setup_plan_approval(&state);

    state.handle_plan_approval_key(key_event(KeyCode::Esc, KeyModifiers::NONE));

    let inner = state.inner.lock();
    // Dialog dismissed, back to input mode
    assert!(!inner.dialog.is_active());
    // Response matches real Claude Code
    assert_eq!(inner.display.response_content, "User rejected tool use");
}

// =========================================================================
// Number Keys Select and Submit
// =========================================================================

#[test]
fn test_number_key_selects_option() {
    let state = create_test_app();
    setup_plan_approval(&state);

    // Press '2' — should select auto-accept and submit
    state.handle_plan_approval_key(key_event(KeyCode::Char('2'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    // Dialog was consumed by confirm_plan_approval
    assert!(!inner.dialog.is_active());
}

#[test]
fn test_number_key_3_selects_manual_approve() {
    let state = create_test_app();
    setup_plan_approval(&state);

    // Press '3' — should select manual approve and submit
    state.handle_plan_approval_key(key_event(KeyCode::Char('3'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    assert!(!inner.dialog.is_active());
}

// =========================================================================
// Free-Text Input
// =========================================================================

#[test]
fn test_navigate_to_free_text_and_type() {
    let state = create_test_app();
    setup_plan_approval(&state);

    // Navigate to free-text option (index 3)
    for _ in 0..3 {
        state.handle_plan_approval_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    }

    // Type "Fix it" — should be accepted as free-text input
    state.handle_plan_approval_key(key_event(KeyCode::Char('F'), KeyModifiers::NONE));
    state.handle_plan_approval_key(key_event(KeyCode::Char('i'), KeyModifiers::NONE));
    state.handle_plan_approval_key(key_event(KeyCode::Char('x'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    let plan = inner.dialog.as_plan_approval().unwrap();
    assert_eq!(plan.free_text, "Fix");
    // Dialog still active (not submitted)
    assert!(inner.dialog.is_active());
}

#[test]
fn test_backspace_on_free_text() {
    let state = create_test_app();
    setup_plan_approval(&state);

    // Navigate to free-text
    for _ in 0..3 {
        state.handle_plan_approval_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    }

    // Type "AB" then backspace
    state.handle_plan_approval_key(key_event(KeyCode::Char('A'), KeyModifiers::NONE));
    state.handle_plan_approval_key(key_event(KeyCode::Char('B'), KeyModifiers::NONE));
    state.handle_plan_approval_key(key_event(KeyCode::Backspace, KeyModifiers::NONE));

    let inner = state.inner.lock();
    assert_eq!(inner.dialog.as_plan_approval().unwrap().free_text, "A");
}

#[test]
fn test_number_key_types_on_free_text() {
    let state = create_test_app();
    setup_plan_approval(&state);

    // Navigate to free-text
    for _ in 0..3 {
        state.handle_plan_approval_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    }

    // Number keys should type into free text, not select and submit
    state.handle_plan_approval_key(key_event(KeyCode::Char('4'), KeyModifiers::NONE));
    state.handle_plan_approval_key(key_event(KeyCode::Char('2'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    assert!(inner.dialog.is_active());
    assert_eq!(inner.dialog.as_plan_approval().unwrap().free_text, "42");
}

// =========================================================================
// Ctrl+C Interrupt
// =========================================================================

#[test]
fn test_ctrl_c_cancels_plan_approval() {
    let state = create_test_app();
    setup_plan_approval(&state);

    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));

    let inner = state.inner.lock();
    assert!(!inner.dialog.is_active());
    assert_eq!(inner.display.response_content, "User rejected tool use");
}
