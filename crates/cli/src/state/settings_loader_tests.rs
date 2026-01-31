// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use std::fs;

#[test]
fn test_settings_paths_resolve() {
    let state_dir = Path::new("/home/user/.claude");
    let working_dir = Path::new("/home/user/project");

    let paths = SettingsPaths::resolve(state_dir, working_dir);

    assert_eq!(
        paths.global,
        Some(PathBuf::from("/home/user/.claude/settings.json"))
    );
    assert_eq!(
        paths.project,
        Some(PathBuf::from("/home/user/project/.claude/settings.json"))
    );
    assert_eq!(
        paths.local,
        Some(PathBuf::from(
            "/home/user/project/.claude/settings.local.json"
        ))
    );
}

#[test]
fn test_settings_paths_project_only() {
    let working_dir = Path::new("/home/user/project");

    let paths = SettingsPaths::project_only(working_dir);

    assert!(paths.global.is_none());
    assert_eq!(
        paths.project,
        Some(PathBuf::from("/home/user/project/.claude/settings.json"))
    );
    assert_eq!(
        paths.local,
        Some(PathBuf::from(
            "/home/user/project/.claude/settings.local.json"
        ))
    );
}

#[test]
fn test_loader_no_files() {
    let temp = tempfile::tempdir().unwrap();
    let paths = SettingsPaths::project_only(temp.path());
    let loader = SettingsLoader::new(paths);

    let settings = loader.load();

    // Should get defaults
    assert!(settings.permissions.allow.is_empty());
    assert!(settings.permissions.deny.is_empty());
}

#[test]
fn test_loader_global_only() {
    let temp = tempfile::tempdir().unwrap();
    let state_dir = temp.path();
    let work_dir = tempfile::tempdir().unwrap();

    // Create global settings
    fs::write(
        state_dir.join("settings.json"),
        r#"{"permissions": {"allow": ["Read"]}}"#,
    )
    .unwrap();

    let paths = SettingsPaths::resolve(state_dir, work_dir.path());
    let loader = SettingsLoader::new(paths);
    let settings = loader.load();

    assert_eq!(settings.permissions.allow, vec!["Read"]);
}

