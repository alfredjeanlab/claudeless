// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_new_todo_state() {
    let state = TodoState::new();
    assert!(state.is_empty());
    assert_eq!(state.len(), 0);
}

#[test]
fn test_add_todo() {
    let mut state = TodoState::new();
    let item = state.add("Test task");

    assert_eq!(item.content, "Test task");
    assert_eq!(item.status, TodoStatus::Pending);
    assert_eq!(state.len(), 1);
}

#[test]
fn test_add_with_active_form() {
    let mut state = TodoState::new();
    let item = state.add_with_active_form("Run tests", "Running tests");

    assert_eq!(item.content, "Run tests");
    assert_eq!(item.active_form, Some("Running tests".to_string()));
}

#[test]
fn test_set_status() {
    let mut state = TodoState::new();
    state.add("Task 1");
    state.add("Task 2");

    assert!(state.set_status("todo_0", TodoStatus::InProgress));
    assert!(state.set_status("todo_1", TodoStatus::Completed));
    assert!(!state.set_status("nonexistent", TodoStatus::Completed));

    assert_eq!(state.get("todo_0").unwrap().status, TodoStatus::InProgress);
    assert_eq!(state.get("todo_1").unwrap().status, TodoStatus::Completed);
}

#[test]
fn test_filters() {
    let mut state = TodoState::new();
    state.add("Pending 1");
    state.add("Pending 2");
    state.add("In progress");
    state.add("Completed");

    state.set_status("todo_2", TodoStatus::InProgress);
    state.set_status("todo_3", TodoStatus::Completed);

    assert_eq!(state.pending().count(), 2);
    assert_eq!(state.in_progress().count(), 1);
    assert_eq!(state.completed().count(), 1);
}

#[test]
fn test_remove() {
    let mut state = TodoState::new();
    state.add("Task 1");
    state.add("Task 2");

    let removed = state.remove("todo_0");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().content, "Task 1");
    assert_eq!(state.len(), 1);
    assert!(state.remove("todo_0").is_none());
}

#[test]
fn test_clear() {
    let mut state = TodoState::new();
    state.add("Task 1");
    state.add("Task 2");

    state.clear();
    assert!(state.is_empty());
}

#[test]
fn test_save_load() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("todos.json");

    let mut state = TodoState::new();
    state.add("Task 1");
    state.add("Task 2");
    state.set_status("todo_1", TodoStatus::Completed);

    state.save(&path).unwrap();

    let loaded = TodoState::load(&path).unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded.get("todo_1").unwrap().status, TodoStatus::Completed);
}

#[test]
fn test_serialization() {
    let mut state = TodoState::new();
    state.add_with_active_form("Build project", "Building project");

    let json = serde_json::to_string(&state).unwrap();
    let loaded: TodoState = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.items[0].content, "Build project");
    assert_eq!(
        loaded.items[0].active_form,
        Some("Building project".to_string())
    );
}
