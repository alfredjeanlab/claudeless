// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Simulated `~/.claude` directory structure.
//!
//! This module provides the `StateDirectory` type, which manages the directory
//! structure and I/O operations. For pure path computation without I/O, see
//! the [`paths`](super::paths) module.

use super::io::files_in;
use super::paths;
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
#[derive(Debug)]
pub struct StateDirectory {
    root: PathBuf,
    initialized: bool,
}

impl StateDirectory {
    /// Create a new state directory at the given root.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            initialized: false,
        }
    }

    /// Create a state directory in a temporary location.
    pub fn temp() -> std::io::Result<Self> {
        let temp = tempfile::tempdir()?;
        let path = temp.keep();
        Ok(Self {
            root: path,
            initialized: false,
        })
    }

    /// Resolve state directory from environment or default to a temp directory.
    ///
    /// Priority:
    /// 1. `CLAUDELESS_CONFIG_DIR` - Claudeless-specific override (highest priority)
    /// 2. `CLAUDELESS_STATE_DIR` - Legacy claudeless override (backwards compatibility)
    /// 3. `CLAUDE_CONFIG_DIR` - Standard Claude Code environment variable
    /// 4. Temporary directory (default)
    pub fn resolve() -> std::io::Result<Self> {
        if let Ok(dir) = std::env::var("CLAUDELESS_CONFIG_DIR") {
            Ok(Self::new(PathBuf::from(dir)))
        } else if let Ok(dir) = std::env::var("CLAUDELESS_STATE_DIR") {
            Ok(Self::new(PathBuf::from(dir)))
        } else if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
            Ok(Self::new(PathBuf::from(dir)))
        } else {
            Self::temp()
        }
    }

    /// Initialize the directory structure.
    pub fn initialize(&mut self) -> Result<(), StateError> {
        fs::create_dir_all(self.todos_dir())?;
        fs::create_dir_all(self.projects_dir())?;
        fs::create_dir_all(self.plans_dir())?;
        fs::create_dir_all(self.sessions_dir())?;

        if !self.settings_path().exists() {
            fs::write(self.settings_path(), "{}")?;
        }

        // Real Claude creates .claude.json on every startup.
        if !self.claude_json_path().exists() {
            fs::write(self.claude_json_path(), "{}\n")?;
        }

        self.set_permissions(&self.root, 0o700)?;
        self.initialized = true;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn todos_dir(&self) -> PathBuf {
        paths::todos_dir(&self.root)
    }

    pub fn projects_dir(&self) -> PathBuf {
        paths::projects_dir(&self.root)
    }

    pub fn plans_dir(&self) -> PathBuf {
        paths::plans_dir(&self.root)
    }

    pub fn sessions_dir(&self) -> PathBuf {
        paths::sessions_dir(&self.root)
    }

    pub fn settings_path(&self) -> PathBuf {
        paths::settings_path(&self.root)
    }

    pub fn claude_json_path(&self) -> PathBuf {
        paths::claude_json_path(&self.root)
    }

    /// Get a settings loader for this state directory and working directory.
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

    pub fn settings_loader(&self, working_dir: &Path) -> super::settings_loader::SettingsLoader {
        self.settings_loader_with_sources(working_dir, None)
    }

    pub fn project_dir(&self, project_path: &Path) -> PathBuf {
        paths::project_dir(&self.root, project_path)
    }

    pub fn session_path(&self, session_id: &str) -> PathBuf {
        paths::session_path(&self.root, session_id)
    }

    pub fn todo_path(&self, context: &str) -> PathBuf {
        paths::todo_path(&self.root, context)
    }

    /// Reset state to clean slate.
    pub fn reset(&mut self) -> Result<(), StateError> {
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
    pub fn validate_structure(&self) -> Result<Vec<String>, StateError> {
        let mut warnings = Vec::new();

        let required_dirs = ["projects", "todos"];
        for dir in required_dirs {
            let path = self.root.join(dir);
            if !path.exists() {
                warnings.push(format!("Missing directory: {}", dir));
            }
        }

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

#[cfg(test)]
#[path = "directory_tests.rs"]
mod tests;
