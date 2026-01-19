// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Integration tests for settings-based permission patterns.
//!
//! Tests tool pattern matching for `permissions.allow` and `permissions.deny`.

use claudeless::config::ToolConfig;
use claudeless::permission::{
    PermissionBypass, PermissionChecker, PermissionMode, PermissionPatterns, PermissionResult,
    ToolPattern,
};
use claudeless::state::PermissionSettings;
use std::collections::HashMap;

// =============================================================================
// ToolPattern Tests
// =============================================================================

mod tool_pattern_parsing {
    use super::*;

    #[test]
    fn test_simple_tool_name() {
        let pattern = ToolPattern::parse("Read").unwrap();
        assert!(pattern.matches("Read", None));
        assert!(pattern.matches("Read", Some("/any/path")));
        assert!(!pattern.matches("Write", None));
    }

    #[test]
    fn test_case_insensitive() {
        let pattern = ToolPattern::parse("Read").unwrap();
        assert!(pattern.matches("read", None));
        assert!(pattern.matches("READ", None));
        assert!(pattern.matches("rEaD", None));
    }

    #[test]
    fn test_exact_argument() {
        let pattern = ToolPattern::parse("Bash(npm test)").unwrap();
        assert!(pattern.matches("Bash", Some("npm test")));
        assert!(!pattern.matches("Bash", Some("npm install")));
        assert!(!pattern.matches("Bash", None));
    }

    #[test]
    fn test_glob_star() {
        let pattern = ToolPattern::parse("Bash(npm *)").unwrap();
        assert!(pattern.matches("Bash", Some("npm test")));
        assert!(pattern.matches("Bash", Some("npm install")));
        assert!(pattern.matches("Bash", Some("npm run build")));
        assert!(!pattern.matches("Bash", Some("cargo test")));
    }

    #[test]
    fn test_glob_question_mark() {
        let pattern = ToolPattern::parse("Bash(test?)").unwrap();
        assert!(pattern.matches("Bash", Some("test1")));
        assert!(pattern.matches("Bash", Some("testa")));
        assert!(!pattern.matches("Bash", Some("test")));
        assert!(!pattern.matches("Bash", Some("test12")));
    }

    #[test]
    fn test_glob_brackets() {
        let pattern = ToolPattern::parse("Bash([abc]*)").unwrap();
        assert!(pattern.matches("Bash", Some("alpha")));
        assert!(pattern.matches("Bash", Some("bravo")));
        assert!(pattern.matches("Bash", Some("charlie")));
        assert!(!pattern.matches("Bash", Some("delta")));
    }

    #[test]
    fn test_file_pattern() {
        let pattern = ToolPattern::parse("Write(*.md)").unwrap();
        assert!(pattern.matches("Write", Some("README.md")));
        assert!(pattern.matches("Write", Some("CHANGELOG.md")));
        assert!(!pattern.matches("Write", Some("main.rs")));
    }

    #[test]
    fn test_invalid_patterns() {
        assert!(ToolPattern::parse("").is_none());
        assert!(ToolPattern::parse("   ").is_none());
    }
}

// =============================================================================
// PermissionPatterns Tests
// =============================================================================

mod permission_patterns {
    use super::*;

    #[test]
    fn test_from_settings() {
        let settings = PermissionSettings {
            allow: vec!["Read".to_string(), "Glob".to_string()],
            deny: vec!["Bash(rm *)".to_string()],
            additional_directories: vec![],
        };

        let patterns = PermissionPatterns::from_settings(&settings);

        assert!(patterns.is_allowed("Read", None));
        assert!(patterns.is_allowed("Glob", None));
        assert!(patterns.is_denied("Bash", Some("rm -rf /")));
        assert!(!patterns.is_denied("Bash", Some("ls")));
    }

    #[test]
    fn test_empty_patterns() {
        let patterns = PermissionPatterns::default();
        assert!(patterns.is_empty());
        assert!(!patterns.is_allowed("Read", None));
        assert!(!patterns.is_denied("Bash", None));
    }

    #[test]
    fn test_both_allow_and_deny_can_match() {
        let settings = PermissionSettings {
            allow: vec!["Bash".to_string()],
            deny: vec!["Bash(rm *)".to_string()],
            additional_directories: vec![],
        };

        let patterns = PermissionPatterns::from_settings(&settings);

        // A command can match both allow and deny
        // Caller is responsible for checking deny first
        assert!(patterns.is_allowed("Bash", Some("rm file")));
        assert!(patterns.is_denied("Bash", Some("rm file")));
    }

    #[test]
    fn test_skips_invalid_patterns() {
        let settings = PermissionSettings {
            allow: vec!["".to_string(), "Read".to_string(), "   ".to_string()],
            deny: vec![],
            additional_directories: vec![],
        };

        let patterns = PermissionPatterns::from_settings(&settings);

        // Only valid pattern should be parsed
        assert!(patterns.is_allowed("Read", None));
        assert!(!patterns.is_allowed("", None));
    }
}

// =============================================================================
// Permission Checker Integration Tests
// =============================================================================

mod permission_checker_integration {
    use super::*;

