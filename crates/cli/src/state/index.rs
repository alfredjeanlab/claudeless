// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Sessions index management for project directories.
//!
//! The sessions-index.json file tracks all sessions in a project directory.
//! Format matches real Claude CLI (v2.1.12).

use super::io::JsonLoad;
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

impl JsonLoad for SessionsIndex {}

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
#[path = "index_tests.rs"]
mod tests;
