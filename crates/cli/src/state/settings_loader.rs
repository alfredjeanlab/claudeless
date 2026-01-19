// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Multi-file settings loading with precedence.
//!
//! Loads settings from multiple locations and merges them with correct precedence:
//! 1. Global (~/.claude/settings.json) - lowest priority
//! 2. Project (.claude/settings.json) - medium priority
//! 3. Local (.claude/settings.local.json) - highest priority

use super::settings::ClaudeSettings;
use std::path::{Path, PathBuf};

/// Paths to search for settings files.
#[derive(Clone, Debug)]
pub struct SettingsPaths {
    /// Global settings (~/.claude/settings.json)
    pub global: Option<PathBuf>,
    /// Project settings (.claude/settings.json)
    pub project: Option<PathBuf>,
    /// Local overrides (.claude/settings.local.json)
    pub local: Option<PathBuf>,
}

impl SettingsPaths {
    /// Resolve settings paths for a given working directory.
    ///
    /// # Arguments
    /// * `state_dir` - The ~/.claude equivalent (or CLAUDELESS_STATE_DIR)
    /// * `working_dir` - The project working directory
    pub fn resolve(state_dir: &Path, working_dir: &Path) -> Self {
        Self {
            global: Some(state_dir.join("settings.json")),
            project: Some(working_dir.join(".claude").join("settings.json")),
            local: Some(working_dir.join(".claude").join("settings.local.json")),
        }
    }

    /// Create paths for testing (no global).
    pub fn project_only(working_dir: &Path) -> Self {
        Self {
            global: None,
            project: Some(working_dir.join(".claude").join("settings.json")),
            local: Some(working_dir.join(".claude").join("settings.local.json")),
        }
    }
}

/// Loads and merges settings from multiple files.
pub struct SettingsLoader {
    paths: SettingsPaths,
}

impl SettingsLoader {
    /// Create a new settings loader.
    pub fn new(paths: SettingsPaths) -> Self {
        Self { paths }
    }

    /// Load and merge all settings files.
    ///
    /// Precedence (later overrides earlier):
    /// 1. Global (~/.claude/settings.json)
    /// 2. Project (.claude/settings.json)
    /// 3. Local (.claude/settings.local.json)
    ///
    /// Missing files are silently skipped.
    pub fn load(&self) -> ClaudeSettings {
        let mut settings = ClaudeSettings::default();

        // Load in precedence order
        for path in [&self.paths.global, &self.paths.project, &self.paths.local]
            .into_iter()
            .flatten()
        {
            if path.exists() {
                match ClaudeSettings::load(path) {
                    Ok(file_settings) => {
                        settings.merge(file_settings);
                    }
                    Err(e) => {
                        // Log warning but continue - don't fail on invalid settings
                        eprintln!(
                            "Warning: Failed to load settings from {}: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        settings
    }

    /// Check which settings files exist.
    pub fn existing_files(&self) -> Vec<&Path> {
        [&self.paths.global, &self.paths.project, &self.paths.local]
            .into_iter()
            .flatten()
            .filter(|path| path.exists())
            .map(|path| path.as_path())
            .collect()
    }

    /// Get the paths being used.
    pub fn paths(&self) -> &SettingsPaths {
        &self.paths
    }
}

#[cfg(test)]
#[path = "settings_loader_tests.rs"]
mod tests;
