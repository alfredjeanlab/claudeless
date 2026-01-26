// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_bypass_allows_all() {
    let checker =
        PermissionChecker::new(PermissionMode::Default, PermissionBypass::new(true, true));
    assert_eq!(checker.check("Bash", "execute"), PermissionResult::Allowed);
    assert_eq!(checker.check("Edit", "edit"), PermissionResult::Allowed);
    assert!(checker.is_bypassed());
}

#[test]
fn test_bypass_permissions_mode_allows_all() {
    let checker = PermissionChecker::new(
        PermissionMode::BypassPermissions,
        PermissionBypass::default(),
    );
    assert_eq!(checker.check("Bash", "execute"), PermissionResult::Allowed);
    assert!(checker.is_bypassed());
}

#[test]
fn test_accept_edits_allows_edits() {
    let checker = PermissionChecker::new(PermissionMode::AcceptEdits, PermissionBypass::default());
    assert_eq!(checker.check("Edit", "edit"), PermissionResult::Allowed);
    assert_eq!(checker.check("Write", "write"), PermissionResult::Allowed);
    assert_eq!(checker.check("Write", "create"), PermissionResult::Allowed);
    assert!(matches!(
        checker.check("Bash", "execute"),
        PermissionResult::NeedsPrompt { .. }
    ));
    assert!(!checker.is_bypassed());
}

#[test]
fn test_dont_ask_denies() {
    let checker = PermissionChecker::new(PermissionMode::DontAsk, PermissionBypass::default());
    let result = checker.check("Bash", "execute");
    assert!(matches!(result, PermissionResult::Denied { .. }));
    if let PermissionResult::Denied { reason } = result {
        assert!(reason.contains("DontAsk"));
    }
}

#[test]
fn test_plan_mode_denies_execution() {
    let checker = PermissionChecker::new(PermissionMode::Plan, PermissionBypass::default());
    let result = checker.check("Bash", "execute");
    assert!(matches!(result, PermissionResult::Denied { .. }));
    if let PermissionResult::Denied { reason } = result {
        assert!(reason.contains("Plan"));
    }
}

#[test]
fn test_default_mode_needs_prompt() {
    let checker = PermissionChecker::new(PermissionMode::Default, PermissionBypass::default());
    let result = checker.check("Bash", "execute");
    assert!(matches!(result, PermissionResult::NeedsPrompt { .. }));
    if let PermissionResult::NeedsPrompt { tool, action } = result {
        assert_eq!(tool, "Bash");
        assert_eq!(action, "execute");
    }
}

#[test]
fn test_delegate_mode_needs_prompt() {
    let checker = PermissionChecker::new(PermissionMode::Delegate, PermissionBypass::default());
    assert!(matches!(
        checker.check("Bash", "execute"),
        PermissionResult::NeedsPrompt { .. }
    ));
}

#[test]
fn test_is_edit_action() {
    assert!(is_edit_action("edit"));
    assert!(is_edit_action("Edit"));
    assert!(is_edit_action("EDIT"));
    assert!(is_edit_action("write"));
    assert!(is_edit_action("create"));
    assert!(is_edit_action("delete"));
    assert!(is_edit_action("modify"));
    assert!(!is_edit_action("execute"));
    assert!(!is_edit_action("read"));
}

// =========================================================================
// Settings Patterns Tests
// =========================================================================

#[test]
fn test_settings_allow_auto_approves() {
    use crate::state::PermissionSettings;

    let settings = PermissionSettings {
        allow: vec!["Read".to_string()],
        deny: vec![],
        additional_directories: vec![],
    };
    let patterns = PermissionPatterns::from_settings(&settings);

    let checker = PermissionChecker::with_patterns(
        PermissionMode::Default,
        PermissionBypass::default(),
        patterns,
    );

    // Read is auto-approved by settings
    assert_eq!(checker.check("Read", "read"), PermissionResult::Allowed);

    // Other tools still need prompt
    assert!(matches!(
        checker.check("Bash", "execute"),
        PermissionResult::NeedsPrompt { .. }
    ));
}

#[test]
fn test_settings_deny_blocks() {
    use crate::state::PermissionSettings;

    let settings = PermissionSettings {
        allow: vec![],
        deny: vec!["Bash(rm *)".to_string()],
        additional_directories: vec![],
    };
    let patterns = PermissionPatterns::from_settings(&settings);

    let checker = PermissionChecker::with_patterns(
        PermissionMode::Default,
        PermissionBypass::default(),
        patterns,
    );

    // rm commands are denied by settings
    let result = checker.check_with_input("Bash", "execute", Some("rm -rf /"));
    assert!(matches!(result, PermissionResult::Denied { .. }));

    // Other bash commands still need prompt
    assert!(matches!(
        checker.check_with_input("Bash", "execute", Some("ls")),
        PermissionResult::NeedsPrompt { .. }
    ));
}

