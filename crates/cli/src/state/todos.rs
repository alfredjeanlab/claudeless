// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Todo list state management.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Status of a todo item
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

/// A single todo item
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TodoItem {
    /// Unique identifier
    pub id: String,

    /// Todo content/description
    pub content: String,

    /// Active form for display during execution
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,

    /// Current status
    pub status: TodoStatus,

    /// Priority (lower = higher priority)
    #[serde(default)]
    pub priority: u32,
}

/// Todo item in Claude CLI format.
///
/// This is the format used by real Claude CLI in `~/.claude/todos/`.
/// File naming: `{sessionId}-agent-{sessionId}.json`
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeTodoItem {
    /// Todo description.
    pub content: String,
    /// Status: "pending", "in_progress", or "completed".
    pub status: String,
    /// Display form during execution (e.g., "Building the project").
    pub active_form: String,
}

impl ClaudeTodoItem {
    /// Convert from internal TodoItem to Claude format.
    pub fn from_todo(item: &TodoItem) -> Self {
        Self {
            content: item.content.clone(),
            status: match item.status {
                TodoStatus::Pending => "pending",
                TodoStatus::InProgress => "in_progress",
                TodoStatus::Completed => "completed",
            }
            .to_string(),
            active_form: item
                .active_form
                .clone()
                .unwrap_or_else(|| format!("{}...", &item.content)),
        }
    }
}

/// Todo list state
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TodoState {
    /// List of todo items
    pub items: Vec<TodoItem>,
}

impl TodoState {
    /// Create empty todo state
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from file
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save to file
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    /// Save in Claude CLI format.
    ///
    /// Claude CLI stores todos as a JSON array of objects with:
    /// - `content`: string (todo description)
    /// - `status`: string ("pending", "in_progress", "completed")
    /// - `activeForm`: string (display form during execution)
    pub fn save_claude_format(&self, path: &Path) -> std::io::Result<()> {
        let items: Vec<ClaudeTodoItem> = self.items.iter().map(ClaudeTodoItem::from_todo).collect();
        let json = serde_json::to_string_pretty(&items)?;
        std::fs::write(path, json)
    }

    /// Add a todo item
    pub fn add(&mut self, content: impl Into<String>) -> &TodoItem {
        let id = format!("todo_{}", self.items.len());
        let item = TodoItem {
            id,
            content: content.into(),
            active_form: None,
            status: TodoStatus::Pending,
            priority: self.items.len() as u32,
        };
        self.items.push(item);
        self.items.last().unwrap()
    }

    /// Add a todo item with active form
    pub fn add_with_active_form(
        &mut self,
        content: impl Into<String>,
        active_form: impl Into<String>,
    ) -> &TodoItem {
        let id = format!("todo_{}", self.items.len());
        let item = TodoItem {
            id,
            content: content.into(),
            active_form: Some(active_form.into()),
            status: TodoStatus::Pending,
            priority: self.items.len() as u32,
        };
        self.items.push(item);
        self.items.last().unwrap()
    }

    /// Get item by ID
    pub fn get(&self, id: &str) -> Option<&TodoItem> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Get mutable item by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut TodoItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    /// Update item status
    pub fn set_status(&mut self, id: &str, status: TodoStatus) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = status;
            true
        } else {
            false
        }
    }

    /// Get pending items
    pub fn pending(&self) -> impl Iterator<Item = &TodoItem> {
        self.items
            .iter()
            .filter(|i| i.status == TodoStatus::Pending)
    }

    /// Get in-progress items
    pub fn in_progress(&self) -> impl Iterator<Item = &TodoItem> {
        self.items
            .iter()
            .filter(|i| i.status == TodoStatus::InProgress)
    }

    /// Get completed items
    pub fn completed(&self) -> impl Iterator<Item = &TodoItem> {
        self.items
            .iter()
            .filter(|i| i.status == TodoStatus::Completed)
    }

    /// Remove item by ID
    pub fn remove(&mut self, id: &str) -> Option<TodoItem> {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            Some(self.items.remove(pos))
        } else {
            None
        }
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get item count
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

#[cfg(test)]
mod tests {
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
}
