# Epic 10: Settings File Support

## Overview

Implement support for `settings.json` and `settings.local.json` files that configure Claude Code behavior. The simulator reads and respects these settings, enabling tests that verify oj's integration with Claude's configuration system.

Claude Code reads settings from three locations with a defined merge order:
1. Global: `~/.claude/settings.json`
2. Project: `.claude/settings.json` (in working directory)
3. Local: `.claude/settings.local.json` (gitignored, user-specific)

Settings control allowed/denied tools, permission behaviors, additional directories, and environment variables. The existing `Settings` struct is a generic key-value store; this epic adds a `PermissionSettings` struct with proper schema parsing and integrates it with the permission checker.

**What's in this epic:**
- Global, project, and local settings file loading
- Settings merge with precedence (global < project < local)
- `permissions.allow`/`deny` pattern matching for auto-approve/reject
- `permissions.additionalDirectories` parsing
- `mcpServers` and `env` parsing (config only, no spawning)
- Integration with `PermissionChecker` for settings-based auto-approve
- Scenario `tool_execution.tools` takes precedence over settings
- Settings inspection API for tests

**What's NOT in this epic:**
- MCP server spawning (just parse config)
- Settings file watching/hot-reload
- Settings UI/editing commands
- Full settings schema validation (permissive parsing)

## Project Structure

```
crates/cli/
├── src/
│   ├── state/
│   │   ├── mod.rs                    # UPDATE: Export new types
│   │   ├── settings.rs               # UPDATE: Add PermissionSettings
│   │   └── settings_loader.rs        # NEW: Multi-file settings loading
│   ├── permission/
│   │   ├── mod.rs                    # UPDATE: Export new types
│   │   ├── check.rs                  # UPDATE: Integrate settings patterns
│   │   └── pattern.rs                # NEW: Tool pattern matching
│   └── session/
│       └── context.rs                # UPDATE: Hold effective settings
├── tests/
│   ├── settings_loading.rs           # NEW: Settings file tests
│   ├── settings_permissions.rs       # NEW: Permission pattern tests
│   └── common/mod.rs                 # UPDATE: Settings test helpers
```

## Dependencies

No new external dependencies. Uses existing:
- `serde`/`serde_json` for settings parsing
- `glob` for tool pattern matching (already in Cargo.toml)
- `regex` for complex patterns (already in Cargo.toml)

---

## Phase 1: PermissionSettings Schema

**Goal**: Define the settings schema that matches real Claude Code's `settings.json` format.

**New types** (`src/state/settings.rs`):

```rust
/// Claude Code permission settings schema.
///
/// Matches the structure of `permissions` in settings.json:
/// ```json
/// {
///   "permissions": {
///     "allow": ["Bash(npm test)", "Read"],
///     "deny": ["Bash(rm *)"],
///     "additionalDirectories": ["/tmp/workspace"]
///   }
/// }
/// ```
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionSettings {
    #[serde(default)] pub allow: Vec<String>,
    #[serde(default)] pub deny: Vec<String>,
    #[serde(default)] pub additional_directories: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    #[serde(default)] pub command: Option<String>,
    #[serde(default)] pub args: Vec<String>,
    #[serde(default)] pub env: HashMap<String, String>,
    #[serde(default)] pub cwd: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSettings {
    #[serde(default)] pub permissions: PermissionSettings,
    #[serde(default)] pub mcp_servers: HashMap<String, McpServerConfig>,
    #[serde(default)] pub env: HashMap<String, String>,
    #[serde(flatten)] pub extra: HashMap<String, serde_json::Value>,
}

impl ClaudeSettings {
    pub fn load(path: &Path) -> std::io::Result<Self> { /* read file, parse JSON */ }

