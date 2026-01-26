// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

fn make_task(id: &str, desc: &str, status: TaskStatus) -> TaskInfo {
    TaskInfo {
        id: id.to_string(),
        description: desc.to_string(),
        status,
    }
}

#[test]
fn new_dialog_is_empty() {
    let dialog = TasksDialog::new();
    assert!(dialog.is_empty());
    assert_eq!(dialog.selected_index(), 0);
}

#[test]
fn set_tasks_updates_list() {
    let mut dialog = TasksDialog::new();
    let tasks = vec![
        make_task("1", "Task 1", TaskStatus::Running),
        make_task("2", "Task 2", TaskStatus::Completed),
    ];
    dialog.set_tasks(tasks);
    assert!(!dialog.is_empty());
    assert_eq!(dialog.tasks.len(), 2);
}

#[test]
fn move_selection_down_wraps_at_bottom() {
    let mut dialog = TasksDialog::new();
    dialog.set_tasks(vec![
        make_task("1", "Task 1", TaskStatus::Running),
        make_task("2", "Task 2", TaskStatus::Completed),
        make_task("3", "Task 3", TaskStatus::Failed),
    ]);

    // Move to last item
    dialog.move_selection_down();
    dialog.move_selection_down();
    assert_eq!(dialog.selected_index(), 2);

    // Should wrap to top
    dialog.move_selection_down();
    assert_eq!(dialog.selected_index(), 0);
}

#[test]
fn move_selection_up_wraps_at_top() {
    let mut dialog = TasksDialog::new();
    dialog.set_tasks(vec![
        make_task("1", "Task 1", TaskStatus::Running),
        make_task("2", "Task 2", TaskStatus::Completed),
        make_task("3", "Task 3", TaskStatus::Failed),
    ]);

    // Already at top
    assert_eq!(dialog.selected_index(), 0);

    // Should wrap to bottom
    dialog.move_selection_up();
    assert_eq!(dialog.selected_index(), 2);
}

#[test]
fn selected_task_returns_correct_task() {
    let mut dialog = TasksDialog::new();
    dialog.set_tasks(vec![
        make_task("1", "Task 1", TaskStatus::Running),
        make_task("2", "Task 2", TaskStatus::Completed),
    ]);

    let task = dialog.selected_task().unwrap();
    assert_eq!(task.id, "1");

    dialog.move_selection_down();
    let task = dialog.selected_task().unwrap();
    assert_eq!(task.id, "2");
}

#[test]
fn selected_task_returns_none_when_empty() {
    let dialog = TasksDialog::new();
    assert!(dialog.selected_task().is_none());
}

#[test]
fn empty_list_navigation_is_noop() {
    let mut dialog = TasksDialog::new();
    dialog.move_selection_down();
    assert_eq!(dialog.selected_index(), 0);
    dialog.move_selection_up();
    assert_eq!(dialog.selected_index(), 0);
}
