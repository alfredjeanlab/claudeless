// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::paths::normalize_project_path;
use super::*;

#[test]
fn test_new_state_directory() {
    let dir = StateDirectory::new("/tmp/test-claude");
    assert_eq!(dir.root(), Path::new("/tmp/test-claude"));
    assert!(!dir.is_initialized());
}

#[test]
fn test_temp_state_directory() {
    let dir = StateDirectory::temp().unwrap();
    assert!(dir.root().exists());
    assert!(!dir.is_initialized());
}

#[test]
fn test_initialize() {
    let mut dir = StateDirectory::temp().unwrap();
    dir.initialize().unwrap();

    assert!(dir.is_initialized());
    assert!(dir.todos_dir().exists());
    assert!(dir.projects_dir().exists());
    assert!(dir.plans_dir().exists());
    assert!(dir.sessions_dir().exists());
    assert!(dir.settings_path().exists());
}

#[test]
fn test_path_methods() {
    let dir = StateDirectory::new("/home/user/.claude");

    assert_eq!(dir.todos_dir(), PathBuf::from("/home/user/.claude/todos"));
    assert_eq!(
        dir.projects_dir(),
        PathBuf::from("/home/user/.claude/projects")
    );
    assert_eq!(dir.plans_dir(), PathBuf::from("/home/user/.claude/plans"));
    assert_eq!(
        dir.sessions_dir(),
        PathBuf::from("/home/user/.claude/sessions")
    );
    assert_eq!(
        dir.settings_path(),
        PathBuf::from("/home/user/.claude/settings.json")
    );
}

#[test]
fn test_project_dir() {
    let dir = StateDirectory::new("/home/user/.claude");
    let project_dir = dir.project_dir(Path::new("/some/project"));
    assert!(project_dir.starts_with("/home/user/.claude/projects/"));
    // Should use normalized path format, not hash
    assert!(project_dir.to_string_lossy().contains("-some-project"));
}

// =========================================================================
// Path Normalization Tests
// =========================================================================
//
// These tests verify that path normalization matches the real Claude CLI
// behavior observed in ~/.claude/projects/ directory names.
//
// Real examples from Claude CLI v2.1.12:
// - /Users/user/Developer/myproject → -Users-user-Developer-myproject
// - /private/var/folders/.../tmp.xxx → -private-var-folders-...-tmp-xxx

#[test]
fn test_normalize_project_path_absolute() {
    // Standard absolute path
    assert_eq!(
        normalize_project_path(Path::new("/Users/user/Developer/myproject")),
        "-Users-user-Developer-myproject"
    );
}

#[test]
fn test_normalize_project_path_simple() {
    assert_eq!(normalize_project_path(Path::new("/tmp/test")), "-tmp-test");
}

#[test]
fn test_normalize_project_path_root() {
    assert_eq!(normalize_project_path(Path::new("/")), "-");
}

#[test]
fn test_normalize_project_path_deep_nesting() {
    // Deep path like macOS temp directories
    // Real Claude CLI behavior: both `/` and `.` are replaced with `-`
    assert_eq!(
        normalize_project_path(Path::new(
            "/private/var/folders/t5/6tq8cxtj20z035rv8hsnzwvh0000gn/T/tmp.4wnhxcEF1K"
        )),
        "-private-var-folders-t5-6tq8cxtj20z035rv8hsnzwvh0000gn-T-tmp-4wnhxcEF1K"
    );
}

#[test]
fn test_normalize_project_path_relative() {
    // Relative paths don't start with /
    assert_eq!(
        normalize_project_path(Path::new("relative/path")),
        "relative-path"
    );
}

#[test]
fn test_normalize_project_path_single_component() {
    assert_eq!(normalize_project_path(Path::new("project")), "project");
    assert_eq!(normalize_project_path(Path::new("/project")), "-project");
}

#[test]
fn test_normalize_project_path_with_dots() {
    // Dots are replaced with hyphens (matches real Claude CLI behavior)
    assert_eq!(
        normalize_project_path(Path::new("/home/user/.config/app")),
        "-home-user--config-app"
    );
}

#[test]
fn test_normalize_project_path_with_hyphens() {
    // Existing hyphens should be preserved
    assert_eq!(
        normalize_project_path(Path::new("/my-project/sub-dir")),
        "-my-project-sub-dir"
    );
}

