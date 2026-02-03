// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::hooks::HookEvent;
use crate::state::{ClaudeSettings, HookCommand, HookDef, HookMatcher};

#[test]
fn test_load_hooks_empty_settings() {
    let settings = ClaudeSettings::default();
    let executor = load_hooks(&settings).unwrap();
    assert!(!executor.has_hooks(&HookEvent::Stop));
}

#[test]
fn test_load_hooks_with_stop_hook() {
    let settings = ClaudeSettings {
        hooks: vec![HookDef {
            matcher: HookMatcher {
                event: "Stop".to_string(),
            },
            hooks: vec![HookCommand {
                command_type: "bash".to_string(),
                command: "echo test".to_string(),
                timeout: 5000,
            }],
        }],
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::Stop));
    assert_eq!(executor.hook_count(&HookEvent::Stop), 1);
}

#[test]
fn test_load_hooks_ignores_unknown_events() {
    let settings = ClaudeSettings {
        hooks: vec![HookDef {
            matcher: HookMatcher {
                event: "UnknownEvent".to_string(),
            },
            hooks: vec![HookCommand {
                command_type: "bash".to_string(),
                command: "echo test".to_string(),
                timeout: 5000,
            }],
        }],
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(!executor.has_hooks(&HookEvent::Stop));
}

#[test]
fn test_load_hooks_ignores_non_bash_commands() {
    let settings = ClaudeSettings {
        hooks: vec![HookDef {
            matcher: HookMatcher {
                event: "Stop".to_string(),
            },
            hooks: vec![HookCommand {
                command_type: "python".to_string(), // Not bash
                command: "print('test')".to_string(),
                timeout: 5000,
            }],
        }],
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(!executor.has_hooks(&HookEvent::Stop));
}

#[test]
fn test_load_hooks_multiple_commands() {
    let settings = ClaudeSettings {
        hooks: vec![HookDef {
            matcher: HookMatcher {
                event: "Stop".to_string(),
            },
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
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::Stop));
    assert_eq!(executor.hook_count(&HookEvent::Stop), 2);
}

#[test]
fn test_load_hooks_pre_tool_use() {
    let settings = ClaudeSettings {
        hooks: vec![HookDef {
            matcher: HookMatcher {
                event: "PreToolUse".to_string(),
            },
            hooks: vec![HookCommand {
                command_type: "bash".to_string(),
                command: "echo pre-tool".to_string(),
                timeout: 5000,
            }],
        }],
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::PreToolExecution));
}

#[test]
fn test_load_hooks_post_tool_use() {
    let settings = ClaudeSettings {
        hooks: vec![HookDef {
            matcher: HookMatcher {
                event: "PostToolUse".to_string(),
            },
            hooks: vec![HookCommand {
                command_type: "bash".to_string(),
                command: "echo post-tool".to_string(),
                timeout: 5000,
            }],
        }],
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::PostToolExecution));
}

#[test]
fn test_load_hooks_session_start() {
    let settings = ClaudeSettings {
        hooks: vec![HookDef {
            matcher: HookMatcher {
                event: "SessionStart".to_string(),
            },
            hooks: vec![HookCommand {
                command_type: "bash".to_string(),
                command: "echo session-start".to_string(),
                timeout: 5000,
            }],
        }],
        ..Default::default()
    };

    let executor = load_hooks(&settings).unwrap();
    assert!(executor.has_hooks(&HookEvent::SessionStart));
    assert_eq!(executor.hook_count(&HookEvent::SessionStart), 1);
}
