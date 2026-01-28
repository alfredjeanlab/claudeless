// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::render::{format_header_lines, format_status_bar};
use super::types::ExitHint;
use super::*;
use crate::ansi::strip_ansi;
use crate::config::ScenarioConfig;
use crate::permission::PermissionMode;
use crate::scenario::Scenario;
use crate::state::session::SessionManager;
use crate::time::ClockHandle;
use crate::tui::widgets::permission::PermissionSelection;

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

// Version display tests

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
// Session Permission Grant Tests
// =========================================================================

#[test]
fn test_session_grant_not_stored_for_single_yes() {
    let state = create_test_app();
    state.show_bash_permission("cat /etc/passwd".to_string(), None);

    // Select "Yes" (single grant)
    {
        let mut inner = state.inner.lock();
        inner.dialog.as_permission_mut().unwrap().dialog.selected = PermissionSelection::Yes;
    }
    state.confirm_permission();

    // Verify no session grant stored
    let inner = state.inner.lock();
    assert!(inner.session_grants.is_empty());
}

#[test]
fn test_session_grant_stored_for_yes_session() {
    let state = create_test_app();
    state.show_bash_permission("cat /etc/passwd".to_string(), None);

    // Select "Yes, allow for session"
    {
        let mut inner = state.inner.lock();
        inner.dialog.as_permission_mut().unwrap().dialog.selected = PermissionSelection::YesSession;
    }
    state.confirm_permission();

    // Verify session grant stored
    let inner = state.inner.lock();
    assert!(!inner.session_grants.is_empty());
}

#[test]
fn test_session_grant_auto_approves_subsequent_request() {
    let state = create_test_app();

    // First request: grant for session
    state.show_bash_permission("cat /etc/passwd".to_string(), None);
    {
        let mut inner = state.inner.lock();
        inner.dialog.as_permission_mut().unwrap().dialog.selected = PermissionSelection::YesSession;
    }
    state.confirm_permission();

    // Clear response content
    {
        let mut inner = state.inner.lock();
        inner.display.response_content.clear();
    }

    // Second request with same prefix: should auto-approve (no pending permission)
    state.show_bash_permission("cat /etc/hosts".to_string(), None);

    let inner = state.inner.lock();
    assert!(inner.dialog.as_permission().is_none()); // Auto-approved, no dialog
    assert!(inner.display.response_content.contains("auto-granted"));
}

#[test]
fn test_session_grant_different_prefix_not_auto_approved() {
    let state = create_test_app();

    // First request: grant for session for cat /etc/
    state.show_bash_permission("cat /etc/passwd".to_string(), None);
    {
        let mut inner = state.inner.lock();
        inner.dialog.as_permission_mut().unwrap().dialog.selected = PermissionSelection::YesSession;
    }
    state.confirm_permission();

    // Second request with different prefix: should NOT auto-approve
    state.show_bash_permission("cat /var/log/syslog".to_string(), None);

    let inner = state.inner.lock();
    assert!(inner.dialog.as_permission().is_some()); // Dialog shown, not auto-approved
    assert_eq!(inner.mode, AppMode::Permission);
}

#[test]
fn test_clear_command_clears_session_grants() {
    let state = create_test_app();

    // Grant session permission
    state.show_bash_permission("cat /etc/passwd".to_string(), None);
    {
        let mut inner = state.inner.lock();
        inner.dialog.as_permission_mut().unwrap().dialog.selected = PermissionSelection::YesSession;
    }
    state.confirm_permission();

    // Verify grant stored
    {
        let inner = state.inner.lock();
        assert!(!inner.session_grants.is_empty());
    }

    // Run /clear
    {
        let mut inner = state.inner.lock();
        inner.input.buffer = "/clear".to_string();
    }
    state.handle_key_event(key_event(KeyCode::Enter, KeyModifiers::empty()));

    // Verify grants cleared
    let inner = state.inner.lock();
    assert!(inner.session_grants.is_empty());
}

#[test]
fn test_no_grant_stored_for_denied_permission() {
    let state = create_test_app();
    state.show_bash_permission("cat /etc/passwd".to_string(), None);

    // Select "No"
    {
        let mut inner = state.inner.lock();
        inner.dialog.as_permission_mut().unwrap().dialog.selected = PermissionSelection::No;
    }
    state.confirm_permission();

    // Verify no session grant stored
    let inner = state.inner.lock();
    assert!(inner.session_grants.is_empty());
}

#[test]
fn test_edit_session_grant_applies_to_all_edits() {
    let state = create_test_app();

    // Grant session permission for edit
    state.show_edit_permission("foo.txt".to_string(), vec![]);
    {
        let mut inner = state.inner.lock();
        inner.dialog.as_permission_mut().unwrap().dialog.selected = PermissionSelection::YesSession;
    }
    state.confirm_permission();

    // Clear response content
    {
        let mut inner = state.inner.lock();
        inner.display.response_content.clear();
    }

    // Second edit request for different file: should auto-approve
    state.show_edit_permission("bar.txt".to_string(), vec![]);

    let inner = state.inner.lock();
    assert!(inner.dialog.as_permission().is_none()); // Auto-approved
    assert!(inner.display.response_content.contains("auto-granted"));
}

#[test]
fn test_write_session_grant_applies_to_all_writes() {
    let state = create_test_app();

    // Grant session permission for write
    state.show_write_permission("foo.txt".to_string(), vec![]);
    {
        let mut inner = state.inner.lock();
        inner.dialog.as_permission_mut().unwrap().dialog.selected = PermissionSelection::YesSession;
    }
    state.confirm_permission();

    // Clear response content
    {
        let mut inner = state.inner.lock();
        inner.display.response_content.clear();
    }

    // Second write request for different file: should auto-approve
    state.show_write_permission("bar.txt".to_string(), vec![]);

    let inner = state.inner.lock();
    assert!(inner.dialog.as_permission().is_none()); // Auto-approved
    assert!(inner.display.response_content.contains("auto-granted"));
}

#[test]
fn test_different_permission_types_tracked_independently() {
    let state = create_test_app();

    // Grant session permission for edit
    state.show_edit_permission("foo.txt".to_string(), vec![]);
    {
        let mut inner = state.inner.lock();
        inner.dialog.as_permission_mut().unwrap().dialog.selected = PermissionSelection::YesSession;
    }
    state.confirm_permission();

    // Write should NOT be auto-approved (different permission type)
    state.show_write_permission("foo.txt".to_string(), vec![]);

    let inner = state.inner.lock();
    assert!(inner.dialog.as_permission().is_some()); // Dialog shown
    assert_eq!(inner.mode, AppMode::Permission);
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

// ============================================================================
// Escape to clear input
// ============================================================================

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