    /// Arrays replaced if non-empty; maps merged with later values winning.
    pub fn merge(&mut self, other: Self) { /* merge permissions, mcp_servers, env, extra */ }
}
```

**Update existing Settings** to coexist:

```rust
impl Settings {
    /// Try to parse as ClaudeSettings schema.
    pub fn as_claude_settings(&self) -> Option<ClaudeSettings> {
        // Convert the generic HashMap to the typed schema
        let json = serde_json::to_value(&self.values).ok()?;
        serde_json::from_value(json).ok()
    }
}
```

**Verification**:
- `ClaudeSettings` can parse valid settings.json
- Unknown fields are preserved in `extra`
- `merge()` correctly combines settings
- Unit tests for all schema fields

---

## Phase 2: Settings Loader

**Goal**: Implement multi-file settings loading with correct precedence.

**New module** (`src/state/settings_loader.rs`):

```rust
use std::path::{Path, PathBuf};
use super::settings::ClaudeSettings;

/// Paths to search for settings files.
#[derive(Clone, Debug)]
pub struct SettingsPaths {
    /// Global settings (~/.claude/settings.json)
    pub global: Option<PathBuf>,
    /// Project settings (.claude/settings.json)
    pub project: Option<PathBuf>,
    /// Local overrides (.claude/settings.local.json)
    pub local: Option<PathBuf>,
}

impl SettingsPaths {
    /// Resolve settings paths for a given working directory.
    ///
    /// # Arguments
    /// * `state_dir` - The ~/.claude equivalent (or CLAUDELESS_STATE_DIR)
    /// * `working_dir` - The project working directory
    pub fn resolve(state_dir: &Path, working_dir: &Path) -> Self {
        Self {
            global: Some(state_dir.join("settings.json")),
            project: Some(working_dir.join(".claude").join("settings.json")),
            local: Some(working_dir.join(".claude").join("settings.local.json")),
        }
    }

    /// Create paths for testing (no global).
    pub fn project_only(working_dir: &Path) -> Self {
        Self {
            global: None,
            project: Some(working_dir.join(".claude").join("settings.json")),
            local: Some(working_dir.join(".claude").join("settings.local.json")),
        }
    }
}

/// Loads and merges settings from multiple files.
pub struct SettingsLoader {
    paths: SettingsPaths,
}

impl SettingsLoader {
    pub fn new(paths: SettingsPaths) -> Self {
        Self { paths }
    }

    /// Load and merge all settings files.
    ///
    /// Precedence (later overrides earlier):
    /// 1. Global (~/.claude/settings.json)
    /// 2. Project (.claude/settings.json)
    /// 3. Local (.claude/settings.local.json)
    ///
    /// Missing files are silently skipped.
    pub fn load(&self) -> ClaudeSettings {
        // For each path in [global, project, local]: load if exists, merge, warn on error
    }

    /// Check which settings files exist.
    pub fn existing_files(&self) -> Vec<&Path> { /* filter existing paths */ }
}
```

**Update StateDirectory** (`src/state/directory.rs`):

```rust
impl StateDirectory {
    /// Get a settings loader for this state directory and working directory.
    pub fn settings_loader(&self, working_dir: &Path) -> SettingsLoader {
        let paths = SettingsPaths::resolve(self.root(), working_dir);
        SettingsLoader::new(paths)
    }
}
```

**Verification**:
- Settings loaded from all three locations
- Merge precedence is correct (local wins)
- Missing files don't cause errors
- Invalid JSON logs warning but continues

---

## Phase 3: Tool Pattern Matching

**Goal**: Implement pattern matching for `permissions.allow` and `permissions.deny`.

Claude Code uses patterns like:
- `"Read"` - matches all Read tool calls
- `"Bash(npm test)"` - matches Bash with specific command
- `"Bash(npm *)"` - glob pattern for command
- `"Edit"` - matches all Edit tool calls

**New module** (`src/permission/pattern.rs`):

```rust
use glob::Pattern;

/// A compiled tool permission pattern.
#[derive(Clone, Debug)]
pub struct ToolPattern {
    /// The tool name (e.g., "Bash", "Read", "Edit")
    pub tool: String,
    /// Optional argument pattern (e.g., "npm test", "npm *")
    pub argument: Option<CompiledPattern>,
}

#[derive(Clone, Debug)]
pub enum CompiledPattern {
    /// Exact string match
    Exact(String),
    /// Glob pattern
    Glob(Pattern),
}

impl ToolPattern {
    /// Parse a pattern string like "Bash(npm test)" or "Read".
    pub fn parse(s: &str) -> Option<Self> {
        // Parse "Tool(arg)" or "Tool" format
        // Arg containing *, ?, [ -> Glob pattern, else Exact
    }

