// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Simulated `~/.claude` directory structure.

use sha2::{Digest, Sha256};
use std::fs::{self, Permissions};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("State directory not initialized")]
    NotInitialized,

    #[error("Invalid project path: {0}")]
    InvalidProject(String),
}

/// Simulated ~/.claude directory structure
pub struct StateDirectory {
    /// Root directory (typically a temp dir in tests)
    root: PathBuf,

    /// Whether the directory has been initialized
    initialized: bool,
}

impl StateDirectory {
    /// Create a new state directory at the given root
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            initialized: false,
        }
    }

    /// Create a state directory in a temporary location
    pub fn temp() -> std::io::Result<Self> {
        let temp = tempfile::tempdir()?;
        let path = temp.keep();
        Ok(Self {
            root: path,
            initialized: false,
        })
    }

    /// Resolve state directory from environment or default to a temp directory.
    ///
    /// # Safety
    ///
    /// This method deliberately defaults to a temporary directory rather than `~/.claude`
    /// to prevent the simulator from accidentally modifying real Claude Code state.
    /// To use a specific directory, set the `CLAUDELESS_STATE_DIR` environment variable.
    pub fn resolve() -> std::io::Result<Self> {
        if let Ok(dir) = std::env::var("CLAUDELESS_STATE_DIR") {
            Ok(Self::new(PathBuf::from(dir)))
        } else {
            // Default to temp directory to avoid touching real ~/.claude
            Self::temp()
        }
    }

    /// Initialize the directory structure
    pub fn initialize(&mut self) -> Result<(), StateError> {
        // Create main directories
        fs::create_dir_all(self.todos_dir())?;
        fs::create_dir_all(self.projects_dir())?;
        fs::create_dir_all(self.plans_dir())?;
        fs::create_dir_all(self.sessions_dir())?;

        // Create settings file with defaults
        if !self.settings_path().exists() {
            fs::write(self.settings_path(), "{}")?;
        }

        // Set permissions (readable/writable by user only)
        self.set_permissions(&self.root, 0o700)?;

        self.initialized = true;
        Ok(())
    }

    /// Check if the directory has been initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the root directory path
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the todos directory path
    pub fn todos_dir(&self) -> PathBuf {
        self.root.join("todos")
    }

    /// Get the projects directory path
    pub fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    /// Get the plans directory path
    pub fn plans_dir(&self) -> PathBuf {
        self.root.join("plans")
    }

    /// Get the sessions directory path
    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    /// Get the settings file path
    pub fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    /// Get a settings loader for this state directory and working directory.
    ///
    /// The loader will search for settings files in:
    /// 1. Global: {state_dir}/settings.json
    /// 2. Project: {working_dir}/.claude/settings.json
    /// 3. Local: {working_dir}/.claude/settings.local.json
    pub fn settings_loader(&self, working_dir: &Path) -> super::settings_loader::SettingsLoader {
        let paths = super::settings_loader::SettingsPaths::resolve(&self.root, working_dir);
        super::settings_loader::SettingsLoader::new(paths)
    }

    /// Get the project directory for a given project path.
    ///
    /// Uses the same path normalization as the real Claude CLI:
    /// `/Users/foo/project` → `~/.claude/projects/-Users-foo-project`
    pub fn project_dir(&self, project_path: &Path) -> PathBuf {
        let dir_name = project_dir_name(project_path);
        self.projects_dir().join(&dir_name)
    }

    /// Get the session file path for a given session ID
    pub fn session_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir().join(format!("{}.json", session_id))
    }

    /// Get the todo file path for a given session/context
    pub fn todo_path(&self, context: &str) -> PathBuf {
        self.todos_dir().join(format!("{}.json", context))
    }

    /// Reset state to clean slate
    pub fn reset(&mut self) -> Result<(), StateError> {
        // Remove all contents but keep structure
        if self.todos_dir().exists() {
            for entry in fs::read_dir(self.todos_dir())? {
                fs::remove_file(entry?.path())?;
            }
        }

        if self.projects_dir().exists() {
            fs::remove_dir_all(self.projects_dir())?;
            fs::create_dir_all(self.projects_dir())?;
        }

        if self.plans_dir().exists() {
            for entry in fs::read_dir(self.plans_dir())? {
                fs::remove_file(entry?.path())?;
            }
        }

        if self.sessions_dir().exists() {
            for entry in fs::read_dir(self.sessions_dir())? {
                fs::remove_file(entry?.path())?;
            }
        }

        // Reset settings to defaults
        fs::write(self.settings_path(), "{}")?;

        Ok(())
    }

    fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), StateError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = Permissions::from_mode(mode);
            fs::set_permissions(path, perms)?;
        }
        #[cfg(not(unix))]
        {
            let _ = (path, mode);
        }
        Ok(())
    }

    /// Validate directory structure matches expected layout
    ///
    /// Returns a list of warnings about any structural issues found.
    /// An empty list indicates the structure matches expectations.
    pub fn validate_structure(&self) -> Result<Vec<String>, StateError> {
        let mut warnings = Vec::new();

        // Check required directories
        let required_dirs = ["projects", "todos"];
        for dir in required_dirs {
            let path = self.root.join(dir);
            if !path.exists() {
                warnings.push(format!("Missing directory: {}", dir));
            }
        }

        // Check settings.json exists and is valid JSON
        let settings_path = self.settings_path();
        if settings_path.exists() {
            match fs::read_to_string(&settings_path) {
                Ok(content) => {
                    if let Err(e) = serde_json::from_str::<serde_json::Value>(&content) {
                        warnings.push(format!("Invalid settings.json: {}", e));
                    }
                }
                Err(e) => {
                    warnings.push(format!("Cannot read settings.json: {}", e));
                }
            }
        } else {
            warnings.push("Missing settings.json".to_string());
        }

        // Check project directories have correct structure
        let projects_dir = self.projects_dir();
        if projects_dir.exists() {
            if let Ok(entries) = fs::read_dir(&projects_dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let project_settings = entry.path().join("settings.json");
                        if !project_settings.exists() {
                            warnings.push(format!(
                                "Project {} missing settings.json",
                                entry.file_name().to_string_lossy()
                            ));
                        }
                    }
                }
            }
        }

        Ok(warnings)
    }
}

