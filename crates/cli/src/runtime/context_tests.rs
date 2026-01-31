// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::cli::{McpOptions, OutputOptions, PermissionOptions, SessionOptions, SimulatorOptions};
use chrono::Datelike;

fn default_cli() -> Cli {
    Cli {
        prompt: None,
        print: false,
        model: DEFAULT_MODEL.to_string(),
        system_prompt: None,
        allowed_tools: vec![],
        disallowed_tools: vec![],
        input_file: None,
        cwd: None,
        setting_sources: None,
        input_format: "text".to_string(),
        fallback_model: None,
        max_budget_usd: None,
        settings: vec![],
        output: OutputOptions::default(),
        session: SessionOptions::default(),
        permissions: PermissionOptions::default(),
        mcp: McpOptions::default(),
        simulator: SimulatorOptions::default(),
    }
}

#[test]
fn test_defaults_applied() {
    let cli = default_cli();
    let ctx = RuntimeContext::build(None, &cli);

    assert_eq!(ctx.model, DEFAULT_MODEL);
    assert_eq!(ctx.claude_version, DEFAULT_CLAUDE_VERSION);
    assert_eq!(ctx.user_name, DEFAULT_USER_NAME);
    assert!(ctx.trusted);
    assert_eq!(ctx.permission_mode, PermissionMode::Default);
}

#[test]
fn test_scenario_overrides_defaults() {
    use crate::config::{EnvironmentConfig, IdentityConfig};

    let cli = default_cli();
    let scenario = ScenarioConfig {
        name: "test".to_string(),
        identity: IdentityConfig {
            default_model: Some("custom-model".to_string()),
            claude_version: Some("3.0.0".to_string()),
            user_name: Some("TestUser".to_string()),
            session_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
        },
        environment: EnvironmentConfig {
            trusted: false,
            permission_mode: Some("plan".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let ctx = RuntimeContext::build(Some(&scenario), &cli);

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
    use crate::config::{EnvironmentConfig, IdentityConfig};

    let mut cli = default_cli();
    cli.model = "cli-model".to_string();
    cli.cwd = Some("/cli/path".to_string());
    cli.session.session_id = Some("12345678-1234-1234-1234-123456789012".to_string());

    let scenario = ScenarioConfig {
        name: "test".to_string(),
        identity: IdentityConfig {
            default_model: Some("scenario-model".to_string()),
            session_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            ..Default::default()
        },
        environment: EnvironmentConfig {
            working_directory: Some("/scenario/path".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let ctx = RuntimeContext::build(Some(&scenario), &cli);

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
    use crate::config::TimingConfig;

    let cli = default_cli();
    let scenario = ScenarioConfig {
        name: "test".to_string(),
        timing: TimingConfig {
            launch_timestamp: Some("2025-01-15T10:30:00Z".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let ctx = RuntimeContext::build(Some(&scenario), &cli);

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

    let ctx = RuntimeContext::build(Some(&scenario), &cli);

    assert_eq!(ctx.working_directory, PathBuf::from("/work/dir"));
    assert_eq!(ctx.project_path, PathBuf::from("/work/dir"));
}

#[test]
fn test_project_path_override() {
    use crate::config::EnvironmentConfig;

    let cli = default_cli();
    let scenario = ScenarioConfig {
        name: "test".to_string(),
        environment: EnvironmentConfig {
            project_path: Some("/project/path".to_string()),
            working_directory: Some("/work/dir".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let ctx = RuntimeContext::build(Some(&scenario), &cli);

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
    let ctx = RuntimeContext::build(None, &cli);

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

    let ctx = RuntimeContext::build_with_settings(None, &cli, settings);

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

    let ctx = RuntimeContext::build_with_settings(None, &cli, settings);

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

    let ctx = RuntimeContext::build_with_settings(None, &cli, settings);
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

    let ctx = RuntimeContext::build_with_settings(None, &cli, settings);

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

    let ctx = RuntimeContext::build_with_state(None, &cli, &state_dir);

    // Settings should be loaded from project
    assert_eq!(ctx.settings().permissions.allow, vec!["TestTool"]);
}
