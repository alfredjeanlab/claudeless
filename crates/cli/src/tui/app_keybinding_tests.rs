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

// ========================
// Ctrl+_ Undo Tests
// ========================

#[test]
fn ctrl_underscore_undoes_to_previous_word_boundary() {
    let state = create_test_app();

    // Type "hello world"
    for c in "hello ".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    for c in "world".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    assert_eq!(state.render_state().input.buffer, "hello world");

    // Ctrl+_ should undo back to "hello" (snapshot taken before space was typed)
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));
    assert_eq!(state.render_state().input.buffer, "hello");
}

#[test]
fn ctrl_underscore_on_empty_does_nothing() {
    let state = create_test_app();

    // Press Ctrl+_ on empty input
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));

    assert_eq!(state.render_state().input.buffer, "");
}

#[test]
fn ctrl_underscore_clears_all_with_multiple_presses() {
    let state = create_test_app();

    // Type "one two three"
    for word in ["one ", "two ", "three"] {
        for c in word.chars() {
            state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
        }
    }

    // Undo all words
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));

    assert_eq!(state.render_state().input.buffer, "");
}

#[test]
fn undo_stack_clears_on_submit() {
    let state = create_test_app();

    for c in "test".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // Submit clears undo stack
    state.handle_key_event(key_event(KeyCode::Enter, KeyModifiers::NONE));

    // Ctrl+_ should do nothing now
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));
    assert_eq!(state.render_state().input.buffer, "");
}

#[test]
fn ctrl_underscore_via_unit_separator_character() {
    // Test that the 0x1f (unit separator) character encoding also triggers undo
    // This is how terminals often send Ctrl+_
    let state = create_test_app();

    // Type "hello world"
    for c in "hello world".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    assert_eq!(state.render_state().input.buffer, "hello world");

    // Send as ASCII 0x1F (unit separator) - how terminals encode Ctrl+_
    state.handle_key_event(key_event(KeyCode::Char('\x1f'), KeyModifiers::NONE));
    assert_eq!(state.render_state().input.buffer, "hello");
}

#[test]
fn ctrl_underscore_via_unit_separator_with_control_modifier() {
    // Test with CONTROL modifier (crossterm might add this)
    let state = create_test_app();

    for c in "hello world".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // Send as '\x1f' with CONTROL modifier
    state.handle_key_event(key_event(KeyCode::Char('\x1f'), KeyModifiers::CONTROL));
    assert_eq!(state.render_state().input.buffer, "hello");
}

#[test]
fn ctrl_slash_also_triggers_undo() {
    // Ctrl+/ is often the same terminal sequence as Ctrl+_
    let state = create_test_app();

    for c in "hello world".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    state.handle_key_event(key_event(KeyCode::Char('/'), KeyModifiers::CONTROL));
    assert_eq!(state.render_state().input.buffer, "hello");
}

// ========================
// Ctrl+Z Suspend Tests
// ========================

#[test]
fn ctrl_z_triggers_suspend_exit() {
    let state = create_test_app();

    // Type some input first
    for c in "hello".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // Ctrl+Z should trigger exit with Suspended reason
    state.handle_key_event(key_event(KeyCode::Char('z'), KeyModifiers::CONTROL));

    assert!(state.should_exit());
    assert_eq!(state.exit_reason(), Some(ExitReason::Suspended));
}

#[test]
fn ctrl_z_raw_char_triggers_suspend_exit() {
    let state = create_test_app();

    // Ctrl+Z may come as raw ASCII 0x1A
    state.handle_key_event(key_event(KeyCode::Char('\x1a'), KeyModifiers::NONE));

    assert!(state.should_exit());
    assert_eq!(state.exit_reason(), Some(ExitReason::Suspended));
}

#[test]
fn state_preserved_after_suspend_request() {
    let state = create_test_app();

    // Type some input
    for c in "hello world".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // Trigger suspend
    state.handle_key_event(key_event(KeyCode::Char('z'), KeyModifiers::CONTROL));

    // State should be unchanged (preserved for resume)
    assert_eq!(state.render_state().input.buffer, "hello world");
}