#[test]
fn test_normalize_project_path_trailing_slash() {
    // Trailing slash becomes trailing hyphen
    assert_eq!(
        normalize_project_path(Path::new("/path/to/dir/")),
        "-path-to-dir-"
    );
}

#[test]
fn test_normalize_project_path_multiple_slashes() {
    // Multiple consecutive slashes become multiple hyphens
    // Note: Path::new normalizes these, but we test the raw behavior
    let path_str = "/path//double";
    assert_eq!(path_str.replace('/', "-"), "-path--double");
}

#[test]
fn test_normalize_project_path_unicode() {
    // Unicode characters should be preserved
    assert_eq!(
        normalize_project_path(Path::new("/home/用户/项目")),
        "-home-用户-项目"
    );
}

#[test]
fn test_normalize_project_path_spaces() {
    // Spaces in paths should be preserved
    assert_eq!(
        normalize_project_path(Path::new("/Users/John Doe/My Project")),
        "-Users-John Doe-My Project"
    );
}

#[test]
fn test_project_dir_name_matches_real_claude() {
    // Verify our normalization matches observed real Claude behavior
    // These are actual directory names from ~/.claude/projects/

    // Note: project_dir_name tries to canonicalize first, so for non-existent
    // paths it falls back to the raw path normalization
    let name = normalize_project_path(Path::new("/Users/user/Developer/myproject"));
    assert_eq!(name, "-Users-user-Developer-myproject");

    let name = normalize_project_path(Path::new("/Users/user"));
    assert_eq!(name, "-Users-user");
}

#[test]
fn test_project_dir_uses_normalized_name() {
    let dir = StateDirectory::new("/home/user/.claude");
    let project = dir.project_dir(Path::new("/Users/test/myproject"));

    // The full path should be: base/projects/normalized_name
    assert_eq!(
        project,
        PathBuf::from("/home/user/.claude/projects/-Users-test-myproject")
    );
}

#[test]
fn test_session_path() {
    let dir = StateDirectory::new("/home/user/.claude");
    assert_eq!(
        dir.session_path("session_abc123"),
        PathBuf::from("/home/user/.claude/sessions/session_abc123.json")
    );
}

#[test]
fn test_todo_path() {
    let dir = StateDirectory::new("/home/user/.claude");
    assert_eq!(
        dir.todo_path("default"),
        PathBuf::from("/home/user/.claude/todos/default.json")
    );
}

#[test]
fn test_reset() {
    let mut dir = StateDirectory::temp().unwrap();
    dir.initialize().unwrap();

    // Create some files
    fs::write(dir.todos_dir().join("test.json"), "{}").unwrap();
    fs::create_dir_all(dir.projects_dir().join("test_project")).unwrap();
    fs::write(dir.plans_dir().join("plan.json"), "{}").unwrap();
    fs::write(dir.sessions_dir().join("session.json"), "{}").unwrap();

    // Verify files exist
    assert!(dir.todos_dir().join("test.json").exists());
    assert!(dir.projects_dir().join("test_project").exists());

    // Reset
    dir.reset().unwrap();

    // Verify files removed but directories remain
    assert!(!dir.todos_dir().join("test.json").exists());
    assert!(!dir.projects_dir().join("test_project").exists());
    assert!(dir.todos_dir().exists());
    assert!(dir.projects_dir().exists());
}

#[cfg(unix)]
#[test]
fn test_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let mut dir = StateDirectory::temp().unwrap();
    dir.initialize().unwrap();

    let perms = fs::metadata(dir.root()).unwrap().permissions();
    assert_eq!(perms.mode() & 0o777, 0o700);
}

// ==========================================================================
// Safety Tests: Verify simulator never touches real ~/.claude by default
// ==========================================================================
//
// These tests ensure the simulator defaults to a temporary directory rather
// than ~/.claude. This is critical because:
//
// 1. The simulator is a TEST DOUBLE - it should never affect real Claude state
// 2. Running tests could corrupt a user's actual Claude Code configuration
// 3. ~/.claude may contain sensitive session data, API keys, or settings
// 4. Parallel test runs could conflict if sharing the real ~/.claude
//
// The only way to use a real path is to explicitly set CLAUDELESS_CONFIG_DIR.

