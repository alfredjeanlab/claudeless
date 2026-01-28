// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn test_new_session() {
    let session = Session::new("test_session");
    assert_eq!(session.id, "test_session");
    assert!(session.turns.is_empty());
    assert!(session.project_path.is_none());
}

#[test]
fn test_session_with_project() {
    let session = Session::new("test").with_project("/some/path");
    assert_eq!(session.project_path, Some("/some/path".to_string()));
}

#[test]
fn test_add_turn() {
    let mut session = Session::new("test");
    session.add_turn("Hello".to_string(), "Hi there!".to_string());

    assert_eq!(session.turn_count(), 1);
    let turn = session.last_turn().unwrap();
    assert_eq!(turn.prompt, "Hello");
    assert_eq!(turn.response, "Hi there!");
    assert_eq!(turn.seq, 0);
}

#[test]
fn test_multiple_turns() {
    let mut session = Session::new("test");
    session.add_turn("First".to_string(), "Response 1".to_string());
    session.add_turn("Second".to_string(), "Response 2".to_string());

    assert_eq!(session.turn_count(), 2);
    assert_eq!(session.turns[0].seq, 0);
    assert_eq!(session.turns[1].seq, 1);
}

#[test]
fn test_session_expiration() {
    let session = Session::new_at("test", 0);
    // Session created at epoch, check if expired after 1 hour
    assert!(session.is_expired_at(Duration::from_secs(3600), 3600001));
    assert!(!session.is_expired_at(Duration::from_secs(3600), 1000));
}

#[test]
fn test_session_save_load() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("session.json");

    let mut session = Session::new("test_session");
    session.add_turn("Hello".to_string(), "Hi!".to_string());
    session.save(&path).unwrap();

    let loaded = Session::load(&path).unwrap();
    assert_eq!(loaded.id, "test_session");
    assert_eq!(loaded.turn_count(), 1);
}

#[test]
fn test_session_manager_create() {
    let mut manager = SessionManager::new();
    let session = manager.create_session();

    assert!(session.id.starts_with("session_"));
    assert_eq!(manager.len(), 1);
}

#[test]
fn test_session_manager_create_with_id() {
    let mut manager = SessionManager::new();
    manager.create_session_with_id("my_session");

    assert_eq!(manager.current_id(), Some("my_session"));
    assert!(manager.get("my_session").is_some());
}

#[test]
fn test_session_manager_current() {
    let mut manager = SessionManager::new();

    // Auto-creates session
    let session1 = manager.current_session();
    let id = session1.id.clone();

    // Returns same session
    let session2 = manager.current_session();
    assert_eq!(session2.id, id);
}

#[test]
fn test_session_manager_resume() {
    let mut manager = SessionManager::new();
    manager.create_session_with_id("session_a");
    manager.create_session_with_id("session_b");

    assert_eq!(manager.current_id(), Some("session_b"));

    manager.resume("session_a");
    assert_eq!(manager.current_id(), Some("session_a"));
}

#[test]
fn test_session_manager_continue() {
    let mut manager = SessionManager::new();

    // Create sessions with different timestamps
    let session1 = manager.create_session_with_id("old");
    session1.last_active_ms = 1000;

    let session2 = manager.create_session_with_id("new");
    session2.last_active_ms = 2000;

    // Switch to old
    manager.resume("old");
    assert_eq!(manager.current_id(), Some("old"));

    // Continue should pick newest
    manager.continue_session();
    assert_eq!(manager.current_id(), Some("new"));
}

#[test]
fn test_session_manager_persistence() {
    let temp = tempfile::tempdir().unwrap();

    // Create and save session
    {
        let mut manager = SessionManager::new().with_storage(temp.path());
        let session = manager.create_session_with_id("persistent");
        session.add_turn("Hello".to_string(), "Hi!".to_string());
        manager.save_current().unwrap();
    }

    // Load in new manager
    {
        let mut manager = SessionManager::new().with_storage(temp.path());
        let session = manager.resume("persistent").unwrap();
        assert_eq!(session.turn_count(), 1);
    }
}

#[test]
fn test_session_manager_clear() {
    let mut manager = SessionManager::new();
    manager.create_session_with_id("session_1");
    manager.create_session_with_id("session_2");

    assert_eq!(manager.len(), 2);
    manager.clear();
    assert!(manager.is_empty());
    assert!(manager.current_id().is_none());
}

#[test]
fn test_turn_tool_calls() {
    let mut session = Session::new("test");
    let turn = session.add_turn_at("Hello".to_string(), "Hi!".to_string(), 1000);

    turn.tool_calls.push(TurnToolCall {
        tool: "Bash".to_string(),
        input: serde_json::json!({"command": "ls"}),
        output: Some("file1\nfile2".to_string()),
    });

    assert_eq!(session.last_turn().unwrap().tool_calls.len(), 1);
}
