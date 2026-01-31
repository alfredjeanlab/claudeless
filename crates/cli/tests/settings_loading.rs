// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for settings file loading and merging.
//!
//! Tests the multi-file settings loading with correct precedence:
//! 1. Global (~/.claude/settings.json) - lowest priority
//! 2. Project (.claude/settings.json) - medium priority
//! 3. Local (.claude/settings.local.json) - highest priority

use claudeless::state::{SettingsPaths, StateDirectory};
use std::fs;
use tempfile::tempdir;

// =============================================================================
// Settings File Loading Tests
// =============================================================================

#[test]
fn test_global_settings_loaded() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Create global settings
    fs::write(
        state_dir.path().join("settings.json"),
        r#"{"permissions": {"allow": ["Read"]}}"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    assert_eq!(settings.permissions.allow, vec!["Read"]);
}

#[test]
fn test_project_settings_loaded() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Create project settings
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"permissions": {"allow": ["Write"]}}"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    assert_eq!(settings.permissions.allow, vec!["Write"]);
}

#[test]
fn test_local_settings_loaded() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Create local settings
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.local.json"),
        r#"{"permissions": {"allow": ["Bash"]}}"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    assert_eq!(settings.permissions.allow, vec!["Bash"]);
}

// =============================================================================
// Precedence Tests
// =============================================================================

#[test]
fn test_project_settings_override_global() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Global settings
    fs::write(
        state_dir.path().join("settings.json"),
        r#"{"permissions": {"allow": ["Read"]}}"#,
    )
    .unwrap();

    // Project settings (should override)
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"permissions": {"allow": ["Write"]}}"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    // Project wins over global
    assert_eq!(settings.permissions.allow, vec!["Write"]);
}

#[test]
fn test_local_settings_highest_priority() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Global, project, and local settings
    fs::write(
        state_dir.path().join("settings.json"),
        r#"{"permissions": {"allow": ["Read"]}}"#,
    )
    .unwrap();

    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"permissions": {"allow": ["Write"]}}"#,
    )
    .unwrap();
    fs::write(
        project_claude.join("settings.local.json"),
        r#"{"permissions": {"allow": ["Bash"]}}"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    // Local wins over project and global
    assert_eq!(settings.permissions.allow, vec!["Bash"]);
}

// =============================================================================
// Merge Behavior Tests
// =============================================================================

#[test]
fn test_env_vars_merged_across_files() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Global with one env var
    fs::write(
        state_dir.path().join("settings.json"),
        r#"{"env": {"GLOBAL": "1"}}"#,
    )
    .unwrap();

    // Project with another
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"env": {"PROJECT": "2"}}"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    // Both env vars should be present (maps are merged)
    assert_eq!(settings.env.get("GLOBAL"), Some(&"1".to_string()));
    assert_eq!(settings.env.get("PROJECT"), Some(&"2".to_string()));
}

#[test]
fn test_env_vars_later_overrides() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Global with env var
    fs::write(
        state_dir.path().join("settings.json"),
        r#"{"env": {"KEY": "global", "ONLY_GLOBAL": "yes"}}"#,
    )
    .unwrap();

    // Project overrides KEY
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"env": {"KEY": "project"}}"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    // KEY should be overridden, ONLY_GLOBAL should be preserved
    assert_eq!(settings.env.get("KEY"), Some(&"project".to_string()));
    assert_eq!(settings.env.get("ONLY_GLOBAL"), Some(&"yes".to_string()));
}

#[test]
fn test_mcp_servers_merged() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Global with one server
    fs::write(
        state_dir.path().join("settings.json"),
        r#"{"mcpServers": {"global-server": {"command": "global-cmd"}}}"#,
    )
    .unwrap();

    // Project with another server
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"mcpServers": {"project-server": {"command": "project-cmd"}}}"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    // Both servers should be present
    assert!(settings.mcp_servers.contains_key("global-server"));
    assert!(settings.mcp_servers.contains_key("project-server"));
}

#[test]
fn test_permission_arrays_replaced_not_merged() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Global with allow patterns
    fs::write(
        state_dir.path().join("settings.json"),
        r#"{"permissions": {"allow": ["Read", "Glob"], "deny": ["Bash(rm *)"]}}"#,
    )
    .unwrap();

    // Project replaces only allow (deny stays from global)
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"permissions": {"allow": ["Write"]}}"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    // allow is replaced entirely (not merged)
    assert_eq!(settings.permissions.allow, vec!["Write"]);
    // deny stays from global (empty array in project doesn't override)
    assert_eq!(settings.permissions.deny, vec!["Bash(rm *)"]);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_missing_files_ok() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // No settings files exist
    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    // Should get defaults
    assert!(settings.permissions.allow.is_empty());
    assert!(settings.permissions.deny.is_empty());
    assert!(settings.env.is_empty());
}

