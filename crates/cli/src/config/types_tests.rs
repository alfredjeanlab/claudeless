// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

// =========================================================================
// Default impls
// =========================================================================

#[test]
fn scenario_config_defaults() {
    let config = ScenarioConfig::default();

    assert!(config.trusted);
    assert!(config.logged_in);
    assert!(config.permission_mode.is_none());
    assert!(config.default.is_none());
    assert!(config.responses.is_empty());
    assert!(config.tools.is_none());
}

#[test]
fn claude_config_defaults() {
    let config = ClaudeConfig::default();

    assert!(config.session_id.is_none());
    assert!(config.project_path.is_none());
    assert!(config.launch_timestamp.is_none());
    assert!(config.username.is_none());
    assert!(config.version.is_none());
    assert!(config.model.is_none());
    assert!(config.provider.is_none());
    assert!(config.placeholder.is_none());
    assert!(config.working_directory.is_none());
    assert!(config.show_welcome_back.is_none());
    assert!(config.welcome_back_right_panel.is_none());
    assert!(config.timeouts.is_none());
}

#[test]
fn response_defaults() {
    let resp = Response::default();

    assert!(resp.say.is_none());
    assert!(resp.tools.is_empty());
    assert!(resp.usage.is_none());
    assert!(resp.delay_ms.is_none());
}

#[test]
fn pattern_default_is_glob_star() {
    let pat = Pattern::default();
    match pat {
        Pattern::Glob(s) => assert_eq!(s, "*"),
        other => panic!("Expected Glob(\"*\"), got {:?}", other),
    }
}

#[test]
fn tools_config_defaults() {
    let config = ToolsConfig::default();

    assert_eq!(config.mode, ToolExecutionMode::Live);
    assert!(config.tools.is_empty());
}

#[test]
fn tool_config_defaults() {
    let config = ToolConfig::default();

    assert!(!config.approve);
    assert!(config.result.is_none());
    assert!(config.error.is_none());
    assert!(config.answers.is_none());
}

// =========================================================================
// Validation
// =========================================================================

#[test]
fn validate_accepts_valid_config() {
    let config = ScenarioConfig {
        claude: ClaudeConfig {
            session_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            launch_timestamp: Some("2025-01-15T10:30:00Z".to_string()),
            ..Default::default()
        },
        permission_mode: Some("plan".to_string()),
        ..Default::default()
    };

    assert!(config.validate().is_ok());
}

#[test]
fn validate_rejects_invalid_session_id() {
    let config = ScenarioConfig {
        claude: ClaudeConfig {
            session_id: Some("not-a-uuid".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let err = config.validate().unwrap_err();
    assert!(err.contains("session_id"));
}

#[test]
fn validate_rejects_invalid_launch_timestamp() {
    let config = ScenarioConfig {
        claude: ClaudeConfig {
            launch_timestamp: Some("not-a-timestamp".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let err = config.validate().unwrap_err();
    assert!(err.contains("launch_timestamp"));
}

#[test]
fn validate_rejects_invalid_permission_mode() {
    let config = ScenarioConfig {
        permission_mode: Some("invalid-mode".to_string()),
        ..Default::default()
    };

    let err = config.validate().unwrap_err();
    assert!(err.contains("permission_mode"));
}

#[test]
fn validate_accepts_all_permission_modes() {
    for mode in VALID_PERMISSION_MODES {
        let config = ScenarioConfig {
            permission_mode: Some(mode.to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_ok(), "mode '{}' should be valid", mode);
    }
}

#[test]
fn validate_accepts_empty_config() {
    let config = ScenarioConfig::default();
    assert!(config.validate().is_ok());
}