    /// Check if this pattern matches a tool call.
    ///
    /// # Arguments
    /// * `tool_name` - The tool being called (e.g., "Bash")
    /// * `tool_input` - The tool input as a string representation
    pub fn matches(&self, tool_name: &str, tool_input: Option<&str>) -> bool {
        // Tool name match (case-insensitive), then check argument pattern
    }
}

/// A collection of allow/deny patterns for permission checking.
#[derive(Clone, Debug, Default)]
pub struct PermissionPatterns {
    pub allow: Vec<ToolPattern>,
    pub deny: Vec<ToolPattern>,
}

impl PermissionPatterns {
    /// Create from permission settings.
    pub fn from_settings(settings: &PermissionSettings) -> Self {
        Self {
            allow: settings.allow.iter().filter_map(|s| ToolPattern::parse(s)).collect(),
            deny: settings.deny.iter().filter_map(|s| ToolPattern::parse(s)).collect(),
        }
    }

    /// Check if a tool call is explicitly allowed by settings.
    pub fn is_allowed(&self, tool: &str, input: Option<&str>) -> bool {
        self.allow.iter().any(|p| p.matches(tool, input))
    }

    /// Check if a tool call is explicitly denied by settings.
    pub fn is_denied(&self, tool: &str, input: Option<&str>) -> bool {
        self.deny.iter().any(|p| p.matches(tool, input))
    }
}
```

**Verification**:
- Parse "Tool" patterns correctly
- Parse "Tool(arg)" patterns correctly
- Parse "Tool(glob*)" patterns correctly
- Match tool name case-insensitively
- Glob patterns work for arguments
- Invalid patterns handled gracefully

---

## Phase 4: Permission Checker Integration

**Goal**: Integrate settings-based patterns with `PermissionChecker`.

**Update PermissionChecker** (`src/permission/check.rs`):

```rust
use super::pattern::PermissionPatterns;

/// Permission checker for tool execution.
pub struct PermissionChecker {
    mode: PermissionMode,
    bypass: PermissionBypass,
    /// Patterns from settings files
    settings_patterns: PermissionPatterns,
    /// Per-tool overrides from scenario (highest priority)
    scenario_overrides: HashMap<String, ToolConfig>,
}

impl PermissionChecker {
    /// Create with mode, bypass, and settings patterns.
    pub fn new(
        mode: PermissionMode,
        bypass: PermissionBypass,
        settings_patterns: PermissionPatterns,
    ) -> Self {
        Self {
            mode,
            bypass,
            settings_patterns,
            scenario_overrides: HashMap::new(),
        }
    }

    /// Add scenario tool overrides (highest priority).
    pub fn with_scenario_overrides(mut self, overrides: HashMap<String, ToolConfig>) -> Self {
        self.scenario_overrides = overrides;
        self
    }

    /// Check if a tool action is allowed.
    ///
    /// Priority order (highest to lowest):
    /// 1. Bypass flags (--dangerously-skip-permissions)
    /// 2. Scenario tool_execution.tools overrides
    /// 3. Settings permissions.deny (explicit deny)
    /// 4. Settings permissions.allow (auto-approve)
    /// 5. Permission mode (default, plan, accept-edits, etc.)
    pub fn check(&self, tool_name: &str, action: &str, tool_input: Option<&str>) -> PermissionResult {
        // Check each level in priority order, return first match
    }

    fn check_by_mode(&self, tool_name: &str, action: &str) -> PermissionResult {
        // BypassPermissions -> Allowed
        // AcceptEdits + edit action -> Allowed
        // DontAsk/Plan -> Denied
        // Default/Delegate -> NeedsPrompt
    }

