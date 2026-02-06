// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::hooks::HookEvent;
use crate::state::{ClaudeSettings, HookCommand, HookDefEntry};
use std::collections::HashMap;

#[test]
fn test_load_hooks_empty_settings() {
    let settings = ClaudeSettings::default();
    let executor = load_hooks(&settings).unwrap();
    assert!(!executor.has_hooks(&HookEvent::Stop));
}

#[test]
fn test_load_hooks_with_stop_hook() {
    let settings = ClaudeSettings {
        hooks: HashMap::from([(
            "Stop".to_string(),
            vec![HookDefEntry {
                matcher: None,
                hooks: vec![HookCommand {
                    command_type: "bash".to_string(),
                    command: "echo test".to_string(),
                    timeout: 5000,
                }],
            }],
        )]),
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::Stop));
    assert_eq!(executor.hook_count(&HookEvent::Stop), 1);
}

#[test]
fn test_load_hooks_ignores_unknown_events() {
    let settings = ClaudeSettings {
        hooks: HashMap::from([(
            "UnknownEvent".to_string(),
            vec![HookDefEntry {
                matcher: None,
                hooks: vec![HookCommand {
                    command_type: "bash".to_string(),
                    command: "echo test".to_string(),
                    timeout: 5000,
                }],
            }],
        )]),
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(!executor.has_hooks(&HookEvent::Stop));
}

#[test]
fn test_load_hooks_ignores_non_bash_commands() {
    let settings = ClaudeSettings {
        hooks: HashMap::from([(
            "Stop".to_string(),
            vec![HookDefEntry {
                matcher: None,
                hooks: vec![HookCommand {
                    command_type: "python".to_string(), // Not bash or command
                    command: "print('test')".to_string(),
                    timeout: 5000,
                }],
            }],
        )]),
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(!executor.has_hooks(&HookEvent::Stop));
}

#[test]
fn test_load_hooks_accepts_command_type() {
    let settings = ClaudeSettings {
        hooks: HashMap::from([(
            "Stop".to_string(),
            vec![HookDefEntry {
                matcher: None,
                hooks: vec![HookCommand {
                    command_type: "command".to_string(),
                    command: "echo test".to_string(),
                    timeout: 5000,
                }],
            }],
        )]),
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::Stop));
    assert_eq!(executor.hook_count(&HookEvent::Stop), 1);
}

#[test]
fn test_load_hooks_multiple_commands() {
    let settings = ClaudeSettings {
        hooks: HashMap::from([(
            "Stop".to_string(),
            vec![HookDefEntry {
                matcher: None,
                hooks: vec![
                    HookCommand {
                        command_type: "bash".to_string(),
                        command: "echo first".to_string(),
                        timeout: 5000,
                    },
                    HookCommand {
                        command_type: "bash".to_string(),
                        command: "echo second".to_string(),
                        timeout: 5000,
                    },
                ],
            }],
        )]),
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::Stop));
    assert_eq!(executor.hook_count(&HookEvent::Stop), 2);
}

#[test]
fn test_load_hooks_pre_tool_use() {
    let settings = ClaudeSettings {
        hooks: HashMap::from([(
            "PreToolUse".to_string(),
            vec![HookDefEntry {
                matcher: None,
                hooks: vec![HookCommand {
                    command_type: "bash".to_string(),
                    command: "echo pre-tool".to_string(),
                    timeout: 5000,
                }],
            }],
        )]),
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::PreToolExecution));
}

#[test]
fn test_load_hooks_post_tool_use() {
    let settings = ClaudeSettings {
        hooks: HashMap::from([(
            "PostToolUse".to_string(),
            vec![HookDefEntry {
                matcher: None,
                hooks: vec![HookCommand {
                    command_type: "bash".to_string(),
                    command: "echo post-tool".to_string(),
                    timeout: 5000,
                }],
            }],
        )]),
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::PostToolExecution));
}

#[test]
fn test_load_hooks_session_start() {
    let settings = ClaudeSettings {
        hooks: HashMap::from([(
            "SessionStart".to_string(),
            vec![HookDefEntry {
                matcher: None,
                hooks: vec![HookCommand {
                    command_type: "bash".to_string(),
                    command: "echo session-start".to_string(),
                    timeout: 5000,
                }],
            }],
        )]),
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::SessionStart));
    assert_eq!(executor.hook_count(&HookEvent::SessionStart), 1);
}

#[test]
fn test_load_hooks_notification_with_matcher() {
    let settings = ClaudeSettings {
        hooks: HashMap::from([(
            "Notification".to_string(),
            vec![HookDefEntry {
                matcher: Some("idle_prompt|permission_prompt".to_string()),
                hooks: vec![HookCommand {
                    command_type: "bash".to_string(),
                    command: "echo notify".to_string(),
                    timeout: 5000,
                }],
            }],
        )]),
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::Notification));
    assert_eq!(executor.hook_count(&HookEvent::Notification), 1);
}

#[test]
fn test_load_hooks_pretooluse_with_matcher() {
    let settings = ClaudeSettings {
        hooks: HashMap::from([(
            "PreToolUse".to_string(),
            vec![HookDefEntry {
                matcher: Some("AskUserQuestion|ExitPlanMode".to_string()),
                hooks: vec![HookCommand {
                    command_type: "command".to_string(),
                    command: "echo pre-tool".to_string(),
                    timeout: 5000,
                }],
            }],
        )]),
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::PreToolExecution));
    assert_eq!(executor.hook_count(&HookEvent::PreToolExecution), 1);
}
