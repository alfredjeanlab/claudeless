// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Sessions index management for project directories.
//!
//! The sessions-index.json file tracks all sessions in a project directory.
//! Format matches real Claude CLI (v2.1.12).

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Entry in sessions-index.json.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionIndexEntry {
    /// Session UUID.
    pub session_id: String,
    /// Full path to session JSONL file.
    pub full_path: String,
    /// File modification time (millis since epoch).
    pub file_mtime: u64,
    /// First prompt text in the session.
    pub first_prompt: String,
    /// Number of messages in the session.
    pub message_count: u32,
    /// Session creation time (ISO 8601).
    pub created: String,
    /// Last modification time (ISO 8601).
    pub modified: String,
    /// Git branch name (empty string if not in a git repo).
    pub git_branch: String,
    /// Project path.
    pub project_path: String,
    /// Whether this is a sidechain session.
    pub is_sidechain: bool,
}

/// sessions-index.json structure.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionsIndex {
    /// Format version (always 1).
    pub version: u32,
    /// Session entries.
    pub entries: Vec<SessionIndexEntry>,
}

impl Default for SessionsIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionsIndex {
    /// Create a new empty sessions index.
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: vec![],
        }
    }

    /// Load sessions index from file.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save sessions index to file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    /// Add or update an entry in the index.
    pub fn add_or_update(&mut self, entry: SessionIndexEntry) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|e| e.session_id == entry.session_id)
        {
            // Update existing entry
            existing.file_mtime = entry.file_mtime;
            existing.message_count = entry.message_count;
            existing.modified = entry.modified;
        } else {
            // Add new entry
            self.entries.push(entry);
        }
    }

    /// Get an entry by session ID.
    pub fn get(&self, session_id: &str) -> Option<&SessionIndexEntry> {
        self.entries.iter().find(|e| e.session_id == session_id)
    }

    /// Get number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Get the current git branch name, or empty string if not in a git repo.
pub fn get_git_branch() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
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
}