#[test]
fn test_resolve_defaults_to_temp_directory_not_home() {
    // SAFETY: When no environment variable is set, resolve() must return
    // a temporary directory, NOT ~/.claude. This prevents accidental
    // modification of real Claude Code state during testing.

    // Clear the env var to test default behavior
    std::env::remove_var("CLAUDELESS_STATE_DIR");

    let dir = StateDirectory::resolve().unwrap();
    let root = dir.root().to_path_buf();

    // The resolved path must NOT be ~/.claude
    if let Ok(home) = std::env::var("HOME") {
        let real_claude_dir = PathBuf::from(&home).join(".claude");
        assert_ne!(
            root, real_claude_dir,
            "SAFETY VIOLATION: resolve() returned ~/.claude without explicit configuration. \
             The simulator must never touch real Claude state by default."
        );
    }

    // The path should be in a temp directory location
    let root_str = root.to_string_lossy();
    let is_temp_path =
        root_str.contains("tmp") || root_str.contains("temp") || root_str.contains("var/folders"); // macOS temp location

    assert!(
        is_temp_path,
        "Expected resolve() to return a temp directory path, got: {}",
        root_str
    );
}

#[test]
fn test_resolve_respects_env_var_override() {
    // When CLAUDELESS_CONFIG_DIR is explicitly set, resolve() should use it.
    // This allows users to intentionally specify a directory for testing.

    let test_dir = tempfile::tempdir().unwrap();
    let test_path = test_dir.path().join("custom-claudeless");

    std::env::set_var("CLAUDELESS_CONFIG_DIR", &test_path);

    let dir = StateDirectory::resolve().unwrap();
    assert_eq!(
        dir.root(),
        test_path.as_path(),
        "resolve() should use CLAUDELESS_CONFIG_DIR when set"
    );

    // Clean up env var
    std::env::remove_var("CLAUDELESS_CONFIG_DIR");
}

#[test]
fn test_resolve_with_env_var_does_not_require_existing_dir() {
    // The directory specified by CLAUDELESS_CONFIG_DIR doesn't need to exist
    // yet - it will be created when initialize() is called.
    //
    // Use a unique temp dir to avoid race conditions with other tests
    // that also set CLAUDELESS_CONFIG_DIR.
    let temp = tempfile::tempdir().unwrap();
    let non_existent = temp.path().join("claudeless-test-nonexistent");
    assert!(!non_existent.exists());

    std::env::set_var("CLAUDELESS_CONFIG_DIR", &non_existent);

    let dir = StateDirectory::resolve().unwrap();
    assert_eq!(dir.root(), non_existent.as_path());
    assert!(!dir.is_initialized());

    // Clean up
    std::env::remove_var("CLAUDELESS_CONFIG_DIR");
}

