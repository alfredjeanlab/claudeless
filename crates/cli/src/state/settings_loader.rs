// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Multi-file settings loading with precedence.
//!
//! Loads settings from multiple locations and merges them with correct precedence:
//! 1. Global (~/.claude/settings.json) - lowest priority
//! 2. Project (.claude/settings.json) - medium priority
//! 3. Local (.claude/settings.local.json) - highest priority

use super::settings::{load_settings_input, ClaudeSettings};
use super::settings_source::SettingSource;
use crate::output::print_warning;
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
    /// Resolve settings paths for a given working directory, optionally filtering by sources.
    ///
    /// # Arguments
    /// * `state_dir` - The ~/.claude equivalent (or CLAUDELESS_STATE_DIR)
    /// * `working_dir` - The project working directory
    /// * `sources` - Optional list of sources to include. If None, all sources are included.
    pub fn resolve_with_sources(
        state_dir: &Path,
        working_dir: &Path,
        sources: Option<&[SettingSource]>,
    ) -> Self {
        let sources = sources.unwrap_or(SettingSource::all());

        Self {
            global: sources
                .contains(&SettingSource::User)
                .then(|| state_dir.join("settings.json")),
            project: sources
                .contains(&SettingSource::Project)
                .then(|| working_dir.join(".claude").join("settings.json")),
            local: sources
                .contains(&SettingSource::Local)
                .then(|| working_dir.join(".claude").join("settings.local.json")),
        }
    }

    /// Resolve all settings paths (existing behavior, delegates to resolve_with_sources).
    ///
    /// # Arguments
    /// * `state_dir` - The ~/.claude equivalent (or CLAUDELESS_STATE_DIR)
    /// * `working_dir` - The project working directory
    pub fn resolve(state_dir: &Path, working_dir: &Path) -> Self {
        Self::resolve_with_sources(state_dir, working_dir, None)
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

    /// Load and merge all settings files, plus CLI-provided settings.
    ///
    /// Precedence (later overrides earlier):
    /// 1. Global (~/.claude/settings.json)
    /// 2. Project (.claude/settings.json)
    /// 3. Local (.claude/settings.local.json)
    /// 4. CLI --settings flags (in order specified)
    pub fn load_with_overrides(&self, cli_settings: &[String]) -> ClaudeSettings {
        let mut settings = self.load();

        for input in cli_settings {
            match load_settings_input(input) {
                Ok(cli_settings) => {
                    settings.merge(cli_settings);
                }
                Err(e) => {
                    print_warning(format_args!(
                        "Failed to load settings from '{}': {}",
                        input, e
                    ));
                }
            }
        }

        settings
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
                        print_warning(format_args!(
                            "Failed to load settings from {}: {}",
                            path.display(),
                            e
                        ));
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
