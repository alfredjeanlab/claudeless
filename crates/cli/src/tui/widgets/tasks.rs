// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tasks dialog widget.
//!
//! Shown when user executes `/tasks` to view background tasks.
//!
//! Note: Rendering has been moved to app.rs as part of the iocraft migration.
//! This module keeps the types used by the app component.

#[cfg(test)]
#[path = "tasks_tests.rs"]
mod tests;

use super::scrollable::ScrollState;

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
#[derive(Clone, Debug)]
pub struct TasksDialog {
    /// List of background tasks
    pub tasks: Vec<TaskInfo>,
    /// Scroll-aware navigation state
    scroll: ScrollState,
}

impl Default for TasksDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl TasksDialog {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            scroll: ScrollState::new(5), // Default visible count
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Get the currently selected task index
    pub fn selected_index(&self) -> usize {
        self.scroll.selected_index
    }

    /// Get the scroll offset for rendering
    pub fn scroll_offset(&self) -> usize {
        self.scroll.scroll_offset
    }

    /// Set the visible item count (call when terminal resizes)
    pub fn set_visible_count(&mut self, count: usize) {
        self.scroll.visible_count = count;
    }

    /// Update tasks list
    pub fn set_tasks(&mut self, tasks: Vec<TaskInfo>) {
        self.tasks = tasks;
        self.scroll.set_total(self.tasks.len());
    }

    /// Move selection up (wraps at top to bottom)
    pub fn move_selection_up(&mut self) {
        self.scroll.select_prev();
    }

    /// Move selection down (wraps at bottom to top)
    pub fn move_selection_down(&mut self) {
        self.scroll.select_next();
    }

    /// Check if there are items above the visible area
    pub fn has_more_above(&self) -> bool {
        self.scroll.has_more_above()
    }

    /// Check if there are items below the visible area
    pub fn has_more_below(&self) -> bool {
        self.scroll.has_more_below()
    }

    /// Get currently selected task
    pub fn selected_task(&self) -> Option<&TaskInfo> {
        self.tasks.get(self.scroll.selected_index)
    }
}