/// Generate a deterministic hash for a project path (deprecated, use normalize_project_path)
pub fn project_hash(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8]) // First 8 bytes = 16 hex chars
}

/// Normalize a project path to match Claude CLI's directory naming convention.
///
/// Real Claude CLI converts paths like `/Users/user/Developer/myproject` to
/// `-Users-user-Developer-myproject` for the projects directory.
///
/// The normalization rules are:
/// 1. Replace all `/` characters with `-`
/// 2. Replace all `.` characters with `-`
/// 3. This results in a leading `-` for absolute paths
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use claudeless::state::directory::normalize_project_path;
///
/// assert_eq!(
///     normalize_project_path(Path::new("/Users/user/Developer/myproject")),
///     "-Users-user-Developer-myproject"
/// );
///
/// assert_eq!(
///     normalize_project_path(Path::new("/tmp/test.txt")),
///     "-tmp-test-txt"
/// );
/// ```
pub fn normalize_project_path(path: &Path) -> String {
    path.to_string_lossy().replace(['/', '.'], "-")
}

/// Get the canonical project directory name for a path.
///
/// This tries to canonicalize the path first (resolving symlinks, etc.)
/// and falls back to the original path if canonicalization fails.
pub fn project_dir_name(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    normalize_project_path(&canonical)
}

#[cfg(test)]
mod tests {
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
    fn test_project_hash_deterministic() {
        let path = Path::new("/some/project/path");
        let hash1 = project_hash(path);
        let hash2 = project_hash(path);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16); // 8 bytes = 16 hex chars
    }

    #[test]
    fn test_project_hash_different_paths() {
        let hash1 = project_hash(Path::new("/path/a"));
        let hash2 = project_hash(Path::new("/path/b"));
        assert_ne!(hash1, hash2);
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
    // The only way to use a real path is to explicitly set CLAUDELESS_STATE_DIR.

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
        let is_temp_path = root_str.contains("tmp")
            || root_str.contains("temp")
            || root_str.contains("var/folders"); // macOS temp location

        assert!(
            is_temp_path,
            "Expected resolve() to return a temp directory path, got: {}",
            root_str
        );
    }

    #[test]
    fn test_resolve_respects_env_var_override() {
        // When CLAUDELESS_STATE_DIR is explicitly set, resolve() should use it.
        // This allows users to intentionally specify a directory for testing.

        let test_dir = tempfile::tempdir().unwrap();
        let test_path = test_dir.path().join("custom-claudeless");

        std::env::set_var("CLAUDELESS_STATE_DIR", &test_path);

        let dir = StateDirectory::resolve().unwrap();
        assert_eq!(
            dir.root(),
            test_path.as_path(),
            "resolve() should use CLAUDELESS_STATE_DIR when set"
        );

        // Clean up env var
        std::env::remove_var("CLAUDELESS_STATE_DIR");
    }

    #[test]
    fn test_resolve_with_env_var_does_not_require_existing_dir() {
        // The directory specified by CLAUDELESS_STATE_DIR doesn't need to exist
        // yet - it will be created when initialize() is called.
        //
        // Use a unique temp dir to avoid race conditions with other tests
        // that also set CLAUDELESS_STATE_DIR.
        let temp = tempfile::tempdir().unwrap();
        let non_existent = temp.path().join("claudeless-test-nonexistent");
        assert!(!non_existent.exists());

        std::env::set_var("CLAUDELESS_STATE_DIR", &non_existent);

        let dir = StateDirectory::resolve().unwrap();
        assert_eq!(dir.root(), non_existent.as_path());
        assert!(!dir.is_initialized());

        // Clean up
        std::env::remove_var("CLAUDELESS_STATE_DIR");
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
}