    #[test]
    fn test_settings_allow_auto_approves() {
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
    fn test_deny_beats_allow() {
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

    #[test]
    fn test_scenario_overrides_beat_settings() {
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

    #[test]
    fn test_scenario_error_overrides_settings() {
        let settings = PermissionSettings {
            allow: vec!["Bash".to_string()], // Allow all Bash
            deny: vec![],
            additional_directories: vec![],
        };
        let patterns = PermissionPatterns::from_settings(&settings);

        // Scenario override with error
        let mut overrides = HashMap::new();
        overrides.insert(
            "Bash".to_string(),
            ToolConfig {
                auto_approve: false,
                result: None,
                error: Some("Simulated failure".to_string()),
            },
        );

        let checker = PermissionChecker::with_patterns(
            PermissionMode::Default,
            PermissionBypass::default(),
            patterns,
        )
        .with_scenario_overrides(overrides);

        // Scenario error overrides settings allow
        let result = checker.check("Bash", "execute");
        assert!(matches!(result, PermissionResult::Denied { .. }));
        if let PermissionResult::Denied { reason } = result {
            assert_eq!(reason, "Simulated failure");
        }
    }
}

// =============================================================================
// Priority Order Tests
// =============================================================================

mod priority_order {
    use super::*;

    #[test]
    fn test_bypass_beats_everything() {
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

    #[test]
    fn test_accept_edits_mode_with_settings() {
        let settings = PermissionSettings {
            allow: vec!["Read".to_string()],
            deny: vec!["Write".to_string()], // Deny Write
            additional_directories: vec![],
        };
        let patterns = PermissionPatterns::from_settings(&settings);

        let checker = PermissionChecker::with_patterns(
            PermissionMode::AcceptEdits,
            PermissionBypass::default(),
            patterns,
        );

        // Read is allowed by settings
        assert_eq!(checker.check("Read", "read"), PermissionResult::Allowed);

        // Write is denied by settings (even though AcceptEdits would allow edits)
        let result = checker.check("Write", "edit");
        assert!(matches!(result, PermissionResult::Denied { .. }));

        // Edit (not in deny list) is allowed by AcceptEdits mode
        assert_eq!(checker.check("Edit", "edit"), PermissionResult::Allowed);
    }
}

// =============================================================================
// Real-World Pattern Tests
// =============================================================================

mod real_world_patterns {
    use super::*;

    #[test]
    fn test_npm_commands() {
        let settings = PermissionSettings {
            allow: vec!["Bash(npm test)".to_string(), "Bash(npm run *)".to_string()],
            deny: vec!["Bash(npm publish)".to_string()],
            additional_directories: vec![],
        };
        let patterns = PermissionPatterns::from_settings(&settings);

        assert!(patterns.is_allowed("Bash", Some("npm test")));
        assert!(patterns.is_allowed("Bash", Some("npm run build")));
        assert!(patterns.is_allowed("Bash", Some("npm run dev")));
        assert!(patterns.is_denied("Bash", Some("npm publish")));
        assert!(!patterns.is_allowed("Bash", Some("npm install")));
    }

    #[test]
    fn test_dangerous_commands() {
        let settings = PermissionSettings {
            allow: vec![],
            deny: vec![
                "Bash(rm *)".to_string(),
                "Bash(sudo *)".to_string(),
                "Bash(chmod *)".to_string(),
            ],
            additional_directories: vec![],
        };
        let patterns = PermissionPatterns::from_settings(&settings);

        assert!(patterns.is_denied("Bash", Some("rm -rf /")));
        assert!(patterns.is_denied("Bash", Some("sudo apt install")));
        assert!(patterns.is_denied("Bash", Some("chmod 777 file")));
        assert!(!patterns.is_denied("Bash", Some("ls -la")));
    }

    #[test]
    fn test_file_type_patterns() {
        let settings = PermissionSettings {
            allow: vec![
                "Read".to_string(),
                "Write(*.md)".to_string(),
                "Write(*.txt)".to_string(),
            ],
            deny: vec!["Write(.env)".to_string(), "Write(*.key)".to_string()],
            additional_directories: vec![],
        };
        let patterns = PermissionPatterns::from_settings(&settings);

        // Read is always allowed
        assert!(patterns.is_allowed("Read", Some("/any/file.rs")));

        // Can write markdown and text files
        assert!(patterns.is_allowed("Write", Some("README.md")));
        assert!(patterns.is_allowed("Write", Some("notes.txt")));

        // Cannot write sensitive files
        assert!(patterns.is_denied("Write", Some(".env")));
        assert!(patterns.is_denied("Write", Some("secret.key")));
    }

    #[test]
    fn test_git_commands() {
        let settings = PermissionSettings {
            allow: vec![
                "Bash(git status)".to_string(),
                "Bash(git log *)".to_string(),
                "Bash(git diff *)".to_string(),
            ],
            deny: vec![
                "Bash(git push --force *)".to_string(),
                "Bash(git reset --hard *)".to_string(),
            ],
            additional_directories: vec![],
        };
        let patterns = PermissionPatterns::from_settings(&settings);

        assert!(patterns.is_allowed("Bash", Some("git status")));
        assert!(patterns.is_allowed("Bash", Some("git log --oneline")));
        assert!(patterns.is_allowed("Bash", Some("git diff HEAD~1")));
        assert!(patterns.is_denied("Bash", Some("git push --force origin main")));
        assert!(patterns.is_denied("Bash", Some("git reset --hard HEAD~1")));
    }
}
