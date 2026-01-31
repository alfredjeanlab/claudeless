# Implementation Plan: --setting-sources CLI Argument

## Overview

Add a `--setting-sources` CLI argument that allows users to specify which settings sources to load. This enables selective loading of user (global), project, and local settings files, providing more control over configuration precedence in testing and CI/CD scenarios.

## Project Structure

```
crates/cli/src/
├── cli.rs                    # Add --setting-sources argument
├── state/
│   ├── mod.rs                # Export SettingSource enum
│   ├── settings_source.rs    # New: SettingSource enum definition
│   ├── settings_loader.rs    # Modify to accept source filter
│   └── directory.rs          # Update settings_loader() method signature
└── session/
    └── context.rs            # Thread sources through to loader
```

## Dependencies

No new external dependencies required. Uses existing:
- `clap` for CLI argument parsing
- Standard library types

## Implementation Phases

### Phase 1: Define SettingSource Enum

**Files:** `crates/cli/src/state/settings_source.rs` (new), `crates/cli/src/state/mod.rs`

Create a new enum representing the available settings sources:

```rust
// settings_source.rs
use clap::ValueEnum;
use std::str::FromStr;

/// Available settings sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum SettingSource {
    /// Global/user settings (~/.claude/settings.json)
    User,
    /// Project settings (.claude/settings.json)
    Project,
    /// Local overrides (.claude/settings.local.json)
    Local,
}

impl SettingSource {
    /// Return all sources in precedence order (lowest to highest).
    pub fn all() -> &'static [SettingSource] {
        &[Self::User, Self::Project, Self::Local]
    }
}

impl FromStr for SettingSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" | "global" => Ok(Self::User),
            "project" => Ok(Self::Project),
            "local" => Ok(Self::Local),
            _ => Err(format!("unknown setting source: {}", s)),
        }
    }
}
```

Export from `mod.rs`:
```rust
pub mod settings_source;
pub use settings_source::SettingSource;
```

**Verification:** `cargo check` passes, unit tests for `FromStr` implementation.

---

### Phase 2: Add CLI Argument

**File:** `crates/cli/src/cli.rs`

Add the `--setting-sources` argument to the `Cli` struct:

```rust
/// Comma-separated list of setting sources to load (user, project, local).
/// When not specified, all sources are loaded.
#[arg(long = "setting-sources", value_delimiter = ',')]
pub setting_sources: Option<Vec<SettingSource>>,
```

**Verification:**
- `cargo test cli_tests` passes
- `./target/debug/claudeless --help` shows the new argument
- Argument parsing works: `--setting-sources user,local`

---

### Phase 3: Update SettingsPaths and SettingsLoader

**File:** `crates/cli/src/state/settings_loader.rs`

Modify `SettingsPaths::resolve()` to accept a source filter:

```rust
impl SettingsPaths {
    /// Resolve settings paths for a given working directory, optionally filtering by sources.
    ///
    /// # Arguments
    /// * `state_dir` - The ~/.claude equivalent
    /// * `working_dir` - The project working directory
    /// * `sources` - Optional list of sources to include. If None, all sources are included.
    pub fn resolve_with_sources(
        state_dir: &Path,
        working_dir: &Path,
        sources: Option<&[SettingSource]>,
    ) -> Self {
        let include_all = sources.is_none();
        let sources = sources.unwrap_or(SettingSource::all());

        Self {
            global: sources.contains(&SettingSource::User)
                .then(|| state_dir.join("settings.json")),
            project: sources.contains(&SettingSource::Project)
                .then(|| working_dir.join(".claude").join("settings.json")),
            local: sources.contains(&SettingSource::Local)
                .then(|| working_dir.join(".claude").join("settings.local.json")),
        }
    }

    /// Resolve all settings paths (existing behavior, delegates to resolve_with_sources).
    pub fn resolve(state_dir: &Path, working_dir: &Path) -> Self {
        Self::resolve_with_sources(state_dir, working_dir, None)
    }
}
```

**Verification:**
- Existing `SettingsPaths::resolve()` behavior unchanged
- Unit tests for `resolve_with_sources()` with various source combinations

---

### Phase 4: Update StateDirectory