#[test]
fn test_loader_project_overrides_global() {
    let global_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();

    // Global settings
    fs::write(
        global_dir.path().join("settings.json"),
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

    let paths = SettingsPaths::resolve(global_dir.path(), work_dir.path());
    let loader = SettingsLoader::new(paths);
    let settings = loader.load();

    assert_eq!(settings.permissions.allow, vec!["Write"]);
}

#[test]
fn test_loader_local_highest_priority() {
    let global_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();

    // Global, project, and local settings
    fs::write(
        global_dir.path().join("settings.json"),
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

    let paths = SettingsPaths::resolve(global_dir.path(), work_dir.path());
    let loader = SettingsLoader::new(paths);
    let settings = loader.load();

    // Local wins
    assert_eq!(settings.permissions.allow, vec!["Bash"]);
}

#[test]
fn test_loader_env_vars_merged() {
    let global_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();

    // Global with one env var
    fs::write(
        global_dir.path().join("settings.json"),
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

    let paths = SettingsPaths::resolve(global_dir.path(), work_dir.path());
    let loader = SettingsLoader::new(paths);
    let settings = loader.load();

    // Both should be present
    assert_eq!(settings.env.get("GLOBAL"), Some(&"1".to_string()));
    assert_eq!(settings.env.get("PROJECT"), Some(&"2".to_string()));
}

#[test]
fn test_loader_invalid_json_skipped() {
    let global_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();

    // Invalid global settings
    fs::write(global_dir.path().join("settings.json"), "not valid json").unwrap();

    // Valid project settings
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"permissions": {"allow": ["Read"]}}"#,
    )
    .unwrap();

    let paths = SettingsPaths::resolve(global_dir.path(), work_dir.path());
    let loader = SettingsLoader::new(paths);
    let settings = loader.load();

    // Project settings should still load
    assert_eq!(settings.permissions.allow, vec!["Read"]);
}

#[test]
fn test_loader_existing_files() {
    let global_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();

    // Create only global and local files
    fs::write(global_dir.path().join("settings.json"), "{}").unwrap();

    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(project_claude.join("settings.local.json"), "{}").unwrap();

    let paths = SettingsPaths::resolve(global_dir.path(), work_dir.path());
    let loader = SettingsLoader::new(paths);
    let existing = loader.existing_files();

    // Should only list files that exist
    assert_eq!(existing.len(), 2);
}

// resolve_with_sources() tests

#[test]
fn test_resolve_with_sources_user_only() {
    let state_dir = Path::new("/home/user/.claude");
    let working_dir = Path::new("/home/user/project");

    let paths =
        SettingsPaths::resolve_with_sources(state_dir, working_dir, Some(&[SettingSource::User]));

    assert_eq!(
        paths.global,
        Some(PathBuf::from("/home/user/.claude/settings.json"))
    );
    assert!(paths.project.is_none());
    assert!(paths.local.is_none());
}

#[test]
fn test_resolve_with_sources_project_and_local() {
    let state_dir = Path::new("/home/user/.claude");
    let working_dir = Path::new("/home/user/project");

    let paths = SettingsPaths::resolve_with_sources(
        state_dir,
        working_dir,
        Some(&[SettingSource::Project, SettingSource::Local]),
    );

    assert!(paths.global.is_none());
    assert_eq!(
        paths.project,
        Some(PathBuf::from("/home/user/project/.claude/settings.json"))
    );
    assert_eq!(
        paths.local,
        Some(PathBuf::from(
            "/home/user/project/.claude/settings.local.json"
        ))
    );
}

#[test]
fn test_resolve_with_sources_none_means_all() {
    let state_dir = Path::new("/home/user/.claude");
    let working_dir = Path::new("/home/user/project");

    let paths = SettingsPaths::resolve_with_sources(state_dir, working_dir, None);

    assert!(paths.global.is_some());
    assert!(paths.project.is_some());
    assert!(paths.local.is_some());
}

#[test]
fn test_resolve_with_sources_empty_slice() {
    let state_dir = Path::new("/home/user/.claude");
    let working_dir = Path::new("/home/user/project");

    let paths = SettingsPaths::resolve_with_sources(state_dir, working_dir, Some(&[]));

    assert!(paths.global.is_none());
    assert!(paths.project.is_none());
    assert!(paths.local.is_none());
}

#[test]
fn test_loader_with_source_filter() {
    let global_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();

    // Global settings
    fs::write(
        global_dir.path().join("settings.json"),
        r#"{"permissions": {"allow": ["Read"]}}"#,
    )
    .unwrap();

    // Project settings
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"permissions": {"allow": ["Write"]}}"#,
    )
    .unwrap();

    // Local settings
    fs::write(
        project_claude.join("settings.local.json"),
        r#"{"permissions": {"allow": ["Bash"]}}"#,
    )
    .unwrap();

    // Load only project and local (skip global/user)
    let paths = SettingsPaths::resolve_with_sources(
        global_dir.path(),
        work_dir.path(),
        Some(&[SettingSource::Project, SettingSource::Local]),
    );
    let loader = SettingsLoader::new(paths);
    let settings = loader.load();

    // Local wins over project, but global was never loaded
    assert_eq!(settings.permissions.allow, vec!["Bash"]);
}

#[test]
fn test_loader_user_only_ignores_project() {
    let global_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();

    // Global settings
    fs::write(
        global_dir.path().join("settings.json"),
        r#"{"permissions": {"allow": ["Read"]}}"#,
    )
    .unwrap();

    // Project settings
    let project_claude = work_dir.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.json"),
        r#"{"permissions": {"allow": ["Write"]}}"#,
    )
    .unwrap();

    // Load only user/global
    let paths = SettingsPaths::resolve_with_sources(
        global_dir.path(),
        work_dir.path(),
        Some(&[SettingSource::User]),
    );
    let loader = SettingsLoader::new(paths);
    let settings = loader.load();

    // Should have global settings, not project
    assert_eq!(settings.permissions.allow, vec!["Read"]);
}

