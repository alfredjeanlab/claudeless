# Implementation Plan: Claudeless-Only Version Display

**Root Feature:** `cl-fcbd`

## Overview

When no Claude Code version is specified in the scenario file or as a CLI argument, Claudeless should display its own version string ("Claudeless 0.1.0") in the TUI header instead of "Claude Code vX.Y.Z". This allows users to distinguish between simulating a specific Claude Code version versus running Claudeless in its native mode.

## Project Structure

Files to modify:

```
crates/cli/src/
├── cli.rs              # Add optional --claude-version CLI argument
├── config.rs           # Already has optional claude_version field ✓
├── tui/
│   └── app.rs          # Add version_display field to TuiConfig
│                       # Update format_header_lines() rendering
└── state/
    └── mod.rs          # Update JSONL version field handling (optional)
```

## Dependencies

No new external dependencies required. Uses existing:
- `env!("CARGO_PKG_VERSION")` for claudeless version ("0.1.0")
- Existing `ScenarioConfig.claude_version: Option<String>`

## Implementation Phases

### Phase 1: Add CLI Flag for Claude Version Override

Add `--claude-version` flag to allow explicit version override from command line.

**File:** `crates/cli/src/cli.rs`

```rust
/// Claude version to simulate (e.g., "2.1.12")
/// When not set, displays "Claudeless" branding instead of "Claude Code"
#[arg(long)]
pub claude_version: Option<String>,
```

**Verification:** `cargo build` passes; `claudeless --help` shows new flag.

---

### Phase 2: Add Version Display Mode to TuiConfig

Track whether a claude version was explicitly specified, enabling conditional header rendering.

**File:** `crates/cli/src/tui/app.rs`

Add new field to `TuiConfig`:

```rust
pub struct TuiConfig {
    pub trusted: bool,
    pub user_name: String,
    pub model: String,
    pub working_directory: PathBuf,
    pub permission_mode: PermissionMode,
    pub compact_delay_ms: Option<u64>,
    /// Explicit Claude version, or None for Claudeless-native mode
    pub claude_version: Option<String>,
}
```

Update `TuiConfig::from_scenario()` to accept CLI version override:

```rust
pub fn from_scenario(
    config: &ScenarioConfig,
    cli_model: Option<&str>,
    cli_permission_mode: &PermissionMode,
    cli_claude_version: Option<&str>,  // New parameter
) -> Self {
    // CLI overrides scenario
    let claude_version = cli_claude_version
        .map(|s| s.to_string())
        .or_else(|| config.claude_version.clone());

    Self {
        // ... existing fields ...
        claude_version,
    }
}
```

Update `Default for TuiConfig`:

```rust
impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            claude_version: None,  // Claudeless-native mode by default
        }
    }
}
```

**Verification:** Unit tests pass; `TuiConfig::from_scenario()` correctly merges CLI and scenario values.

---

### Phase 3: Update TUI Header Rendering

Modify `format_header_lines()` to conditionally display Claudeless or Claude Code branding.

**File:** `crates/cli/src/tui/app.rs` (around line 1586)

Current:
```rust
let line1 = format!(" ▐▛███▜▌   Claude Code v{}", env!("CARGO_PKG_VERSION"));
```

New:
```rust
let line1 = match &state.config.claude_version {
    Some(version) => format!(" ▐▛███▜▌   Claude Code v{}", version),
    None => format!(" ▐▛███▜▌   Claudeless {}", env!("CARGO_PKG_VERSION")),
};
```

**Verification:** TUI displays correct header based on configuration.

---

### Phase 4: Update Call Sites

Update all places that construct `TuiConfig::from_scenario()` to pass the new parameter.

**File:** `crates/cli/src/main.rs` (around line 346)

```rust
let tui_config = TuiConfig::from_scenario(
    scenario.config(),
    Some(&cli.model),
    &cli.permission_mode,
    cli.claude_version.as_deref(),  // Pass CLI version
);
```

**Verification:** Integration works end-to-end.

---

### Phase 5: Add Tests

Add unit tests for the new behavior.

**File:** `crates/cli/src/tui/app_tests.rs`

```rust
#[test]
fn header_shows_claudeless_when_no_version_specified() {
    let config = TuiConfig::default();
    assert!(config.claude_version.is_none());
    // Verify header rendering would show "Claudeless X.Y.Z"
}

#[test]
fn header_shows_claude_code_when_version_specified() {
    let mut config = TuiConfig::default();
    config.claude_version = Some("2.1.12".to_string());
    // Verify header rendering would show "Claude Code v2.1.12"
}

#[test]
fn cli_version_overrides_scenario() {
    let mut scenario_config = ScenarioConfig::default();
    scenario_config.claude_version = Some("1.0.0".to_string());

    let tui_config = TuiConfig::from_scenario(
        &scenario_config,
        None,
        &PermissionMode::Default,
        Some("2.0.0"),  // CLI override
    );

    assert_eq!(tui_config.claude_version, Some("2.0.0".to_string()));
}
```

**Verification:** `cargo test` passes all new tests.

---

### Phase 6 (Optional): Update JSONL State Files

Consider whether JSONL session files should also reflect this distinction. Currently `state/mod.rs` uses `env!("CARGO_PKG_VERSION")` for all version fields.

Options:
1. Keep JSONL as-is (always shows claudeless version) - simpler
2. Make JSONL reflect the simulated version when claude_version is set

Recommend option 1 for now as JSONL is internal state, not user-facing display.

**No changes required for this phase.**

## Key Implementation Details

### Version Precedence

1. CLI `--claude-version` flag (highest priority)
2. Scenario file `claude_version` field
3. None → Claudeless-native mode (displays "Claudeless X.Y.Z")

### Backward Compatibility

- Existing scenario files with `claude_version` set continue to work unchanged
- Existing scenario files without `claude_version` now show Claudeless branding (behavior change, but desired)
- `DEFAULT_CLAUDE_VERSION` constant can remain for other uses but is no longer used as fallback for display

### Unicode Logo

The header uses the same ASCII art logo for both modes:
```
 ▐▛███▜▌   Claude Code v2.1.12    (when version specified)
 ▐▛███▜▌   Claudeless 0.1.0       (when no version specified)
```

## Verification Plan

1. **Unit tests:** Run `cargo test --all`
2. **Manual TUI test (no scenario):**
   ```bash
   cargo run -- --tui
   # Header should show "Claudeless 0.1.0"
   ```
3. **Manual TUI test (with scenario version):**
   ```bash
   # Create scenario with claude_version = "2.1.12"
   cargo run -- --tui --scenario path/to/scenario.toml
   # Header should show "Claude Code v2.1.12"
   ```
4. **Manual TUI test (with CLI override):**
   ```bash
   cargo run -- --tui --claude-version 3.0.0
   # Header should show "Claude Code v3.0.0"
   ```
5. **Full check:** Run `make check` to ensure all lints/tests pass
