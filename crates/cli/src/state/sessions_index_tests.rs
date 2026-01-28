// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]
use super::*;

#[test]
fn test_new_sessions_index() {
    let index = SessionsIndex::new();
    assert_eq!(index.version, 1);
    assert!(index.entries.is_empty());
}

#[test]
fn test_add_entry() {
    let mut index = SessionsIndex::new();
    let entry = SessionIndexEntry {
        session_id: "test-session".to_string(),
        full_path: "/path/to/session.jsonl".to_string(),
        file_mtime: 1234567890,
        first_prompt: "Hello".to_string(),
        message_count: 2,
        created: "2025-01-15T10:00:00Z".to_string(),
        modified: "2025-01-15T10:05:00Z".to_string(),
        git_branch: "main".to_string(),
        project_path: "/some/project".to_string(),
        is_sidechain: false,
    };

    index.add_or_update(entry);
    assert_eq!(index.len(), 1);
    assert!(index.get("test-session").is_some());
}

#[test]
fn test_update_entry() {
    let mut index = SessionsIndex::new();
    let entry1 = SessionIndexEntry {
        session_id: "test-session".to_string(),
        full_path: "/path/to/session.jsonl".to_string(),
        file_mtime: 1000,
        first_prompt: "Hello".to_string(),
        message_count: 2,
        created: "2025-01-15T10:00:00Z".to_string(),
        modified: "2025-01-15T10:05:00Z".to_string(),
        git_branch: "main".to_string(),
        project_path: "/some/project".to_string(),
        is_sidechain: false,
    };
    index.add_or_update(entry1);

    // Update with same session ID
    let entry2 = SessionIndexEntry {
        session_id: "test-session".to_string(),
        full_path: "/path/to/session.jsonl".to_string(),
        file_mtime: 2000,
        first_prompt: "Hello".to_string(),
        message_count: 4,
        created: "2025-01-15T10:00:00Z".to_string(),
        modified: "2025-01-15T10:10:00Z".to_string(),
        git_branch: "main".to_string(),
        project_path: "/some/project".to_string(),
        is_sidechain: false,
    };
    index.add_or_update(entry2);

    assert_eq!(index.len(), 1); // Still only one entry
    let entry = index.get("test-session").unwrap();
    assert_eq!(entry.file_mtime, 2000);
    assert_eq!(entry.message_count, 4);
}

#[test]
fn test_save_load() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sessions-index.json");

    let mut index = SessionsIndex::new();
    index.add_or_update(SessionIndexEntry {
        session_id: "session-1".to_string(),
        full_path: "/path/session.jsonl".to_string(),
        file_mtime: 1234567890,
        first_prompt: "Test prompt".to_string(),
        message_count: 2,
        created: "2025-01-15T10:00:00Z".to_string(),
        modified: "2025-01-15T10:05:00Z".to_string(),
        git_branch: "main".to_string(),
        project_path: "/project".to_string(),
        is_sidechain: false,
    });

    index.save(&path).unwrap();

    let loaded = SessionsIndex::load(&path).unwrap();
    assert_eq!(loaded.version, 1);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.get("session-1").unwrap().first_prompt, "Test prompt");
}

#[test]
fn test_serialization_format() {
    let index = SessionsIndex {
        version: 1,
        entries: vec![SessionIndexEntry {
            session_id: "abc123".to_string(),
            full_path: "/path/file.jsonl".to_string(),
            file_mtime: 1000,
            first_prompt: "Hello".to_string(),
            message_count: 2,
            created: "2025-01-15T10:00:00Z".to_string(),
            modified: "2025-01-15T10:05:00Z".to_string(),
            git_branch: "main".to_string(),
            project_path: "/project".to_string(),
            is_sidechain: false,
        }],
    };

    let json = serde_json::to_string(&index).unwrap();

    // Verify camelCase serialization
    assert!(json.contains("sessionId"));
    assert!(json.contains("fullPath"));
    assert!(json.contains("fileMtime"));
    assert!(json.contains("firstPrompt"));
    assert!(json.contains("messageCount"));
    assert!(json.contains("gitBranch"));
    assert!(json.contains("projectPath"));
    assert!(json.contains("isSidechain"));
}
