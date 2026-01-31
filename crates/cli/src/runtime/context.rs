// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Runtime context merging scenario config with CLI args.

use crate::cli::Cli;
use crate::config::{
    ScenarioConfig, ToolConfig, DEFAULT_CLAUDE_VERSION, DEFAULT_MODEL, DEFAULT_USER_NAME,
};
use crate::permission::{PermissionBypass, PermissionChecker, PermissionMode, PermissionPatterns};
use crate::state::{ClaudeSettings, StateDirectory};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Merged configuration from scenario + CLI, with defaults applied.
///
/// Precedence rules:
/// - CLI args override scenario config
/// - Scenario config overrides defaults
#[derive(Clone, Debug)]
pub struct RuntimeContext {
    /// Model to use for this session.
    pub model: String,
    /// Claude version string.
    pub claude_version: String,
    /// User display name.
    pub user_name: String,
    /// Session UUID.
    pub session_id: Uuid,
    /// Project path for state directory naming.
    pub project_path: PathBuf,
    /// Working directory for tool execution.
    pub working_directory: PathBuf,
    /// Session start timestamp.
    pub launch_timestamp: DateTime<Utc>,
    /// Whether the working directory is trusted.
    pub trusted: bool,
    /// Permission mode for tool execution.
    pub permission_mode: PermissionMode,
    /// Effective settings (merged from all sources)
    effective_settings: ClaudeSettings,
    /// Compiled permission patterns from settings
    permission_patterns: PermissionPatterns,
}

impl RuntimeContext {
    /// Build context from scenario and CLI, applying precedence rules:
    /// CLI args > scenario config > defaults
    ///
    /// Note: This method creates a context without settings file support.
    /// Use `build_with_state` to include settings file loading.
    pub fn build(scenario: Option<&ScenarioConfig>, cli: &Cli) -> Self {
        Self::build_internal(scenario, cli, ClaudeSettings::default())
    }

    /// Build context with settings loaded from state directory.
    ///
    /// Loads settings from sources specified in CLI args, or all sources if not specified:
    /// 1. Global: {state_dir}/settings.json
    /// 2. Project: {working_dir}/.claude/settings.json
    /// 3. Local: {working_dir}/.claude/settings.local.json
    pub fn build_with_state(
        scenario: Option<&ScenarioConfig>,
        cli: &Cli,
        state_dir: &StateDirectory,
    ) -> Self {
        // First build to get the working directory
        let working_directory = cli
            .cwd
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| {
                scenario
                    .and_then(|s| s.environment.working_directory.as_ref())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Load and merge settings with source filtering
        let sources = cli.setting_sources.as_deref();
        let loader = state_dir.settings_loader_with_sources(&working_directory, sources);
        let effective_settings = loader.load();

        Self::build_internal(scenario, cli, effective_settings)
    }

    /// Build context with custom settings.
    ///
    /// Useful for testing or when settings are loaded separately.
    pub fn build_with_settings(
        scenario: Option<&ScenarioConfig>,
        cli: &Cli,
        settings: ClaudeSettings,
    ) -> Self {
        Self::build_internal(scenario, cli, settings)
    }

    fn build_internal(
        scenario: Option<&ScenarioConfig>,
        cli: &Cli,
        effective_settings: ClaudeSettings,
    ) -> Self {
        // Model: CLI always wins, fall back to scenario, then default
        let model = if cli.model != DEFAULT_MODEL {
            cli.model.clone()
        } else {
            scenario
                .and_then(|s| s.identity.default_model.clone())
                .unwrap_or_else(|| cli.model.clone())
        };

        // Claude version: scenario config or default
        let claude_version = scenario
            .and_then(|s| s.identity.claude_version.clone())
            .unwrap_or_else(|| DEFAULT_CLAUDE_VERSION.to_string());

        // User name: scenario config or default
        let user_name = scenario
            .and_then(|s| s.identity.user_name.clone())
            .unwrap_or_else(|| DEFAULT_USER_NAME.to_string());

        // Session ID: CLI overrides scenario, then generate random
        let session_id = cli
            .session
            .session_id
            .as_ref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .or_else(|| {
                scenario
                    .and_then(|s| s.identity.session_id.as_ref())
                    .and_then(|s| Uuid::parse_str(s).ok())
            })
            .unwrap_or_else(Uuid::new_v4);

        // Working directory: CLI overrides scenario, then current dir
        let working_directory = cli
            .cwd
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| {
                scenario
                    .and_then(|s| s.environment.working_directory.as_ref())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Project path: scenario config or working directory
        let project_path = scenario
            .and_then(|s| s.environment.project_path.as_ref())
            .map(PathBuf::from)
            .unwrap_or_else(|| working_directory.clone());

        // Launch timestamp: scenario config or current time
        let launch_timestamp = scenario
            .and_then(|s| s.timing.launch_timestamp.as_ref())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt: DateTime<chrono::FixedOffset>| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        // Trusted: scenario config (default true)
        let trusted = scenario.map(|s| s.environment.trusted).unwrap_or(true);

        // Permission mode: scenario config or CLI default
        let permission_mode = scenario
            .and_then(|s| s.environment.permission_mode.as_ref())
            .and_then(|s| parse_permission_mode(s))
            .unwrap_or_else(|| cli.permissions.permission_mode.clone());

        // Compile permission patterns from settings
        let permission_patterns =
            PermissionPatterns::from_settings(&effective_settings.permissions);

        Self {
            model,
            claude_version,
            user_name,
            session_id,
            project_path,
            working_directory,
            launch_timestamp,
            trusted,
            permission_mode,
            effective_settings,
            permission_patterns,
        }
    }

    /// Get effective settings (merged from all sources).
    pub fn settings(&self) -> &ClaudeSettings {
        &self.effective_settings
    }

    /// Get permission patterns from settings.
    pub fn permission_patterns(&self) -> &PermissionPatterns {
        &self.permission_patterns
    }

    /// Create a permission checker with current context.
    ///
    /// The checker uses:
    /// - Current permission mode
    /// - Bypass configuration from CLI args
    /// - Settings patterns for auto-approve/deny
    /// - Optional scenario tool overrides
    pub fn permission_checker(&self, bypass: PermissionBypass) -> PermissionChecker {
        PermissionChecker::with_patterns(
            self.permission_mode.clone(),
            bypass,
            self.permission_patterns.clone(),
        )
    }

    /// Create a permission checker with scenario tool overrides.
    pub fn permission_checker_with_overrides(
        &self,
        bypass: PermissionBypass,
        scenario_tools: HashMap<String, ToolConfig>,
    ) -> PermissionChecker {
        self.permission_checker(bypass)
            .with_scenario_overrides(scenario_tools)
    }

    /// Get environment variables from settings.
    pub fn settings_env(&self) -> &HashMap<String, String> {
        &self.effective_settings.env
    }

    /// Get additional directories from settings.
    pub fn additional_directories(&self) -> &[String] {
        &self.effective_settings.permissions.additional_directories
    }
}

/// Parse permission mode from string.
fn parse_permission_mode(s: &str) -> Option<PermissionMode> {
    match s.to_lowercase().as_str() {
        "default" => Some(PermissionMode::Default),
        "accept-edits" | "acceptedits" => Some(PermissionMode::AcceptEdits),
        "bypass-permissions" | "bypasspermissions" => Some(PermissionMode::BypassPermissions),
        "delegate" => Some(PermissionMode::Delegate),
        "dont-ask" | "dontask" => Some(PermissionMode::DontAsk),
        "plan" => Some(PermissionMode::Plan),
        _ => None,
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
