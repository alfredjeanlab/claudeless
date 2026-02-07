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
        assert_eq!(
            inner.dialog.as_elicitation().unwrap().questions[0].cursor,
            0
        );
    }

    // Press Down
    state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));

    {
        let inner = state.inner.lock();
        assert_eq!(
            inner.dialog.as_elicitation().unwrap().questions[0].cursor,
            1
        );
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
        assert_eq!(
            inner.dialog.as_elicitation().unwrap().questions[0].cursor,
            1
        );
    }
}

#[test]
fn test_arrow_up_wraps_to_type_something() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Up from position 0 wraps to "Type something." (index 3, skipping "Chat about this" at 4)
    state.handle_elicitation_key(key_event(KeyCode::Up, KeyModifiers::NONE));
    {
        let inner = state.inner.lock();
        let q = &inner.dialog.as_elicitation().unwrap().questions[0];
        assert_eq!(q.cursor, 3); // "Type something." for 3 defined options
    }
}

#[test]
fn test_arrow_down_clamps_at_chat_about_this() {
    let state = create_test_app();
    setup_elicitation(&state);

    // 3 defined options + "Type something." + "Chat about this" = 5 rows (indices 0..4)
    // Move to last row (index 4), then press Down — should stay at 4
    for _ in 0..5 {
        state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    }
    {
        let inner = state.inner.lock();
        assert_eq!(
            inner.dialog.as_elicitation().unwrap().questions[0].cursor,
            4
        );
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
    assert_eq!(
        inner.display.response_content,
        "User declined to answer questions"
    );
}

// =========================================================================
// Number Keys Select and Submit
// =========================================================================

#[test]
fn test_number_key_selects_and_advances_to_submit() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Press '2' — should select Python (index 1) and advance to submit tab
    // (single question → directly to submit tab)
    state.handle_elicitation_key(key_event(KeyCode::Char('2'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    let elicitation = inner.dialog.as_elicitation().unwrap();
    assert_eq!(elicitation.questions[0].selected, vec![1]);
    assert!(elicitation.on_submit_tab);
}

#[test]
fn test_out_of_range_number_advances_to_submit() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Press '9' — out of range for 3 options, selection unchanged but still advances
    state.handle_elicitation_key(key_event(KeyCode::Char('9'), KeyModifiers::NONE));

    // Dialog still active, advanced to submit tab (single question)
    let inner = state.inner.lock();
    let elicitation = inner.dialog.as_elicitation().unwrap();
    assert!(elicitation.questions[0].selected.is_empty());
    assert!(elicitation.on_submit_tab);
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
    assert_eq!(
        inner.dialog.as_elicitation().unwrap().questions[0].cursor,
        0
    );
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

// =========================================================================
// Free-text "Type something." via Key Handler
// =========================================================================

#[test]
fn test_navigate_to_type_something_and_type() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Navigate past 3 defined options to "Type something." (index 3)
    state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));

    // Type "Hi" — should be accepted as free-text input
    state.handle_elicitation_key(key_event(KeyCode::Char('H'), KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Char('i'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    let q = &inner.dialog.as_elicitation().unwrap().questions[0];
    assert_eq!(q.free_text, "Hi");
    // Dialog still active (not submitted)
    assert!(inner.dialog.is_active());
}

#[test]
fn test_backspace_on_free_text() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Navigate to "Type something."
    for _ in 0..3 {
        state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    }

    // Type "AB" then backspace
    state.handle_elicitation_key(key_event(KeyCode::Char('A'), KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Char('B'), KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Backspace, KeyModifiers::NONE));

    let inner = state.inner.lock();
    assert_eq!(
        inner.dialog.as_elicitation().unwrap().questions[0].free_text,
        "A"
    );
}

#[test]
fn test_space_types_space_on_free_text() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Navigate to "Type something."
    for _ in 0..3 {
        state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    }

    // Space should insert a space character (not toggle selection)
    state.handle_elicitation_key(key_event(KeyCode::Char('a'), KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Char(' '), KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Char('b'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    assert_eq!(
        inner.dialog.as_elicitation().unwrap().questions[0].free_text,
        "a b"
    );
}

#[test]
fn test_number_key_types_on_free_text() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Navigate to "Type something."
    for _ in 0..3 {
        state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    }

    // Number keys should type into free text, not select and submit
    state.handle_elicitation_key(key_event(KeyCode::Char('4'), KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Char('2'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    assert!(inner.dialog.is_active());
    assert_eq!(
        inner.dialog.as_elicitation().unwrap().questions[0].free_text,
        "42"
    );
}

// =========================================================================
// "Chat about this" via Key Handler
// =========================================================================

#[test]
fn test_navigate_to_chat_about_this() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Navigate past 3 defined options + "Type something." to "Chat about this" (index 4)
    for _ in 0..4 {
        state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    }

    let inner = state.inner.lock();
    assert_eq!(
        inner.dialog.as_elicitation().unwrap().questions[0].cursor,
        4
    );
    assert!(inner.dialog.is_active());
}

#[test]
fn test_enter_on_chat_about_this_dismisses_with_clarification() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Navigate to "Chat about this"
    for _ in 0..4 {
        state.handle_elicitation_key(key_event(KeyCode::Down, KeyModifiers::NONE));
    }

    // Press Enter
    state.handle_elicitation_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

    let inner = state.inner.lock();
    assert!(!inner.dialog.is_active());
    assert!(inner
        .display
        .response_content
        .contains("user wants to clarify"));
    assert!(inner.display.response_content.contains("What language?"));
}