**File:** `crates/cli/src/state/directory.rs`

Update the `settings_loader()` method to accept optional sources:

```rust
impl StateDirectory {
    /// Get a settings loader for this state directory and working directory.
    ///
    /// # Arguments
    /// * `working_dir` - The project working directory
    /// * `sources` - Optional list of sources to include. If None, all sources are loaded.
    pub fn settings_loader_with_sources(
        &self,
        working_dir: &Path,
        sources: Option<&[SettingSource]>,
    ) -> super::settings_loader::SettingsLoader {
        let paths = super::settings_loader::SettingsPaths::resolve_with_sources(
            &self.root,
            working_dir,
            sources,
        );
        super::settings_loader::SettingsLoader::new(paths)
    }

    /// Get a settings loader that loads all sources (existing behavior).
    pub fn settings_loader(&self, working_dir: &Path) -> super::settings_loader::SettingsLoader {
        self.settings_loader_with_sources(working_dir, None)
    }
}
```

**Verification:** Existing `settings_loader()` behavior unchanged.

---

### Phase 5: Thread Through SessionContext

**File:** `crates/cli/src/session/context.rs`

Update `build_with_state()` to accept and pass sources:

```rust
impl SessionContext {
    /// Build context with settings loaded from state directory.
    ///
    /// Loads settings from sources specified in CLI args, or all sources if not specified.
    pub fn build_with_state(
        scenario: Option<&ScenarioConfig>,
        cli: &Cli,
        state_dir: &StateDirectory,
    ) -> Self {
        // ... existing working_directory resolution ...

        // Load and merge settings with source filtering
        let sources = cli.setting_sources.as_deref();
        let loader = state_dir.settings_loader_with_sources(&working_directory, sources);
        let effective_settings = loader.load();

        Self::build_internal(scenario, cli, effective_settings)
    }
}
```

**Verification:** End-to-end test with `--setting-sources` argument.

---

### Phase 6: Integration Tests

**File:** `crates/cli/tests/settings_loading.rs`

Add tests for source filtering:

```rust
#[test]
fn test_setting_sources_user_only() {
    // Setup: user, project, and local settings with distinct values
    // CLI: --setting-sources user
    // Verify: only user settings loaded
}

#[test]
fn test_setting_sources_project_and_local() {
    // Setup: all three sources
    // CLI: --setting-sources project,local
    // Verify: user settings excluded, project and local merged
}

#[test]
fn test_setting_sources_empty_means_all() {
    // Verify: no --setting-sources loads all sources (backward compatible)
}

#[test]
fn test_setting_sources_order_independent() {
    // CLI: --setting-sources local,user,project
    // Verify: precedence still user < project < local regardless of arg order
}
```

**Verification:** `cargo test settings_loading` passes.

## Key Implementation Details

### Source Names

- `user` (alias: `global`) - Maps to `~/.claude/settings.json` or state dir equivalent
- `project` - Maps to `.claude/settings.json` in working directory
- `local` - Maps to `.claude/settings.local.json` in working directory

### Precedence

Source precedence remains fixed regardless of `--setting-sources` argument order:
1. user (lowest)
2. project
3. local (highest)

The argument only controls which sources are **included**, not their relative priority.

### Backward Compatibility

- When `--setting-sources` is not specified, all sources are loaded (existing behavior)
- All existing tests should continue to pass unchanged

### Error Handling

- Unknown source names result in clap parsing error with helpful message
- Empty `--setting-sources` value (e.g., `--setting-sources ""`) is treated as no sources

## Verification Plan

1. **Unit Tests:**
   - `SettingSource::from_str()` parsing
   - `SettingsPaths::resolve_with_sources()` with various source combinations

2. **Integration Tests:**
   - Source filtering works correctly
   - Precedence maintained
   - Backward compatibility verified

3. **Manual Testing:**
   ```bash
   # Load only user settings
   claudeless --setting-sources user -p "test"

   # Load project and local only (skip user)
   claudeless --setting-sources project,local -p "test"

   # Verify help text
   claudeless --help | grep setting-sources
   ```

4. **Run Full Test Suite:**
   ```bash
   make check
   ```