#[test]
fn clear_exit_state_resets_for_resume() {
    let state = create_test_app();

    // Trigger suspend
    state.handle_key_event(key_event(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert!(state.should_exit());
    assert_eq!(state.exit_reason(), Some(ExitReason::Suspended));

    // Clear exit state (simulates what happens after resume)
    state.clear_exit_state();

    // Should be able to continue
    assert!(!state.should_exit());
    assert_eq!(state.exit_reason(), None);
}

// ========================
// Ctrl+S Stash Tests
// ========================

#[test]
fn ctrl_s_stashes_non_empty_input() {
    let state = create_test_app();

    // Type some text
    for c in "hello world".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    assert_eq!(state.input_buffer(), "hello world");

    // Ctrl+S to stash
    state.handle_key_event(key_event(KeyCode::Char('s'), KeyModifiers::CONTROL));

    // Input should be cleared
    assert_eq!(state.input_buffer(), "");
    let render = state.render_state();
    assert_eq!(render.input.stash, Some("hello world".to_string()));
    assert!(render.input.show_stash_indicator);
}

#[test]
fn ctrl_s_empty_input_does_nothing() {
    let state = create_test_app();

    // Ctrl+S with empty input
    state.handle_key_event(key_event(KeyCode::Char('s'), KeyModifiers::CONTROL));

    // Nothing should change
    assert_eq!(state.input_buffer(), "");
    let render = state.render_state();
    assert_eq!(render.input.stash, None);
    assert!(!render.input.show_stash_indicator);
}

#[test]
fn ctrl_s_restores_stashed_text() {
    let state = create_test_app();

    // Type and stash
    for c in "stashed text".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    state.handle_key_event(key_event(KeyCode::Char('s'), KeyModifiers::CONTROL));

    assert_eq!(state.input_buffer(), "");

    // Ctrl+S again to restore
    state.handle_key_event(key_event(KeyCode::Char('s'), KeyModifiers::CONTROL));

    // Stashed text should be restored
    assert_eq!(state.input_buffer(), "stashed text");
    let render = state.render_state();
    assert_eq!(render.input.stash, None);
    assert!(!render.input.show_stash_indicator);
}

#[test]
fn ctrl_s_raw_char_works() {
    let state = create_test_app();

    // Type some text
    for c in "test".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // Ctrl+S may come as raw ASCII 0x13
    state.handle_key_event(key_event(KeyCode::Char('\x13'), KeyModifiers::NONE));

    assert_eq!(state.input_buffer(), "");
    assert!(state.render_state().input.show_stash_indicator);
}

// ========================
// Meta+T / Alt+T Thinking Dialog Tests
// ========================

#[test]
fn meta_t_opens_thinking_dialog() {
    let state = create_test_app();

    state.handle_key_event(key_event(KeyCode::Char('t'), KeyModifiers::META));

    assert_eq!(state.mode(), AppMode::ThinkingToggle);
}

#[test]
fn alt_t_opens_thinking_dialog() {
    let state = create_test_app();

    // ALT should work same as META
    state.handle_key_event(key_event(KeyCode::Char('t'), KeyModifiers::ALT));

    assert_eq!(state.mode(), AppMode::ThinkingToggle);
}

// ========================
// Meta+P / Alt+P Model Picker Tests
// ========================

#[test]
fn meta_p_opens_model_picker() {
    let state = create_test_app();

    state.handle_key_event(key_event(KeyCode::Char('p'), KeyModifiers::META));

    assert_eq!(state.mode(), AppMode::ModelPicker);
}

#[test]
fn alt_p_opens_model_picker() {
    let state = create_test_app();

    // ALT should work same as META
    state.handle_key_event(key_event(KeyCode::Char('p'), KeyModifiers::ALT));

    assert_eq!(state.mode(), AppMode::ModelPicker);
}
