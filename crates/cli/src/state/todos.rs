// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Todo list state management.

use super::io::JsonLoad;
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
    pub status: TodoStatus,
    /// Display form during execution (e.g., "Building the project").
    pub active_form: String,
}

impl ClaudeTodoItem {
    /// Convert from internal TodoItem to Claude format.
    pub fn from_todo(item: &TodoItem) -> Self {
        Self {
            content: item.content.clone(),
            status: item.status.clone(),
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
        &self.items[self.items.len() - 1]
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
        &self.items[self.items.len() - 1]
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

impl JsonLoad for TodoState {}

#[cfg(test)]
#[path = "todos_tests.rs"]
mod tests;
