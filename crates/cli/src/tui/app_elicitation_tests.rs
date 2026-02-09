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
        assert_eq!(inner.dialog.as_elicitation().unwrap().questions[0].cursor, 4);
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
fn test_number_key_immediately_submits_single_question() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Press '2' — single question: should select Python and immediately submit
    // (no review page for single-question elicitation)
    state.handle_elicitation_key(key_event(KeyCode::Char('2'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    // Dialog dismissed (confirmed)
    assert!(
        !inner.dialog.is_active(),
        "single-question should immediately submit, not go to review page"
    );
}

#[test]
fn test_out_of_range_number_submits_single_question() {
    let state = create_test_app();
    setup_elicitation(&state);

    // Press '9' — out of range for 3 options, but single question still submits immediately
    state.handle_elicitation_key(key_event(KeyCode::Char('9'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    // Dialog dismissed (confirmed with default first option)
    assert!(
        !inner.dialog.is_active(),
        "single-question should immediately submit even with out-of-range number"
    );
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
    assert_eq!(inner.dialog.as_elicitation().unwrap().questions[0].free_text, "A");
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
    assert_eq!(inner.dialog.as_elicitation().unwrap().questions[0].free_text, "a b");
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
    assert_eq!(inner.dialog.as_elicitation().unwrap().questions[0].free_text, "42");
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
    assert_eq!(inner.dialog.as_elicitation().unwrap().questions[0].cursor, 4);
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
    assert!(inner.display.response_content.contains("user wants to clarify"));
    assert!(inner.display.response_content.contains("What language?"));
}

// =========================================================================
// Multi-question: number keys advance, submit tab required
// =========================================================================

fn setup_multi_question_elicitation(state: &TuiAppState) {
    let input = json!({
        "questions": [
            {
                "question": "What language?",
                "header": "Language",
                "options": [
                    { "label": "Rust", "description": "Systems programming" },
                    { "label": "Python", "description": "Scripting" }
                ],
                "multiSelect": false
            },
            {
                "question": "What project type?",
                "header": "Project",
                "options": [
                    { "label": "CLI", "description": "Command line tool" },
                    { "label": "Web", "description": "Web application" }
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

#[test]
fn test_multi_question_number_key_advances_to_next_question() {
    let state = create_test_app();
    setup_multi_question_elicitation(&state);

    // Press '1' — should select Rust and advance to Q2 (not submit)
    state.handle_elicitation_key(key_event(KeyCode::Char('1'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    let elicitation = inner.dialog.as_elicitation().unwrap();
    assert_eq!(elicitation.questions[0].selected, vec![0]);
    assert_eq!(elicitation.current_question, 1, "should advance to Q2");
    assert!(!elicitation.on_submit_tab, "should not be on submit tab yet");
}

#[test]
fn test_multi_question_last_question_goes_to_submit_tab() {
    let state = create_test_app();
    setup_multi_question_elicitation(&state);

    // Select Q1, then Q2 — should end up on submit tab
    state.handle_elicitation_key(key_event(KeyCode::Char('1'), KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Char('1'), KeyModifiers::NONE));

    let inner = state.inner.lock();
    let elicitation = inner.dialog.as_elicitation().unwrap();
    assert!(elicitation.on_submit_tab, "after answering all questions, should be on submit tab");
    assert!(inner.dialog.is_active(), "dialog should still be active (awaiting submit)");
}

#[test]
fn test_multi_question_enter_on_submit_tab_confirms() {
    let state = create_test_app();
    setup_multi_question_elicitation(&state);

    // Answer both questions, then Enter on submit tab
    state.handle_elicitation_key(key_event(KeyCode::Char('1'), KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::Char('2'), KeyModifiers::NONE));
    // Now on submit tab, cursor at "Submit answers" (0)
    state.handle_elicitation_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

    let inner = state.inner.lock();
    assert!(!inner.dialog.is_active(), "dialog should be dismissed after submit");
}

#[test]
fn test_multi_question_tab_navigates_questions() {
    let state = create_test_app();
    setup_multi_question_elicitation(&state);

    // Tab should advance to Q2
    state.handle_elicitation_key(key_event(KeyCode::Tab, KeyModifiers::NONE));

    let inner = state.inner.lock();
    let elicitation = inner.dialog.as_elicitation().unwrap();
    assert_eq!(elicitation.current_question, 1);
    assert!(!elicitation.on_submit_tab);
}

#[test]
fn test_multi_question_backtab_navigates_back() {
    let state = create_test_app();
    setup_multi_question_elicitation(&state);

    // Tab to Q2, then BackTab to Q1
    state.handle_elicitation_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
    state.handle_elicitation_key(key_event(KeyCode::BackTab, KeyModifiers::NONE));

    let inner = state.inner.lock();
    let elicitation = inner.dialog.as_elicitation().unwrap();
    assert_eq!(elicitation.current_question, 0);
}
