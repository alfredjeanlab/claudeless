// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Path computation and normalization utilities for state directories.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Normalize a project path to match Claude CLI's directory naming convention.
///
/// Real Claude CLI converts paths like `/Users/user/Developer/myproject` to
/// `-Users-user-Developer-myproject` for the projects directory.
pub fn normalize_project_path(path: &Path) -> String {
    path.to_string_lossy().replace(['/', '.'], "-")
}

/// Get the canonical project directory name for a path.
///
/// Tries to canonicalize the path first (resolving symlinks, etc.)
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
    hex::encode(&result[..8])
}

// Free functions for path computation

pub fn todos_dir(root: &Path) -> PathBuf {
    root.join("todos")
}

pub fn projects_dir(root: &Path) -> PathBuf {
    root.join("projects")
}

pub fn plans_dir(root: &Path) -> PathBuf {
    root.join("plans")
}

pub fn sessions_dir(root: &Path) -> PathBuf {
    root.join("sessions")
}

pub fn settings_path(root: &Path) -> PathBuf {
    root.join("settings.json")
}

pub fn project_dir(root: &Path, project_path: &Path) -> PathBuf {
    let dir_name = project_dir_name(project_path);
    projects_dir(root).join(&dir_name)
}

pub fn session_path(root: &Path, session_id: &str) -> PathBuf {
    sessions_dir(root).join(format!("{}.json", session_id))
}

pub fn todo_path(root: &Path, context: &str) -> PathBuf {
    todos_dir(root).join(format!("{}.json", context))
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
