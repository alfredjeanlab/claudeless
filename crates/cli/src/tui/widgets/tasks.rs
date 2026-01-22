// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tasks dialog widget.
//!
//! Shown when user executes `/tasks` to view background tasks.
//!
//! Note: Rendering has been moved to app.rs as part of the iocraft migration.
//! This module keeps the types used by the app component.

/// Background task info for display
#[derive(Clone, Debug)]
pub struct TaskInfo {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TaskStatus {
    Running,
    Completed,
    Failed,
}

/// State for the /tasks dialog
#[derive(Clone, Debug, Default)]
pub struct TasksDialog {
    /// List of background tasks
    pub tasks: Vec<TaskInfo>,
    /// Currently selected task index
    pub selected_index: usize,
}

impl TasksDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if !self.tasks.is_empty() && self.selected_index < self.tasks.len() - 1 {
            self.selected_index += 1;
        }
    }
}