// load_with_overrides() tests

#[test]
fn test_load_with_overrides_inline_json() {
    let temp = tempfile::tempdir().unwrap();
    let paths = SettingsPaths::project_only(temp.path());
    let loader = SettingsLoader::new(paths);

    let cli_settings = vec![r#"{"permissions": {"allow": ["Read"]}}"#.to_string()];
    let settings = loader.load_with_overrides(&cli_settings);

    assert_eq!(settings.permissions.allow, vec!["Read"]);
}

#[test]
fn test_load_with_overrides_multiple() {
    let temp = tempfile::tempdir().unwrap();
    let paths = SettingsPaths::project_only(temp.path());
    let loader = SettingsLoader::new(paths);

    let cli_settings = vec![
        r#"{"permissions": {"allow": ["Read"]}}"#.to_string(),
        r#"{"permissions": {"allow": ["Write"]}}"#.to_string(),
    ];
    let settings = loader.load_with_overrides(&cli_settings);

    // Later settings should override earlier ones
    assert_eq!(settings.permissions.allow, vec!["Write"]);
}

#[test]
fn test_load_with_overrides_env_merged() {
    let temp = tempfile::tempdir().unwrap();
    let paths = SettingsPaths::project_only(temp.path());
    let loader = SettingsLoader::new(paths);

    let cli_settings = vec![
        r#"{"env": {"A": "1"}}"#.to_string(),
        r#"{"env": {"B": "2"}}"#.to_string(),
    ];
    let settings = loader.load_with_overrides(&cli_settings);

    // Env maps should be merged
    assert_eq!(settings.env.get("A"), Some(&"1".to_string()));
    assert_eq!(settings.env.get("B"), Some(&"2".to_string()));
}

#[test]
fn test_load_with_overrides_cli_overrides_file() {
    let global_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();

    // Create global settings file
    fs::write(
        global_dir.path().join("settings.json"),
        r#"{"permissions": {"allow": ["Read"]}}"#,
    )
    .unwrap();

    let paths = SettingsPaths::resolve(global_dir.path(), work_dir.path());
    let loader = SettingsLoader::new(paths);

    // CLI settings should override file settings
    let cli_settings = vec![r#"{"permissions": {"allow": ["Bash"]}}"#.to_string()];
    let settings = loader.load_with_overrides(&cli_settings);

    assert_eq!(settings.permissions.allow, vec!["Bash"]);
}

#[test]
fn test_load_with_overrides_invalid_skipped() {
    let temp = tempfile::tempdir().unwrap();
    let paths = SettingsPaths::project_only(temp.path());
    let loader = SettingsLoader::new(paths);

    // Mix of valid and invalid settings
    let cli_settings = vec![
        r#"{"permissions": {"allow": ["Read"]}}"#.to_string(),
        "not valid json".to_string(),
        r#"{"permissions": {"deny": ["Bash(rm *)"]}}"#.to_string(),
    ];
    let settings = loader.load_with_overrides(&cli_settings);

    // Valid settings should still be applied
    assert_eq!(settings.permissions.allow, vec!["Read"]);
    assert_eq!(settings.permissions.deny, vec!["Bash(rm *)"]);
}

#[test]
fn test_load_with_overrides_file_path() {
    let temp = tempfile::tempdir().unwrap();
    let settings_file = temp.path().join("custom-settings.json");
    fs::write(&settings_file, r#"{"permissions": {"allow": ["Write"]}}"#).unwrap();

    let paths = SettingsPaths::project_only(temp.path());
    let loader = SettingsLoader::new(paths);

    let cli_settings = vec![settings_file.to_str().unwrap().to_string()];
    let settings = loader.load_with_overrides(&cli_settings);

    assert_eq!(settings.permissions.allow, vec!["Write"]);
}
