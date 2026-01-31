# Implementation Plan: --settings Flag

## Overview

Add a `--settings <file-or-json>` CLI flag that allows users to load additional settings from either a JSON file path or an inline JSON string. This flag integrates with the existing settings loading infrastructure and applies with highest precedence (after global/project/local settings files).

## Project Structure

```
crates/cli/src/
├── cli.rs                      # Add --settings flag definition
├── state/
│   ├── settings.rs             # Add load_from_str() method
│   └── settings_loader.rs      # Add support for additional CLI settings
└── main.rs                     # Integrate settings loading before MCP/context
```

## Dependencies

No new dependencies required. Uses existing:
- `clap` for CLI argument parsing
- `serde_json` for JSON parsing
- `json5` for JSON5 support (comments, trailing commas)

## Implementation Phases

### Phase 1: Add CLI Flag Definition

**File:** `crates/cli/src/cli.rs`

Add the `--settings` flag to the `Cli` struct, following the pattern established by `--mcp-config`:

```rust
/// Load settings from a JSON file or inline JSON string (can be specified multiple times)
#[arg(long, value_name = "FILE_OR_JSON")]
pub settings: Vec<String>,
```

**Verification:** Run `cargo build` and confirm `claude --help` shows the new flag.

### Phase 2: Add Settings Parsing from String

**File:** `crates/cli/src/state/settings.rs`

Add a method to `ClaudeSettings` for parsing from a string, mirroring the pattern from `mcp/config.rs`:

```rust
impl ClaudeSettings {
    /// Parse settings from a JSON/JSON5 string.
    pub fn parse(content: &str) -> Result<Self, std::io::Error> {
        json5::from_str(content)
            .or_else(|_| serde_json::from_str(content))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
```

Add a helper function for file-or-JSON detection:

```rust
/// Load settings from a file path or inline JSON string.
///
/// Determines whether input is a file path or inline JSON based on content:
/// - Starts with `{` -> parse as inline JSON
/// - Otherwise -> treat as file path
pub fn load_settings_input(input: &str) -> Result<ClaudeSettings, std::io::Error> {
    let trimmed = input.trim();
    if trimmed.starts_with('{') {
        ClaudeSettings::parse(trimmed)
    } else {
        ClaudeSettings::load(Path::new(input))
    }
}
```

**Verification:** Add unit tests in `settings_tests.rs`:
- Test parsing inline JSON
- Test parsing JSON5 with comments
- Test file path loading
- Test error handling for invalid JSON

### Phase 3: Extend SettingsLoader with CLI Overrides

**File:** `crates/cli/src/state/settings_loader.rs`

Extend `SettingsLoader` to accept additional settings from CLI:

```rust
impl SettingsLoader {
    /// Load and merge all settings files, plus CLI-provided settings.
    ///
    /// Precedence (later overrides earlier):
    /// 1. Global (~/.claude/settings.json)
    /// 2. Project (.claude/settings.json)
    /// 3. Local (.claude/settings.local.json)
    /// 4. CLI --settings flags (in order specified)
    pub fn load_with_overrides(&self, cli_settings: &[String]) -> ClaudeSettings {
        let mut settings = self.load();

        for input in cli_settings {
            match load_settings_input(input) {
                Ok(cli_settings) => {
                    settings.merge(cli_settings);
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to load settings from '{}': {}",
                        input, e
                    );
                }
            }
        }

        settings
    }
}
```

**Verification:** Add unit tests in `settings_loader_tests.rs`:
- Test CLI settings override file settings
- Test multiple CLI settings merge in order
- Test invalid CLI settings produce warnings but don't fail

### Phase 4: Integrate Settings Loading in main.rs

**File:** `crates/cli/src/main.rs`

Update `main()` to load settings before building session context. The settings should be available for:
1. Permission settings (allow/deny patterns)
2. Additional directories
3. Environment variable overrides

```rust
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Load settings (files + CLI overrides)
    let settings = load_settings(&cli);

    // ... rest of main
}

/// Load settings from all sources with correct precedence.
fn load_settings(cli: &Cli) -> ClaudeSettings {
    let state_dir = /* resolve ~/.claude or CLAUDELESS_STATE_DIR */;
    let working_dir = cli.cwd.as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let paths = SettingsPaths::resolve(&state_dir, &working_dir);
    let loader = SettingsLoader::new(paths);

    loader.load_with_overrides(&cli.settings)
}
```

**Verification:**
- Test with `--settings '{"permissions":{"allow":["Read"]}}'`
- Test with `--settings /path/to/settings.json`
- Verify settings are correctly merged with file-based settings

### Phase 5: Add Integration Tests

**File:** `crates/cli/tests/settings_cli.rs`

Add integration tests for the complete flow:

```rust
#[test]
fn test_settings_flag_inline_json() {
    // Test that --settings '{"permissions":{"allow":["Read"]}}' works
}

#[test]
fn test_settings_flag_file_path() {
    // Test that --settings /path/to/file.json works
}

#[test]
fn test_settings_flag_multiple() {
    // Test multiple --settings flags merge correctly
}

#[test]
fn test_settings_flag_precedence() {
    // Test CLI settings override file-based settings
}

#[test]
fn test_settings_flag_invalid_graceful() {
    // Test invalid settings produce warning but don't fail
}
```

**Verification:** Run `cargo test --all` and confirm all tests pass.

## Key Implementation Details

### File vs JSON Detection

Following the established pattern from `load_mcp_config()`:
- If input starts with `{` after trimming whitespace, parse as inline JSON
- Otherwise, treat as a file path

This is simple and reliable - JSON objects must start with `{`, and file paths rarely do.

### JSON5 Support

Use JSON5 parsing first (via `json5` crate) to support:
- Comments (`// ...` and `/* ... */`)
- Trailing commas
- Unquoted object keys

Fall back to strict JSON if JSON5 parsing fails.

### Error Handling

- Invalid settings produce a warning message to stderr
- Parsing continues with remaining settings
- This matches the behavior of file-based settings loading
- Consider adding `--strict-settings` flag (future enhancement) similar to `--strict-mcp-config`

### Merge Semantics

Uses the existing `ClaudeSettings::merge()` method:
- Permission arrays are replaced (not concatenated) if non-empty
- HashMap fields (mcp_servers, env, extra) are merged with override semantics
- CLI-provided settings have highest precedence

### Example Usage

```bash
# Inline JSON
claude --settings '{"permissions":{"allow":["Read","Bash(npm *)"]}}' -p "hello"

# File path
claude --settings ./custom-settings.json -p "hello"

# Multiple sources (later overrides earlier)
claude --settings ./base.json --settings '{"env":{"DEBUG":"1"}}' -p "hello"
```

## Verification Plan

1. **Unit Tests**
   - `ClaudeSettings::parse()` with valid/invalid JSON
   - `load_settings_input()` file vs JSON detection
   - `SettingsLoader::load_with_overrides()` merge behavior

2. **Integration Tests**
   - Full CLI flow with `--settings` flag
   - Precedence verification (CLI overrides files)
   - Error handling for invalid inputs

3. **Manual Testing**
   - `cargo run -- --help` shows the flag
   - `cargo run -- --settings '{"env":{"FOO":"bar"}}' -p "test"` works
   - Invalid JSON produces warning but continues

4. **Pre-commit Checks**
   - `make check` passes (lint, format, test, build)
   - `cargo clippy --all-targets --all-features` clean
   - `cargo test --all` passes
