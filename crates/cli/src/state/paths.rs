// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Path computation and normalization utilities for state directories.
//!
//! This module provides pure functions for computing paths within the state directory
//! structure. These functions are deterministic and have no side effects.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Normalize a project path to match Claude CLI's directory naming convention.
///
/// Real Claude CLI converts paths like `/Users/user/Developer/myproject` to
/// `-Users-user-Developer-myproject` for the projects directory.
///
/// The normalization rules are:
/// 1. Replace all `/` characters with `-`
/// 2. Replace all `.` characters with `-`
/// 3. This results in a leading `-` for absolute paths
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use claudeless::state::paths::normalize_project_path;
///
/// assert_eq!(
///     normalize_project_path(Path::new("/Users/user/Developer/myproject")),
///     "-Users-user-Developer-myproject"
/// );
///
/// assert_eq!(
///     normalize_project_path(Path::new("/tmp/test.txt")),
///     "-tmp-test-txt"
/// );
/// ```
pub fn normalize_project_path(path: &Path) -> String {
    path.to_string_lossy().replace(['/', '.'], "-")
}

/// Get the canonical project directory name for a path.
///
/// This tries to canonicalize the path first (resolving symlinks, etc.)
/// and falls back to the original path if canonicalization fails.
pub fn project_dir_name(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    normalize_project_path(&canonical)
}

/// Generate a deterministic hash for a project path (deprecated, use normalize_project_path)
#[deprecated(since = "0.1.0", note = "Use normalize_project_path instead")]
pub fn project_hash(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8]) // First 8 bytes = 16 hex chars
}

/// Path computation helper for state directory structure.
///
/// This struct provides methods for computing paths within the state directory
/// without actually performing any I/O operations.
#[derive(Debug, Clone)]
pub struct StatePaths {
    root: PathBuf,
}

impl StatePaths {
    /// Create a new path computer rooted at the given directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Get the root directory path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the todos directory path.
    pub fn todos_dir(&self) -> PathBuf {
        self.root.join("todos")
    }

    /// Get the projects directory path.
    pub fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    /// Get the plans directory path.
    pub fn plans_dir(&self) -> PathBuf {
        self.root.join("plans")
    }

    /// Get the sessions directory path.
    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    /// Get the settings file path.
    pub fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    /// Get the project directory for a given project path.
    ///
    /// Uses the same path normalization as the real Claude CLI:
    /// `/Users/foo/project` → `~/.claude/projects/-Users-foo-project`
    pub fn project_dir(&self, project_path: &Path) -> PathBuf {
        let dir_name = project_dir_name(project_path);
        self.projects_dir().join(&dir_name)
    }

    /// Get the session file path for a given session ID.
    pub fn session_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir().join(format!("{}.json", session_id))
    }

    /// Get the todo file path for a given context.
    pub fn todo_path(&self, context: &str) -> PathBuf {
        self.todos_dir().join(format!("{}.json", context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_project_path() {
        assert_eq!(
            normalize_project_path(Path::new("/Users/user/project")),
            "-Users-user-project"
        );
        assert_eq!(
            normalize_project_path(Path::new("/tmp/test.txt")),
            "-tmp-test-txt"
        );
        assert_eq!(
            normalize_project_path(Path::new("relative/path")),
            "relative-path"
        );
    }

    #[test]
    fn test_state_paths() {
        let paths = StatePaths::new("/home/user/.claude");

        assert_eq!(paths.root(), Path::new("/home/user/.claude"));
        assert_eq!(paths.todos_dir(), PathBuf::from("/home/user/.claude/todos"));
        assert_eq!(
            paths.projects_dir(),
            PathBuf::from("/home/user/.claude/projects")
        );
        assert_eq!(paths.plans_dir(), PathBuf::from("/home/user/.claude/plans"));
        assert_eq!(
            paths.sessions_dir(),
            PathBuf::from("/home/user/.claude/sessions")
        );
        assert_eq!(
            paths.settings_path(),
            PathBuf::from("/home/user/.claude/settings.json")
        );
    }

    #[test]
    fn test_state_paths_session() {
        let paths = StatePaths::new("/tmp/state");
        assert_eq!(
            paths.session_path("abc-123"),
            PathBuf::from("/tmp/state/sessions/abc-123.json")
        );
    }

    #[test]
    fn test_state_paths_todo() {
        let paths = StatePaths::new("/tmp/state");
        assert_eq!(
            paths.todo_path("context1"),
            PathBuf::from("/tmp/state/todos/context1.json")
        );
    }
}