    /// Get effective settings patterns (for inspection).
    pub fn settings_patterns(&self) -> &PermissionPatterns {
        &self.settings_patterns
    }
}
```

**Update mod.rs exports**:

```rust
pub mod pattern;
pub use pattern::{PermissionPatterns, ToolPattern};
```

**Verification**:
- Scenario overrides beat settings
- Settings deny beats settings allow
- Settings allow auto-approves (no prompt)
- Mode-based fallback works as before
- Backward compatible (empty patterns = old behavior)

---

## Phase 5: Session Context Integration

**Goal**: Wire settings loading into `SessionContext` so the TUI and CLI use them.

**Update SessionContext** (`src/session/context.rs`):

```rust
use crate::state::settings_loader::SettingsLoader;
use crate::state::ClaudeSettings;
use crate::permission::{PermissionPatterns, PermissionChecker};

pub struct SessionContext {
    // ... existing fields ...

    /// Effective settings (merged from all sources)
    effective_settings: ClaudeSettings,

    /// Compiled permission patterns from settings
    permission_patterns: PermissionPatterns,
}

impl SessionContext {
    /// Create session context with settings loading.
    pub fn new(
        scenario_config: &ScenarioConfig,
        state_dir: &StateDirectory,
        working_dir: &Path,
        cli_args: &Cli,
    ) -> Self {
        // Load settings via state_dir.settings_loader(), compile permission patterns
    }

    /// Get effective settings.
    pub fn settings(&self) -> &ClaudeSettings { &self.effective_settings }

    /// Get permission patterns from settings.
    pub fn permission_patterns(&self) -> &PermissionPatterns { &self.permission_patterns }

    /// Create a permission checker with current context.
    pub fn permission_checker(&self, mode: PermissionMode, bypass: PermissionBypass) -> PermissionChecker {
        // Create checker with patterns, add scenario overrides if present
    }

    /// Get environment variables from settings.
    pub fn settings_env(&self) -> &HashMap<String, String> { &self.effective_settings.env }

    /// Get additional directories from settings.
    pub fn additional_directories(&self) -> &[String] { &self.effective_settings.permissions.additional_directories }
}
```

**Update main.rs** to pass settings to permission checking:

```rust
fn run_with_context(ctx: &SessionContext, /* ... */) {
    let checker = ctx.permission_checker(mode, bypass);

    // Use checker for tool execution decisions
    // ...
}
```

**Verification**:
- Settings loaded at session start
- Permission patterns available in context
- Checker created with correct precedence
- Environment variables accessible
- Additional directories accessible

---

## Phase 6: Integration Tests

**Goal**: Comprehensive tests for settings file behavior.

**New test file** (`tests/settings_loading.rs`):

```rust
//! Integration tests for settings file loading and merging.

use claudeless::state::{ClaudeSettings, StateDirectory};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_global_settings_loaded() {
    // Setup: global settings with allow: ["Read"]
    // Assert: settings.permissions.allow == ["Read"]
}

#[test]
fn test_project_settings_override_global() {
    // Setup: global allow: ["Read"], project allow: ["Write"]
    // Assert: settings.permissions.allow == ["Write"]
}

#[test]
fn test_local_settings_highest_priority() {
    // Setup: global allow: ["Read"], project allow: ["Write"], local allow: ["Bash"]
    // Assert: settings.permissions.allow == ["Bash"]
}

#[test]
fn test_env_vars_merged() {
    // Setup: global env: {GLOBAL: "1"}, project env: {PROJECT: "2"}
    // Assert: both GLOBAL and PROJECT present in merged settings
}

#[test]
fn test_invalid_json_skipped_with_warning() {
    // Setup: invalid global JSON, valid project settings
    // Assert: project settings still load successfully
}

#[test]
fn test_missing_files_ok() {
    // Setup: no settings files exist
    // Assert: returns empty defaults without error
}
```

**New test file** (`tests/settings_permissions.rs`):

```rust
//! Integration tests for settings-based permission patterns.

use claudeless::permission::{PermissionPatterns, ToolPattern};
use claudeless::state::PermissionSettings;

#[test]
fn test_tool_pattern_simple() {
    // "Read" matches Read/read (case-insensitive), not Write
}

#[test]
fn test_tool_pattern_with_exact_arg() {
    // "Bash(npm test)" matches only exact "npm test", not "npm install" or None
}

#[test]
fn test_tool_pattern_with_glob() {
    // "Bash(npm *)" matches "npm test", "npm install", not "cargo test"
}

