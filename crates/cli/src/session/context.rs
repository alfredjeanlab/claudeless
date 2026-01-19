// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Session context merging scenario config with CLI args.

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
pub struct SessionContext {
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

impl SessionContext {
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
    /// This loads and merges settings from:
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
                    .and_then(|s| s.working_directory.as_ref())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Load and merge settings
        let loader = state_dir.settings_loader(&working_directory);
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
                .and_then(|s| s.default_model.clone())
                .unwrap_or_else(|| cli.model.clone())
        };

        // Claude version: scenario config or default
        let claude_version = scenario
            .and_then(|s| s.claude_version.clone())
            .unwrap_or_else(|| DEFAULT_CLAUDE_VERSION.to_string());

        // User name: scenario config or default
        let user_name = scenario
            .and_then(|s| s.user_name.clone())
            .unwrap_or_else(|| DEFAULT_USER_NAME.to_string());

        // Session ID: CLI overrides scenario, then generate random
        let session_id = cli
            .session_id
            .as_ref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .or_else(|| {
                scenario
                    .and_then(|s| s.session_id.as_ref())
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
                    .and_then(|s| s.working_directory.as_ref())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Project path: scenario config or working directory
        let project_path = scenario
            .and_then(|s| s.project_path.as_ref())
            .map(PathBuf::from)
            .unwrap_or_else(|| working_directory.clone());

        // Launch timestamp: scenario config or current time
        let launch_timestamp = scenario
            .and_then(|s| s.launch_timestamp.as_ref())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt: DateTime<chrono::FixedOffset>| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        // Trusted: scenario config (default true)
        let trusted = scenario.map(|s| s.trusted).unwrap_or(true);

        // Permission mode: scenario config or CLI default
        let permission_mode = scenario
            .and_then(|s| s.permission_mode.as_ref())
            .and_then(|s| parse_permission_mode(s))
            .unwrap_or_else(|| cli.permission_mode.clone());

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
mod tests {
    use super::*;
    use crate::cli::OutputFormat;
    use chrono::Datelike;

    fn default_cli() -> Cli {
        Cli {
            prompt: None,
            print: false,
            model: DEFAULT_MODEL.to_string(),
            output_format: OutputFormat::Text,
            max_tokens: None,
            system_prompt: None,
            continue_conversation: false,
            resume: None,
            allowed_tools: vec![],
            disallowed_tools: vec![],
            permission_mode: PermissionMode::Default,
            allow_dangerously_skip_permissions: false,
            dangerously_skip_permissions: false,
            input_file: None,
            cwd: None,
            input_format: "text".to_string(),
            session_id: None,
            verbose: false,
            debug: None,
            include_partial_messages: false,
            fallback_model: None,
            max_budget_usd: None,
            mcp_config: vec![],
            strict_mcp_config: false,
            mcp_debug: false,
            scenario: None,
            capture: None,
            failure: None,
            delay_ms: None,
            tui: false,
            no_tui: false,
            tool_execution_mode: None,
            sandbox_root: None,
            allow_real_bash: false,
        }
    }

    #[test]
    fn test_defaults_applied() {
        let cli = default_cli();
        let ctx = SessionContext::build(None, &cli);

        assert_eq!(ctx.model, DEFAULT_MODEL);
        assert_eq!(ctx.claude_version, DEFAULT_CLAUDE_VERSION);
        assert_eq!(ctx.user_name, DEFAULT_USER_NAME);
        assert!(ctx.trusted);
        assert_eq!(ctx.permission_mode, PermissionMode::Default);
    }

    #[test]
    fn test_scenario_overrides_defaults() {
        let cli = default_cli();
        let scenario = ScenarioConfig {
            name: "test".to_string(),
            default_model: Some("custom-model".to_string()),
            claude_version: Some("3.0.0".to_string()),
            user_name: Some("TestUser".to_string()),
            session_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            trusted: false,
            permission_mode: Some("plan".to_string()),
            ..Default::default()
        };

        let ctx = SessionContext::build(Some(&scenario), &cli);

        assert_eq!(ctx.model, "custom-model");
        assert_eq!(ctx.claude_version, "3.0.0");
        assert_eq!(ctx.user_name, "TestUser");
        assert_eq!(
            ctx.session_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert!(!ctx.trusted);
        assert_eq!(ctx.permission_mode, PermissionMode::Plan);
    }

    #[test]
    fn test_cli_overrides_scenario() {
        let mut cli = default_cli();
        cli.model = "cli-model".to_string();
        cli.cwd = Some("/cli/path".to_string());
        cli.session_id = Some("12345678-1234-1234-1234-123456789012".to_string());

        let scenario = ScenarioConfig {
            name: "test".to_string(),
            default_model: Some("scenario-model".to_string()),
            working_directory: Some("/scenario/path".to_string()),
            session_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            ..Default::default()
        };

        let ctx = SessionContext::build(Some(&scenario), &cli);

        // CLI should win
        assert_eq!(ctx.model, "cli-model");
        assert_eq!(ctx.working_directory, PathBuf::from("/cli/path"));
        assert_eq!(
            ctx.session_id.to_string(),
            "12345678-1234-1234-1234-123456789012"
        );
    }

    #[test]
    fn test_launch_timestamp_parsing() {
        let cli = default_cli();
        let scenario = ScenarioConfig {
            name: "test".to_string(),
            launch_timestamp: Some("2025-01-15T10:30:00Z".to_string()),
            ..Default::default()
        };

        let ctx = SessionContext::build(Some(&scenario), &cli);

        assert_eq!(ctx.launch_timestamp.year(), 2025);
        assert_eq!(ctx.launch_timestamp.month(), 1);
        assert_eq!(ctx.launch_timestamp.day(), 15);
    }

    #[test]
    fn test_project_path_defaults_to_working_dir() {
        let mut cli = default_cli();
        cli.cwd = Some("/work/dir".to_string());

        let scenario = ScenarioConfig {
            name: "test".to_string(),
            ..Default::default()
        };

        let ctx = SessionContext::build(Some(&scenario), &cli);

        assert_eq!(ctx.working_directory, PathBuf::from("/work/dir"));
        assert_eq!(ctx.project_path, PathBuf::from("/work/dir"));
    }

    #[test]
    fn test_project_path_override() {
        let cli = default_cli();
        let scenario = ScenarioConfig {
            name: "test".to_string(),
            project_path: Some("/project/path".to_string()),
            working_directory: Some("/work/dir".to_string()),
            ..Default::default()
        };

        let ctx = SessionContext::build(Some(&scenario), &cli);

        assert_eq!(ctx.working_directory, PathBuf::from("/work/dir"));
        assert_eq!(ctx.project_path, PathBuf::from("/project/path"));
    }

    #[test]
    fn test_permission_mode_parsing() {
        assert_eq!(
            parse_permission_mode("default"),
            Some(PermissionMode::Default)
        );
        assert_eq!(
            parse_permission_mode("accept-edits"),
            Some(PermissionMode::AcceptEdits)
        );
        assert_eq!(
            parse_permission_mode("bypass-permissions"),
            Some(PermissionMode::BypassPermissions)
        );
        assert_eq!(
            parse_permission_mode("delegate"),
            Some(PermissionMode::Delegate)
        );
        assert_eq!(
            parse_permission_mode("dont-ask"),
            Some(PermissionMode::DontAsk)
        );
        assert_eq!(parse_permission_mode("plan"), Some(PermissionMode::Plan));
        assert_eq!(parse_permission_mode("invalid"), None);
    }

    // =========================================================================
    // Settings Integration Tests
    // =========================================================================

    #[test]
    fn test_build_has_empty_settings() {
        let cli = default_cli();
        let ctx = SessionContext::build(None, &cli);

        assert!(ctx.settings().permissions.allow.is_empty());
        assert!(ctx.settings().permissions.deny.is_empty());
        assert!(ctx.settings_env().is_empty());
        assert!(ctx.additional_directories().is_empty());
    }

    #[test]
    fn test_build_with_settings() {
        use crate::state::{ClaudeSettings, PermissionSettings};

        let cli = default_cli();
        let settings = ClaudeSettings {
            permissions: PermissionSettings {
                allow: vec!["Read".to_string()],
                deny: vec!["Bash(rm *)".to_string()],
                additional_directories: vec!["/tmp".to_string()],
            },
            env: {
                let mut env = HashMap::new();
                env.insert("KEY".to_string(), "value".to_string());
                env
            },
            ..Default::default()
        };

        let ctx = SessionContext::build_with_settings(None, &cli, settings);

        assert_eq!(ctx.settings().permissions.allow, vec!["Read"]);
        assert_eq!(ctx.settings().permissions.deny, vec!["Bash(rm *)"]);
        assert_eq!(ctx.additional_directories(), &["/tmp"]);
        assert_eq!(ctx.settings_env().get("KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_permission_patterns_from_settings() {
        use crate::state::{ClaudeSettings, PermissionSettings};

        let cli = default_cli();
        let settings = ClaudeSettings {
            permissions: PermissionSettings {
                allow: vec!["Read".to_string(), "Glob".to_string()],
                deny: vec!["Bash(rm *)".to_string()],
                additional_directories: vec![],
            },
            ..Default::default()
        };

        let ctx = SessionContext::build_with_settings(None, &cli, settings);

        // Patterns should be compiled from settings
        assert!(ctx.permission_patterns().is_allowed("Read", None));
        assert!(ctx.permission_patterns().is_allowed("Glob", None));
        assert!(ctx
            .permission_patterns()
            .is_denied("Bash", Some("rm -rf /")));
        assert!(!ctx.permission_patterns().is_denied("Bash", Some("ls")));
    }

    #[test]
    fn test_permission_checker_uses_settings() {
        use crate::permission::{PermissionBypass, PermissionResult};
        use crate::state::{ClaudeSettings, PermissionSettings};

        let cli = default_cli();
        let settings = ClaudeSettings {
            permissions: PermissionSettings {
                allow: vec!["Read".to_string()],
                deny: vec!["Bash(rm *)".to_string()],
                additional_directories: vec![],
            },
            ..Default::default()
        };

        let ctx = SessionContext::build_with_settings(None, &cli, settings);
        let checker = ctx.permission_checker(PermissionBypass::default());

        // Read auto-approved by settings
        assert_eq!(checker.check("Read", "read"), PermissionResult::Allowed);

        // rm commands denied by settings
        assert!(matches!(
            checker.check_with_input("Bash", "execute", Some("rm -rf /")),
            PermissionResult::Denied { .. }
        ));

        // Other bash needs prompt
        assert!(matches!(
            checker.check_with_input("Bash", "execute", Some("ls")),
            PermissionResult::NeedsPrompt { .. }
        ));
    }

    #[test]
    fn test_permission_checker_with_overrides() {
        use crate::permission::{PermissionBypass, PermissionResult};
        use crate::state::{ClaudeSettings, PermissionSettings};

        let cli = default_cli();
        let settings = ClaudeSettings {
            permissions: PermissionSettings {
                allow: vec![],
                deny: vec!["Bash".to_string()], // Deny all Bash
                additional_directories: vec![],
            },
            ..Default::default()
        };

        let ctx = SessionContext::build_with_settings(None, &cli, settings);

        // Create scenario overrides that allow Bash
        let mut overrides = HashMap::new();
        overrides.insert(
            "Bash".to_string(),
            ToolConfig {
                auto_approve: true,
                result: None,
                error: None,
            },
        );

        let checker = ctx.permission_checker_with_overrides(PermissionBypass::default(), overrides);

        // Scenario override should beat settings deny
        assert_eq!(checker.check("Bash", "execute"), PermissionResult::Allowed);
    }

    #[test]
    fn test_build_with_state_loads_settings() {
        use std::fs;

        let work_dir = tempfile::tempdir().unwrap();
        let state_dir = StateDirectory::new(tempfile::tempdir().unwrap().path());

        // Create project settings file
        let claude_dir = work_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"permissions": {"allow": ["TestTool"]}}"#,
        )
        .unwrap();

        let mut cli = default_cli();
        cli.cwd = Some(work_dir.path().to_string_lossy().to_string());

        let ctx = SessionContext::build_with_state(None, &cli, &state_dir);

        // Settings should be loaded from project
        assert_eq!(ctx.settings().permissions.allow, vec!["TestTool"]);
    }
}