#[test]
fn test_resolve_respects_claude_config_dir() {
    // When CLAUDE_CONFIG_DIR is set (and CLAUDELESS_CONFIG_DIR/CLAUDELESS_STATE_DIR are not),
    // resolve() should use it. This provides compatibility with Claude Code's
    // standard environment variable.

    let test_dir = tempfile::tempdir().unwrap();
    let test_path = test_dir.path().join("claude-config");

    // Ensure claudeless-specific vars are not set
    std::env::remove_var("CLAUDELESS_CONFIG_DIR");
    std::env::remove_var("CLAUDELESS_STATE_DIR");
    std::env::set_var("CLAUDE_CONFIG_DIR", &test_path);

    let dir = StateDirectory::resolve().unwrap();
    assert_eq!(
        dir.root(),
        test_path.as_path(),
        "resolve() should use CLAUDE_CONFIG_DIR when claudeless-specific vars are not set"
    );

    // Clean up env var
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

#[test]
fn test_resolve_claudeless_config_dir_takes_precedence() {
    // When CLAUDELESS_CONFIG_DIR, CLAUDELESS_STATE_DIR, and CLAUDE_CONFIG_DIR are all set,
    // CLAUDELESS_CONFIG_DIR should take precedence.

    let test_dir = tempfile::tempdir().unwrap();
    let config_path = test_dir.path().join("claudeless-config");
    let state_path = test_dir.path().join("claudeless-state");
    let claude_path = test_dir.path().join("claude-config");

    std::env::set_var("CLAUDELESS_CONFIG_DIR", &config_path);
    std::env::set_var("CLAUDELESS_STATE_DIR", &state_path);
    std::env::set_var("CLAUDE_CONFIG_DIR", &claude_path);

    let dir = StateDirectory::resolve().unwrap();
    assert_eq!(
        dir.root(),
        config_path.as_path(),
        "CLAUDELESS_CONFIG_DIR should take precedence over CLAUDELESS_STATE_DIR and CLAUDE_CONFIG_DIR"
    );

    // Clean up env vars
    std::env::remove_var("CLAUDELESS_CONFIG_DIR");
    std::env::remove_var("CLAUDELESS_STATE_DIR");
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

#[test]
fn test_resolve_claudeless_state_dir_backwards_compat() {
    // CLAUDELESS_STATE_DIR should still work for backwards compatibility,
    // taking precedence over CLAUDE_CONFIG_DIR but not CLAUDELESS_CONFIG_DIR.

    let test_dir = tempfile::tempdir().unwrap();
    let state_path = test_dir.path().join("claudeless-state");
    let claude_path = test_dir.path().join("claude-config");

    // Ensure CLAUDELESS_CONFIG_DIR is not set
    std::env::remove_var("CLAUDELESS_CONFIG_DIR");
    std::env::set_var("CLAUDELESS_STATE_DIR", &state_path);
    std::env::set_var("CLAUDE_CONFIG_DIR", &claude_path);

    let dir = StateDirectory::resolve().unwrap();
    assert_eq!(
        dir.root(),
        state_path.as_path(),
        "CLAUDELESS_STATE_DIR should take precedence over CLAUDE_CONFIG_DIR for backwards compatibility"
    );

    // Clean up env vars
    std::env::remove_var("CLAUDELESS_STATE_DIR");
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

#[test]
fn test_temp_creates_unique_directories() {
    // Each call to temp() should create a unique directory to prevent
    // test interference when running parallel tests.

    let dir1 = StateDirectory::temp().unwrap();
    let dir2 = StateDirectory::temp().unwrap();

    assert_ne!(
        dir1.root(),
        dir2.root(),
        "temp() must create unique directories for test isolation"
    );
}

// =========================================================================
// Structure Validation Tests
// =========================================================================

#[test]
fn test_validate_structure_after_init() {
    let mut dir = StateDirectory::temp().unwrap();
    dir.initialize().unwrap();

    let warnings = dir.validate_structure().unwrap();
    assert!(
        warnings.is_empty(),
        "Initialized directory should have no warnings: {:?}",
        warnings
    );
}

#[test]
fn test_validate_structure_missing_dirs() {
    let dir = StateDirectory::temp().unwrap();
    // Don't initialize - should have missing directories

    let warnings = dir.validate_structure().unwrap();
    assert!(!warnings.is_empty());
    assert!(warnings.iter().any(|w| w.contains("Missing directory")));
}

#[test]
fn test_validate_structure_invalid_settings() {
    let mut dir = StateDirectory::temp().unwrap();
    dir.initialize().unwrap();

    // Write invalid JSON to settings.json
    fs::write(dir.settings_path(), "not valid json").unwrap();

    let warnings = dir.validate_structure().unwrap();
    assert!(warnings.iter().any(|w| w.contains("Invalid settings.json")));
}

#[test]
fn test_validate_structure_project_without_settings() {
    let mut dir = StateDirectory::temp().unwrap();
    dir.initialize().unwrap();

    // Create a project directory without settings.json
    let project_dir = dir.projects_dir().join("test-project");
    fs::create_dir_all(&project_dir).unwrap();

    let warnings = dir.validate_structure().unwrap();
    assert!(warnings
        .iter()
        .any(|w| w.contains("Project") && w.contains("missing settings.json")));
}

#[test]
fn test_initialized_structure_matches_real_claude() {
    let mut dir = StateDirectory::temp().unwrap();
    dir.initialize().unwrap();

    // Verify structure matches real ~/.claude
    assert!(dir.settings_path().exists(), "settings.json should exist");
    assert!(dir.projects_dir().exists(), "projects/ should exist");
    assert!(dir.todos_dir().exists(), "todos/ should exist");

    // Validate and ensure no warnings
    let warnings = dir.validate_structure().unwrap();
    assert!(warnings.is_empty(), "Warnings: {:?}", warnings);
}

#[test]
fn test_settings_json_format() {
    let mut dir = StateDirectory::temp().unwrap();
    dir.initialize().unwrap();

    let settings_content = fs::read_to_string(dir.settings_path()).unwrap();
    let settings: serde_json::Value = serde_json::from_str(&settings_content).unwrap();

    // Real Claude settings.json is a valid JSON object (minimum is {})
    assert!(settings.is_object());
}
