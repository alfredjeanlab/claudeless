// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Simulated `~/.claude` directory structure.
//!
//! This module provides the `StateDirectory` type, which manages the directory
//! structure and I/O operations. For pure path computation without I/O, see
//! the [`paths`](super::paths) module.

use super::io::files_in;
use super::paths::StatePaths;
use std::fs::{self, Permissions};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("State directory not initialized")]
    NotInitialized,

    #[error("Invalid project path: {0}")]
    InvalidProject(String),
}

/// Simulated ~/.claude directory structure.
///
/// This struct combines path computation (via [`StatePaths`]) with directory
/// lifecycle management and I/O operations.
#[derive(Debug)]
pub struct StateDirectory {
    /// Path computation helper
    paths: StatePaths,

    /// Whether the directory has been initialized
    initialized: bool,
}

impl StateDirectory {
    /// Create a new state directory at the given root.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            paths: StatePaths::new(root),
            initialized: false,
        }
    }

    /// Create a state directory in a temporary location.
    pub fn temp() -> std::io::Result<Self> {
        let temp = tempfile::tempdir()?;
        let path = temp.keep();
        Ok(Self {
            paths: StatePaths::new(path),
            initialized: false,
        })
    }

    /// Resolve state directory from environment or default to a temp directory.
    ///
    /// # Priority
    ///
    /// 1. `CLAUDELESS_CONFIG_DIR` - Claudeless-specific override (highest priority)
    /// 2. `CLAUDELESS_STATE_DIR` - Legacy claudeless override (backwards compatibility)
    /// 3. `CLAUDE_CONFIG_DIR` - Standard Claude Code environment variable
    /// 4. Temporary directory (default)
    ///
    /// # Safety
    ///
    /// This method deliberately defaults to a temporary directory rather than `~/.claude`
    /// to prevent the simulator from accidentally modifying real Claude Code state.
    pub fn resolve() -> std::io::Result<Self> {
        if let Ok(dir) = std::env::var("CLAUDELESS_CONFIG_DIR") {
            Ok(Self::new(PathBuf::from(dir)))
        } else if let Ok(dir) = std::env::var("CLAUDELESS_STATE_DIR") {
            Ok(Self::new(PathBuf::from(dir)))
        } else if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
            Ok(Self::new(PathBuf::from(dir)))
        } else {
            // Default to temp directory to avoid touching real ~/.claude
            Self::temp()
        }
    }

    /// Initialize the directory structure.
    pub fn initialize(&mut self) -> Result<(), StateError> {
        // Create main directories
        fs::create_dir_all(self.todos_dir())?;
        fs::create_dir_all(self.projects_dir())?;
        fs::create_dir_all(self.plans_dir())?;
        fs::create_dir_all(self.sessions_dir())?;

        // Create settings file with defaults
        if !self.settings_path().exists() {
            fs::write(self.settings_path(), "{}")?;
        }

        // Set permissions (readable/writable by user only)
        self.set_permissions(self.root(), 0o700)?;

        self.initialized = true;
        Ok(())
    }

    /// Check if the directory has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the underlying path computation helper.
    pub fn paths(&self) -> &StatePaths {
        &self.paths
    }

    /// Get the root directory path.
    pub fn root(&self) -> &Path {
        self.paths.root()
    }

    /// Get the todos directory path.
    pub fn todos_dir(&self) -> PathBuf {
        self.paths.todos_dir()
    }

    /// Get the projects directory path.
    pub fn projects_dir(&self) -> PathBuf {
        self.paths.projects_dir()
    }

    /// Get the plans directory path.
    pub fn plans_dir(&self) -> PathBuf {
        self.paths.plans_dir()
    }

    /// Get the sessions directory path.
    pub fn sessions_dir(&self) -> PathBuf {
        self.paths.sessions_dir()
    }

    /// Get the settings file path.
    pub fn settings_path(&self) -> PathBuf {
        self.paths.settings_path()
    }

    /// Get a settings loader for this state directory and working directory.
    ///
    /// The loader will search for settings files in:
    /// 1. Global: {state_dir}/settings.json
    /// 2. Project: {working_dir}/.claude/settings.json
    /// 3. Local: {working_dir}/.claude/settings.local.json
    ///
    /// # Arguments
    /// * `working_dir` - The project working directory
    /// * `sources` - Optional list of sources to include. If None, all sources are loaded.
    pub fn settings_loader_with_sources(
        &self,
        working_dir: &Path,
        sources: Option<&[super::settings_source::SettingSource]>,
    ) -> super::settings_loader::SettingsLoader {
        let paths = super::settings_loader::SettingsPaths::resolve_with_sources(
            self.root(),
            working_dir,
            sources,
        );
        super::settings_loader::SettingsLoader::new(paths)
    }

    /// Get a settings loader that loads all sources (existing behavior).
    pub fn settings_loader(&self, working_dir: &Path) -> super::settings_loader::SettingsLoader {
        self.settings_loader_with_sources(working_dir, None)
    }

    /// Get the project directory for a given project path.
    ///
    /// Uses the same path normalization as the real Claude CLI:
    /// `/Users/foo/project` → `~/.claude/projects/-Users-foo-project`
    pub fn project_dir(&self, project_path: &Path) -> PathBuf {
        self.paths.project_dir(project_path)
    }

    /// Get the session file path for a given session ID.
    pub fn session_path(&self, session_id: &str) -> PathBuf {
        self.paths.session_path(session_id)
    }

    /// Get the todo file path for a given session/context.
    pub fn todo_path(&self, context: &str) -> PathBuf {
        self.paths.todo_path(context)
    }

    /// Reset state to clean slate
    pub fn reset(&mut self) -> Result<(), StateError> {
        // Remove all contents but keep structure
        for path in files_in(&self.todos_dir()) {
            fs::remove_file(path)?;
        }

        if self.projects_dir().exists() {
            fs::remove_dir_all(self.projects_dir())?;
            fs::create_dir_all(self.projects_dir())?;
        }

        for path in files_in(&self.plans_dir()) {
            fs::remove_file(path)?;
        }

        for path in files_in(&self.sessions_dir()) {
            fs::remove_file(path)?;
        }

        // Reset settings to defaults
        fs::write(self.settings_path(), "{}")?;

        Ok(())
    }

    fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), StateError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = Permissions::from_mode(mode);
            fs::set_permissions(path, perms)?;
        }
        #[cfg(not(unix))]
        {
            let _ = (path, mode);
        }
        Ok(())
    }

    /// Validate directory structure matches expected layout.
    ///
    /// Returns a list of warnings about any structural issues found.
    /// An empty list indicates the structure matches expectations.
    pub fn validate_structure(&self) -> Result<Vec<String>, StateError> {
        let mut warnings = Vec::new();

        // Check required directories
        let required_dirs = ["projects", "todos"];
        for dir in required_dirs {
            let path = self.root().join(dir);
            if !path.exists() {
                warnings.push(format!("Missing directory: {}", dir));
            }
        }

        // Check settings.json exists and is valid JSON
        let settings_path = self.settings_path();
        if settings_path.exists() {
            match fs::read_to_string(&settings_path) {
                Ok(content) => {
                    if let Err(e) = serde_json::from_str::<serde_json::Value>(&content) {
                        warnings.push(format!("Invalid settings.json: {}", e));
                    }
                }
                Err(e) => {
                    warnings.push(format!("Cannot read settings.json: {}", e));
                }
            }
        } else {
            warnings.push("Missing settings.json".to_string());
        }

        // Check project directories have correct structure
        let projects_dir = self.projects_dir();
        if projects_dir.exists() {
            if let Ok(entries) = fs::read_dir(&projects_dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let project_settings = entry.path().join("settings.json");
                        if !project_settings.exists() {
                            warnings.push(format!(
                                "Project {} missing settings.json",
                                entry.file_name().to_string_lossy()
                            ));
                        }
                    }
                }
            }
        }

        Ok(warnings)
    }
}

// Re-export path utilities for backwards compatibility
pub use super::paths::{normalize_project_path, project_dir_name};

#[cfg(test)]
#[path = "directory_tests.rs"]
mod tests;