#[test]
fn test_settings_deny_beats_allow() {
    use crate::state::PermissionSettings;

    let settings = PermissionSettings {
        allow: vec!["Bash".to_string()],
        deny: vec!["Bash(rm *)".to_string()],
        additional_directories: vec![],
    };
    let patterns = PermissionPatterns::from_settings(&settings);

    let checker = PermissionChecker::with_patterns(
        PermissionMode::Default,
        PermissionBypass::default(),
        patterns,
    );

    // Generic Bash is allowed
    assert_eq!(
        checker.check_with_input("Bash", "execute", Some("echo hello")),
        PermissionResult::Allowed
    );

    // But rm commands are denied (deny beats allow)
    let result = checker.check_with_input("Bash", "execute", Some("rm -rf /"));
    assert!(matches!(result, PermissionResult::Denied { .. }));
}

// =========================================================================
// Scenario Override Tests
// =========================================================================

#[test]
fn test_scenario_override_auto_approve() {
    let mut overrides = HashMap::new();
    overrides.insert(
        "Bash".to_string(),
        ToolConfig {
            auto_approve: true,
            result: None,
            error: None,
        },
    );

    let checker = PermissionChecker::new(PermissionMode::Default, PermissionBypass::default())
        .with_scenario_overrides(overrides);

    // Bash is auto-approved by scenario
    assert_eq!(checker.check("Bash", "execute"), PermissionResult::Allowed);

    // Other tools still need prompt
    assert!(matches!(
        checker.check("Read", "read"),
        PermissionResult::NeedsPrompt { .. }
    ));
}

#[test]
fn test_scenario_override_error() {
    let mut overrides = HashMap::new();
    overrides.insert(
        "Bash".to_string(),
        ToolConfig {
            auto_approve: false,
            result: None,
            error: Some("Simulated error".to_string()),
        },
    );

    let checker = PermissionChecker::new(PermissionMode::Default, PermissionBypass::default())
        .with_scenario_overrides(overrides);

    let result = checker.check("Bash", "execute");
    assert!(matches!(result, PermissionResult::Denied { .. }));
    if let PermissionResult::Denied { reason } = result {
        assert_eq!(reason, "Simulated error");
    }
}

#[test]
fn test_scenario_overrides_beat_settings() {
    use crate::state::PermissionSettings;

    let settings = PermissionSettings {
        allow: vec![],
        deny: vec!["Bash".to_string()], // Deny all Bash
        additional_directories: vec![],
    };
    let patterns = PermissionPatterns::from_settings(&settings);

    // Scenario override to allow Bash
    let mut overrides = HashMap::new();
    overrides.insert(
        "Bash".to_string(),
        ToolConfig {
            auto_approve: true,
            result: None,
            error: None,
        },
    );

    let checker = PermissionChecker::with_patterns(
        PermissionMode::Default,
        PermissionBypass::default(),
        patterns,
    )
    .with_scenario_overrides(overrides);

    // Scenario override wins - Bash is allowed despite settings deny
    assert_eq!(checker.check("Bash", "execute"), PermissionResult::Allowed);
}

// =========================================================================
// Priority Order Tests
// =========================================================================

#[test]
fn test_bypass_beats_everything() {
    use crate::state::PermissionSettings;

    // Even with deny patterns and scenario errors, bypass wins
    let settings = PermissionSettings {
        allow: vec![],
        deny: vec!["Bash".to_string()],
        additional_directories: vec![],
    };
    let patterns = PermissionPatterns::from_settings(&settings);

    let mut overrides = HashMap::new();
    overrides.insert(
        "Bash".to_string(),
        ToolConfig {
            auto_approve: false,
            result: None,
            error: Some("Error".to_string()),
        },
    );

    let checker = PermissionChecker::with_patterns(
        PermissionMode::DontAsk,           // Would normally deny
        PermissionBypass::new(true, true), // But bypass is active
        patterns,
    )
    .with_scenario_overrides(overrides);

    // Bypass wins over everything
    assert_eq!(checker.check("Bash", "execute"), PermissionResult::Allowed);
}

#[test]
fn test_mode_applies_when_no_patterns_match() {
    use crate::state::PermissionSettings;

    let settings = PermissionSettings {
        allow: vec!["Read".to_string()],
        deny: vec![],
        additional_directories: vec![],
    };
    let patterns = PermissionPatterns::from_settings(&settings);

    let checker = PermissionChecker::with_patterns(
        PermissionMode::DontAsk, // Denies unmatched tools
        PermissionBypass::default(),
        patterns,
    );

    // Read is allowed by settings
    assert_eq!(checker.check("Read", "read"), PermissionResult::Allowed);

    // Bash is denied by mode (no pattern match)
    let result = checker.check("Bash", "execute");
    assert!(matches!(result, PermissionResult::Denied { .. }));
}
