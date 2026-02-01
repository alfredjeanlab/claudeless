// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::types::ExitHint;
use super::*;
use crate::state::session::SessionManager;
use crate::time::ClockHandle;

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

// =============================================================================
// Slash Menu Tests
// =============================================================================

#[test]
fn test_typing_slash_opens_menu() {
    let state = create_test_app();

    // Type /
    state.handle_key_event(key_event(KeyCode::Char('/'), KeyModifiers::empty()));

    let render = state.render_state();
    assert_eq!(render.input.buffer, "/");
    assert!(render.display.slash_menu.is_some());

    let menu = render.display.slash_menu.unwrap();
    assert!(!menu.filtered_commands.is_empty());
    assert_eq!(menu.selected_index, 0);
}

#[test]
fn test_typing_characters_filters_menu() {
    let state = create_test_app();

    // Type /co
    state.handle_key_event(key_event(KeyCode::Char('/'), KeyModifiers::empty()));
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::empty()));
    state.handle_key_event(key_event(KeyCode::Char('o'), KeyModifiers::empty()));

    let render = state.render_state();
    assert_eq!(render.input.buffer, "/co");
    assert!(render.display.slash_menu.is_some());

    let menu = render.display.slash_menu.unwrap();
    // Should filter to commands matching "co" - compact, config, context, etc.
    let names: Vec<_> = menu.filtered_commands.iter().map(|c| c.name).collect();
    assert!(names.contains(&"compact"));
    assert!(names.contains(&"config"));
    assert!(names.contains(&"context"));
}

#[test]
fn test_down_arrow_moves_selection() {
    let state = create_test_app();

    // Type /
    state.handle_key_event(key_event(KeyCode::Char('/'), KeyModifiers::empty()));

    let render = state.render_state();
    let initial_selection = render.display.slash_menu.as_ref().unwrap().selected_index;
    assert_eq!(initial_selection, 0);

    // Press Down
    state.handle_key_event(key_event(KeyCode::Down, KeyModifiers::empty()));

    let render = state.render_state();
    let new_selection = render.display.slash_menu.as_ref().unwrap().selected_index;
    assert_eq!(new_selection, 1);
}

#[test]
fn test_up_arrow_moves_selection() {
    let state = create_test_app();

    // Type /
    state.handle_key_event(key_event(KeyCode::Char('/'), KeyModifiers::empty()));

    // Press Down twice
    state.handle_key_event(key_event(KeyCode::Down, KeyModifiers::empty()));
    state.handle_key_event(key_event(KeyCode::Down, KeyModifiers::empty()));

    let render = state.render_state();
    assert_eq!(
        render.display.slash_menu.as_ref().unwrap().selected_index,
        2
    );

    // Press Up
    state.handle_key_event(key_event(KeyCode::Up, KeyModifiers::empty()));

    let render = state.render_state();
    assert_eq!(
        render.display.slash_menu.as_ref().unwrap().selected_index,
        1
    );
}

#[test]
fn test_tab_completes_selected_command() {
    let state = create_test_app();

    // Type /
    state.handle_key_event(key_event(KeyCode::Char('/'), KeyModifiers::empty()));

    // Tab to complete first command (should be /add-dir)
    state.handle_key_event(key_event(KeyCode::Tab, KeyModifiers::empty()));

    let render = state.render_state();
    // Menu should be closed
    assert!(render.display.slash_menu.is_none());
    // Input should be completed command
    assert_eq!(render.input.buffer, "/add-dir");
}

#[test]
fn test_tab_completes_after_navigation() {
    let state = create_test_app();

    // Type /
    state.handle_key_event(key_event(KeyCode::Char('/'), KeyModifiers::empty()));

    // Press Down to select second command
    state.handle_key_event(key_event(KeyCode::Down, KeyModifiers::empty()));

    // Tab to complete
    state.handle_key_event(key_event(KeyCode::Tab, KeyModifiers::empty()));

    let render = state.render_state();
    // Should complete to second command (agents)
    assert_eq!(render.input.buffer, "/agents");
}

#[test]
fn test_escape_closes_menu_keeps_text() {
    let state = create_test_app();

    // Type /
    state.handle_key_event(key_event(KeyCode::Char('/'), KeyModifiers::empty()));

    // Verify menu is open
    let render = state.render_state();
    assert!(render.display.slash_menu.is_some());

    // Press Escape
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));

    let render = state.render_state();
    // Menu should be closed
    assert!(render.display.slash_menu.is_none());
    // Input should still have /
    assert_eq!(render.input.buffer, "/");
    // Should show escape hint
    assert_eq!(render.display.exit_hint, Some(ExitHint::Escape));
}

#[test]
fn test_backspace_updates_filter() {
    let state = create_test_app();

    // Type /co
    state.handle_key_event(key_event(KeyCode::Char('/'), KeyModifiers::empty()));
    state.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::empty()));
    state.handle_key_event(key_event(KeyCode::Char('o'), KeyModifiers::empty()));

    let render = state.render_state();
    let initial_count = render
        .display
        .slash_menu
        .as_ref()
        .unwrap()
        .filtered_commands
        .len();

    // Backspace to /c
    state.handle_key_event(key_event(KeyCode::Backspace, KeyModifiers::empty()));

    let render = state.render_state();
    assert_eq!(render.input.buffer, "/c");
    // Should have more commands now (less restrictive filter)
    let new_count = render
        .display
        .slash_menu
        .as_ref()
        .unwrap()
        .filtered_commands
        .len();
    assert!(new_count >= initial_count);
}

#[test]
fn test_deleting_slash_closes_menu() {
    let state = create_test_app();

    // Type /
    state.handle_key_event(key_event(KeyCode::Char('/'), KeyModifiers::empty()));

    // Verify menu is open
    let render = state.render_state();
    assert!(render.display.slash_menu.is_some());

    // Backspace to remove /
    state.handle_key_event(key_event(KeyCode::Backspace, KeyModifiers::empty()));

    let render = state.render_state();
    // Menu should be closed (no / in input)
    assert!(render.display.slash_menu.is_none());
    assert_eq!(render.input.buffer, "");
}
