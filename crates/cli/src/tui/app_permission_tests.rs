// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use crate::config::ScenarioConfig;
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