#[test]
fn test_invalid_json_skipped_with_warning() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Invalid global settings
    fs::write(state_dir.path().join("settings.json"), "not valid json").unwrap();

    // Valid project settings
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"permissions": {"allow": ["Read"]}}"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    // Project settings should still load despite global being invalid
    assert_eq!(settings.permissions.allow, vec!["Read"]);
}

#[test]
fn test_partial_invalid_still_loads_valid() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Invalid local, valid project
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"permissions": {"allow": ["Project"]}}"#,
    )
    .unwrap();
    fs::write(project_claude.join("settings.local.json"), "{{{invalid").unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    // Project settings should load despite local being invalid
    assert_eq!(settings.permissions.allow, vec!["Project"]);
}

// =============================================================================
// Schema Tests
// =============================================================================

#[test]
fn test_unknown_fields_preserved() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Settings with unknown fields (forward compatibility)
    fs::write(
        state_dir.path().join("settings.json"),
        r#"{
            "permissions": {"allow": ["Read"]},
            "futureFeature": true,
            "nestedFuture": {"key": "value"}
        }"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    // Known fields work
    assert_eq!(settings.permissions.allow, vec!["Read"]);
    // Unknown fields preserved in extra
    assert!(settings.extra.contains_key("futureFeature"));
    assert!(settings.extra.contains_key("nestedFuture"));
}

#[test]
fn test_mcp_server_config_fully_parsed() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    fs::write(
        state_dir.path().join("settings.json"),
        r#"{
            "mcpServers": {
                "test-server": {
                    "command": "npx",
                    "args": ["-y", "@test/server"],
                    "cwd": "/some/path",
                    "env": {"SERVER_KEY": "secret"}
                }
            }
        }"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    let server = settings.mcp_servers.get("test-server").unwrap();
    assert_eq!(server.command, "npx");
    assert_eq!(server.args, vec!["-y", "@test/server"]);
    assert_eq!(server.cwd.as_deref(), Some("/some/path"));
    assert_eq!(server.env.get("SERVER_KEY"), Some(&"secret".to_string()));
}

#[test]
fn test_additional_directories_loaded() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    fs::write(
        state_dir.path().join("settings.json"),
        r#"{
            "permissions": {
                "additionalDirectories": ["/tmp/workspace", "/home/shared"]
            }
        }"#,
    )
    .unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let settings = loader.load();

    assert_eq!(
        settings.permissions.additional_directories,
        vec!["/tmp/workspace", "/home/shared"]
    );
}

// =============================================================================
// SettingsPaths Tests
// =============================================================================

#[test]
fn test_settings_paths_resolve() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    let paths = SettingsPaths::resolve(state_dir.path(), work_dir.path());

    assert_eq!(paths.global, Some(state_dir.path().join("settings.json")));
    assert_eq!(
        paths.project,
        Some(work_dir.path().join(".claude/settings.json"))
    );
    assert_eq!(
        paths.local,
        Some(work_dir.path().join(".claude/settings.local.json"))
    );
}

#[test]
fn test_settings_paths_project_only() {
    let work_dir = tempdir().unwrap();

    let paths = SettingsPaths::project_only(work_dir.path());

    assert!(paths.global.is_none());
    assert_eq!(
        paths.project,
        Some(work_dir.path().join(".claude/settings.json"))
    );
    assert_eq!(
        paths.local,
        Some(work_dir.path().join(".claude/settings.local.json"))
    );
}

#[test]
fn test_existing_files_reported() {
    let state_dir = tempdir().unwrap();
    let work_dir = tempdir().unwrap();

    // Create only global and local files
    fs::write(state_dir.path().join("settings.json"), "{}").unwrap();

    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(project_claude.join("settings.local.json"), "{}").unwrap();

    let dir = StateDirectory::new(state_dir.path());
    let loader = dir.settings_loader(work_dir.path());
    let existing = loader.existing_files();

    // Should only list files that exist (global and local, not project)
    assert_eq!(existing.len(), 2);
}