#[test]
fn test_patterns_deny_beats_allow() {
    // allow: ["Bash"], deny: ["Bash(rm *)"]
    // Assert: "echo hello" allowed, "rm -rf /" denied
}

#[test]
fn test_permission_checker_with_settings() {
    // allow: ["Read"], deny: ["Bash(rm *)"]
    // Assert: Read -> Allowed, "rm -rf /" -> Denied, "ls" -> NeedsPrompt
}

#[test]
fn test_scenario_overrides_beat_settings() {
    // Settings deny: ["Bash"], but scenario override auto_approve: true
    // Assert: Bash -> Allowed (scenario wins)
}
```

**Verification**:
- All new tests pass
- `cargo test -p claudeless --test settings_loading`
- `cargo test -p claudeless --test settings_permissions`
- `make check` passes

---

## Key Implementation Details

### Settings File Locations

| File | Location | Purpose |
|------|----------|---------|
| Global | `~/.claude/settings.json` | User-wide defaults |
| Project | `.claude/settings.json` | Project-specific settings (committed) |
| Local | `.claude/settings.local.json` | User-specific overrides (gitignored) |

### Merge Strategy

Arrays (`allow`, `deny`, `additionalDirectories`) are **replaced**, not merged:
```
Global:  {"permissions": {"allow": ["Read", "Glob"]}}
Project: {"permissions": {"allow": ["Write"]}}
Result:  {"permissions": {"allow": ["Write"]}}  // Project replaces global
```

Maps (`mcpServers`, `env`) are **merged** with later values winning:
```
Global:  {"env": {"A": "1", "B": "2"}}
Project: {"env": {"B": "3", "C": "4"}}
Result:  {"env": {"A": "1", "B": "3", "C": "4"}}  // B overridden, C added
```

### Permission Check Priority

```
┌─────────────────────────────────────────────────────┐
│ 1. Bypass flags (--dangerously-skip-permissions)   │  ← Highest
├─────────────────────────────────────────────────────┤
│ 2. Scenario tool_execution.tools overrides         │
├─────────────────────────────────────────────────────┤
│ 3. Settings permissions.deny patterns              │
├─────────────────────────────────────────────────────┤
│ 4. Settings permissions.allow patterns             │
├─────────────────────────────────────────────────────┤
│ 5. Permission mode (default, plan, accept-edits)   │  ← Lowest
└─────────────────────────────────────────────────────┘
```

### Tool Pattern Syntax

```
Pattern           Matches
──────────────────────────────────────────────────
Read              All Read tool calls
Bash(npm test)    Bash with exact command "npm test"
Bash(npm *)       Bash with any command starting with "npm "
Edit              All Edit tool calls
Write(*.md)       Write to any .md file
```

---

## Verification Plan

### Unit Tests

| Module | Key Tests |
|--------|-----------|
| `state/settings.rs` | Schema parsing, merge behavior, type accessors |
| `state/settings_loader.rs` | Multi-file loading, precedence, error handling |
| `permission/pattern.rs` | Pattern parsing, matching, edge cases |
| `permission/check.rs` | Priority order, settings integration |

### Integration Tests

| Test File | Description |
|-----------|-------------|
| `settings_loading.rs` | File loading, merging, missing files |
| `settings_permissions.rs` | Pattern matching, checker integration |

### Test Commands

```bash
# Run all settings tests
cargo test -p claudeless settings

# Run specific test file
cargo test -p claudeless --test settings_loading
cargo test -p claudeless --test settings_permissions

# Full CI check
make check
```

### Manual Verification Checklist

- [ ] Global settings.json loaded from state directory
- [ ] Project .claude/settings.json loaded from working directory
- [ ] Local .claude/settings.local.json loaded (highest priority)
- [ ] Missing settings files don't cause errors
- [ ] Invalid JSON logs warning but continues
- [ ] `permissions.allow` patterns auto-approve matching tools
- [ ] `permissions.deny` patterns reject matching tools
- [ ] Deny beats allow when both match
- [ ] Scenario overrides beat settings
- [ ] Glob patterns work in tool arguments
- [ ] Environment variables merged correctly
- [ ] MCP server configs parsed (not spawned)
- [ ] All existing tests still pass
- [ ] `make check` passes
