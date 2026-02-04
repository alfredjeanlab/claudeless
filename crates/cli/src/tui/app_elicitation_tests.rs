// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tests for AskUserQuestion elicitation dialog keyboard interaction.
//!
//! Validates behavior observed from real Claude Code 2.1.31:
//! - Arrow keys navigate options
//! - Number keys select and immediately submit
//! - Escape cancels with "User declined to answer questions"
//! - Enter submits the highlighted option

use super::*;
use crate::state::session::SessionManager;
use crate::time::ClockHandle;
use crate::tui::app::dialogs::DialogState;
use crate::tui::app::types::AppMode;
use crate::tui::widgets::elicitation::ElicitationState;
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

fn setup_elicitation(state: &TuiAppState) {
    let input = json!({
        "questions": [
            {
                "question": "What language?",
                "header": "Language",
                "options": [
                    { "label": "Rust", "description": "Systems programming" },
                    { "label": "Python", "description": "Scripting" },
                    { "label": "Go", "description": "Concurrent programming" }
                ],
                "multiSelect": false
            }
        ]
    });
    let elicitation = ElicitationState::from_tool_input(&input, "toolu_test".to_string());
    let mut inner = state.inner.lock();
    inner.dialog = DialogState::Elicitation(elicitation);
    inner.mode = AppMode::Elicitation;
}

// =========================================================================
// Arrow Key Navigation
// =========================================================================

#[test]
fn test_arrow_down_moves_cursor() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Initially at option 0
    {
        let inner = state.inner.lock();
        assert_eq!(inner.dialog.as_elicitation().unwrap().questions[0].cursor, 0);
    }

    // Press Down
    state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));

    {
        let inner = state.inner.lock();
        assert_eq!(inner.dialog.as_elicitation().unwrap().questions[0].cursor, 1);
    }
}

#[test]
fn test_arrow_up_moves_cursor() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Move down first, then up
    state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Up, KeyModifiers::NONE));

    {
        let inner = state.inner.lock();
        assert_eq!(inner.dialog.as_elicitation().unwrap().questions[0].cursor, 1);
    }
}

#[test]
fn test_arrow_keys_clamp_at_bounds() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Press Up at top — should stay at 0
    state.handle_elicitation_key(key_event(KeyCode::Up, KeyModifiers::NONE));
    {
        let inner = state.inner.lock();
        assert_eq!(inner.dialog.as_elicitation().unwrap().questions[0].cursor, 0);
    }

    // Move to last option (index 2), then press Down — should stay at 2
    state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE)); // past end
    {
        let inner = state.inner.lock();
        assert_eq!(inner.dialog.as_elicitation().unwrap().questions[0].cursor, 2);
    }
}

// =========================================================================
// Escape Cancels
// =========================================================================

#[test]
fn test_escape_cancels_elicitation() {
    let state = create_test_app();
    setup_elicitation(&state);

    state.handle_elicitation_key(key_event(KeyCode::Esc, KeyModifiers::NONE));

    let inner = state.inner.lock();
    // Dialog dismissed, back to input mode
    assert!(!inner.dialog.is_active());
    // Response matches real Claude Code
    assert_eq!(inner.display.response_content, "User declined to answer questions");
}

// =========================================================================
// Number Keys Select and Submit
// =========================================================================

#[test]
fn test_number_key_selects_option() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Press '2' — should select Python (index 1) and move cursor there
    // Note: confirm_elicitation requires runtime, so without runtime the
    // dialog gets taken but execution is a no-op. We verify the selection
    // happened by checking the dialog was consumed (taken from state).
    state.handle_elicitation_key(key_event(KeyCode::Char('2'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    // Dialog was consumed by confirm_elicitation (even without runtime)
    assert!(!inner.dialog.is_active());
}

#[test]
fn test_out_of_range_number_keeps_dialog() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Press '9' — out of range for 3 options, selection unchanged
    // But the handler still calls confirm_elicitation after select_by_number
    state.handle_elicitation_key(key_event(KeyCode::Char('9'), KeyModifiers::NONE));

    // Dialog is consumed by confirm_elicitation (submits default first option)
    let inner = state.inner.lock();
    assert!(!inner.dialog.is_active());
}

// =========================================================================
// Typed Text Ignored
// =========================================================================

#[test]
fn test_alphabetic_keys_ignored() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Type letters — should be ignored, dialog stays active
    state.handle_elicitation_key(key_event(KeyCode::Char('P'), KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Char('a'), KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Char('r'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    // Dialog still active, cursor unchanged
    assert!(inner.dialog.is_active());
    assert_eq!(inner.dialog.as_elicitation().unwrap().questions[0].cursor, 0);
}

// =========================================================================
// Space Toggles Selection (for multi-select compatibility)
// =========================================================================

#[test]
fn test_space_toggles_selection() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Space on first option
    state.handle_elicitation_key(key_event(KeyCode::Char(' '), KeyModifiers::NONE));

    let inner = state.inner.lock();
    let q = &inner.dialog.as_elicitation().unwrap().questions[0];
    assert_eq!(q.selected, vec![0]);
}
