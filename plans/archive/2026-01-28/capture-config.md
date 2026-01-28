# Capture Spec Configuration System

## Overview

Implement a TOML-based configuration system for capture specifications. This allows defining capture behavior declaratively for integration testing, including what to capture (TUI, dot-claude, CLI), expected states to validate, key sequences to send, and normalization rules for deterministic output comparison.

## Project Structure

```
crates/cli/src/
├── capture_spec.rs          # NEW: CaptureSpec struct and loading logic
├── capture_spec_tests.rs    # NEW: Unit tests for capture spec
├── config.rs                # MODIFY: Add CaptureSpec field to ScenarioConfig
└── scenario.rs              # MODIFY: Validate and wire up capture spec

scenarios/
├── capture-basic.toml       # NEW: Example basic capture scenario
└── capture-full.toml        # NEW: Example full-featured capture scenario

docs/
└── CAPTURE.md               # NEW: Capture configuration documentation
```

## Dependencies

No new external dependencies required. Uses existing:
- `serde` (derive) - serialization/deserialization
- `toml` - TOML parsing
- `regex` - pattern matching for normalization rules
- `thiserror` - error types

## Implementation Phases

### Phase 1: Core CaptureSpec Struct

Define the main configuration struct and supporting types in `capture_spec.rs`.

**File: `crates/cli/src/capture_spec.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Capture type - what interface to capture
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureType {
    /// Terminal UI mode (ratatui-based)
    #[default]
    Tui,
    /// .claude directory state capture
    DotClaude,
    /// CLI stdout/stderr capture
    Cli,
}

/// A key sequence to send to the TUI
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeySequence {
    /// Human-readable name for this sequence
    #[serde(default)]
    pub name: Option<String>,

    /// Keys to send (e.g., ["h", "e", "l", "l", "o", "Enter"])
    pub keys: Vec<String>,

    /// Delay in ms before sending (default: 0)
    #[serde(default)]
    pub delay_ms: Option<u64>,

    /// Wait for specific state before sending
    #[serde(default)]
    pub wait_for: Option<StateCondition>,
}

/// Condition to wait for before proceeding
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateCondition {
    /// Wait for text to appear
    TextVisible { pattern: String },
    /// Wait for prompt to be ready
    PromptReady,
    /// Wait for response to complete
    ResponseComplete,
    /// Wait for specific element
    ElementVisible { selector: String },
}

/// Expected state to validate
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedState {
    /// Name for error reporting
    #[serde(default)]
    pub name: Option<String>,

    /// When to check this state (after which key sequence index)
    #[serde(default)]
    pub after_sequence: Option<usize>,

    /// Conditions that must be true
    pub conditions: Vec<StateCondition>,
}

/// Normalization rule for deterministic comparison
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NormalizationRule {
    /// Replace matching pattern with fixed string
    Replace {
        pattern: String,
        replacement: String,
        #[serde(default)]
        flags: Option<String>,
    },
    /// Remove lines matching pattern
    RemoveLines { pattern: String },
    /// Strip ANSI escape codes
    StripAnsi,
    /// Normalize timestamps to fixed value
    NormalizeTimestamps { format: Option<String> },
    /// Normalize UUIDs to placeholder
    NormalizeUuids,
    /// Normalize file paths
    NormalizePaths { base: Option<String> },
}

/// Main capture specification
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureSpec {
    /// Name for logging/debugging
    #[serde(default)]
    pub name: String,

    /// Claude version to simulate (default from scenario or DEFAULT_CLAUDE_VERSION)
    #[serde(default)]
    pub claude_version: Option<String>,

    /// What to capture
    #[serde(default)]
    pub capture_type: CaptureType,

    /// Key sequences to send (TUI mode)
    #[serde(default)]
    pub key_sequences: Vec<KeySequence>,

    /// Expected states to validate
    #[serde(default)]
    pub expected_states: Vec<ExpectedState>,

    /// Normalization rules for output
    #[serde(default)]
    pub normalization_rules: Vec<NormalizationRule>,

    /// Number of retries on transient failures (default: 0)
    #[serde(default)]
    pub retry_count: u32,

    /// Timeout in milliseconds (default: 30000)
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// Output file path for captured data
    #[serde(default)]
    pub output_file: Option<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_timeout_ms() -> u64 {
    30_000
}
```

**Milestone**: `CaptureSpec` compiles and can be serialized/deserialized.

### Phase 2: Config Loading and Validation

Add loading functions and validation logic.

**Add to `capture_spec.rs`:**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureSpecError {
    #[error("invalid capture type: {0}")]
    InvalidCaptureType(String),

    #[error("invalid key sequence at index {index}: {message}")]
    InvalidKeySequence { index: usize, message: String },

    #[error("invalid normalization rule: {0}")]
    InvalidNormalizationRule(String),

    #[error("invalid regex pattern '{pattern}': {error}")]
    InvalidRegex { pattern: String, error: String },

    #[error("timeout must be positive, got {0}")]
    InvalidTimeout(u64),

    #[error("expected state references invalid sequence index {index}, max is {max}")]
    InvalidSequenceReference { index: usize, max: usize },
}

impl CaptureSpec {
    /// Validate the capture spec configuration
    pub fn validate(&self) -> Result<(), CaptureSpecError> {
        // Validate timeout
        if self.timeout_ms == 0 {
            return Err(CaptureSpecError::InvalidTimeout(0));
        }

        // Validate key sequences
        for (i, seq) in self.key_sequences.iter().enumerate() {
            if seq.keys.is_empty() {
                return Err(CaptureSpecError::InvalidKeySequence {
                    index: i,
                    message: "keys array cannot be empty".to_string(),
                });
            }
        }

        // Validate expected state sequence references
        let max_seq = self.key_sequences.len();
        for state in &self.expected_states {
            if let Some(after) = state.after_sequence {
                if after >= max_seq {
                    return Err(CaptureSpecError::InvalidSequenceReference {
                        index: after,
                        max: max_seq.saturating_sub(1),
                    });
                }
            }
        }

        // Validate normalization rule regex patterns
        for rule in &self.normalization_rules {
            if let NormalizationRule::Replace { pattern, .. }
                | NormalizationRule::RemoveLines { pattern } = rule
            {
                regex::Regex::new(pattern).map_err(|e| {
                    CaptureSpecError::InvalidRegex {
                        pattern: pattern.clone(),
                        error: e.to_string(),
                    }
                })?;
            }
        }

        Ok(())
    }
}
```

**Milestone**: Validation catches malformed configs with clear error messages.

### Phase 3: Integration with ScenarioConfig

Wire up CaptureSpec into the existing scenario system.

**Modify `config.rs`:**

```rust
// Add import at top
use crate::capture_spec::CaptureSpec;

// Add field to ScenarioConfig struct
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioConfig {
    // ... existing fields ...

    /// Capture specification for recording/playback
    #[serde(default)]
    pub capture: Option<CaptureSpec>,
}

// Update Default impl
impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            // ... existing fields ...
            capture: None,
        }
    }
}
```

**Modify `scenario.rs`** (in `from_config`):**

```rust
// Validate capture spec if present
if let Some(ref capture) = config.capture {
    capture.validate().map_err(|e| ScenarioError::Config(e.to_string()))?;
}
```

**Milestone**: Scenarios can include capture configuration that gets validated on load.

### Phase 4: Normalization Engine

Implement the normalization pipeline for deterministic output comparison.

**Add to `capture_spec.rs`:**

```rust
impl NormalizationRule {
    /// Apply this rule to input text
    pub fn apply(&self, input: &str) -> String {
        match self {
            NormalizationRule::Replace { pattern, replacement, flags } => {
                let case_insensitive = flags.as_ref()
                    .is_some_and(|f| f.contains('i'));
                let re = regex::RegexBuilder::new(pattern)
                    .case_insensitive(case_insensitive)
                    .build()
                    .expect("validated");
                re.replace_all(input, replacement.as_str()).into_owned()
            }
            NormalizationRule::RemoveLines { pattern } => {
                let re = regex::Regex::new(pattern).expect("validated");
                input.lines()
                    .filter(|line| !re.is_match(line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            NormalizationRule::StripAnsi => {
                strip_ansi_escapes(input)
            }
            NormalizationRule::NormalizeTimestamps { .. } => {
                let re = regex::Regex::new(
                    r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?"
                ).unwrap();
                re.replace_all(input, "[TIMESTAMP]").into_owned()
            }
            NormalizationRule::NormalizeUuids => {
                let re = regex::Regex::new(
                    r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}"
                ).unwrap();
                re.replace_all(input, "[UUID]").into_owned()
            }
            NormalizationRule::NormalizePaths { base } => {
                let home = dirs::home_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                let mut result = input.replace(&home, "~");
                if let Some(base_path) = base {
                    result = result.replace(base_path, "[PROJECT]");
                }
                result
            }
        }
    }
}

impl CaptureSpec {
    /// Apply all normalization rules to input
    pub fn normalize(&self, input: &str) -> String {
        self.normalization_rules
            .iter()
            .fold(input.to_string(), |acc, rule| rule.apply(&acc))
    }
}

fn strip_ansi_escapes(input: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(input, "").into_owned()
}
```

**Milestone**: Normalization pipeline transforms output for consistent comparison.

### Phase 5: Unit Tests

Create comprehensive unit tests following project convention.

**File: `crates/cli/src/capture_spec_tests.rs`**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn deserialize_minimal_spec() {
    let toml = r#"
        name = "minimal"
    "#;
    let spec: CaptureSpec = toml::from_str(toml).unwrap();
    assert_eq!(spec.name, "minimal");
    assert_eq!(spec.capture_type, CaptureType::Tui);
    assert_eq!(spec.timeout_ms, 30_000);
}

#[test]
fn deserialize_full_spec() {
    let toml = r#"
        name = "full-capture"
        claude_version = "2.1.12"
        capture_type = "tui"
        retry_count = 3
        timeout_ms = 60000

        [[key_sequences]]
        name = "type hello"
        keys = ["h", "e", "l", "l", "o", "Enter"]
        delay_ms = 100

        [[expected_states]]
        name = "check response"
        after_sequence = 0

        [[expected_states.conditions]]
        type = "text_visible"
        pattern = "Hello"

        [[normalization_rules]]
        type = "strip_ansi"

        [[normalization_rules]]
        type = "normalize_uuids"
    "#;
    let spec: CaptureSpec = toml::from_str(toml).unwrap();
    assert_eq!(spec.name, "full-capture");
    assert_eq!(spec.retry_count, 3);
    assert_eq!(spec.key_sequences.len(), 1);
    assert_eq!(spec.normalization_rules.len(), 2);
}

#[test]
fn validate_empty_keys_fails() {
    let spec = CaptureSpec {
        key_sequences: vec![KeySequence {
            name: None,
            keys: vec![], // Empty!
            delay_ms: None,
            wait_for: None,
        }],
        ..Default::default()
    };
    let err = spec.validate().unwrap_err();
    assert!(matches!(err, CaptureSpecError::InvalidKeySequence { .. }));
}

#[test]
fn validate_invalid_regex_fails() {
    let spec = CaptureSpec {
        normalization_rules: vec![NormalizationRule::Replace {
            pattern: "[invalid".to_string(),
            replacement: "x".to_string(),
            flags: None,
        }],
        ..Default::default()
    };
    let err = spec.validate().unwrap_err();
    assert!(matches!(err, CaptureSpecError::InvalidRegex { .. }));
}

#[test]
fn normalize_strips_ansi() {
    let spec = CaptureSpec {
        normalization_rules: vec![NormalizationRule::StripAnsi],
        ..Default::default()
    };
    let input = "\x1b[31mred\x1b[0m text";
    assert_eq!(spec.normalize(input), "red text");
}

#[test]
fn normalize_replaces_uuids() {
    let spec = CaptureSpec {
        normalization_rules: vec![NormalizationRule::NormalizeUuids],
        ..Default::default()
    };
    let input = "session: a1b2c3d4-e5f6-7890-abcd-ef1234567890";
    assert_eq!(spec.normalize(input), "session: [UUID]");
}
```

**Milestone**: All unit tests pass, covering edge cases.

### Phase 6: Example Scenarios and Documentation

Create example scenario files and documentation.

**File: `scenarios/capture-basic.toml`**

```toml
name = "basic-capture"

[capture]
name = "simple-capture"
capture_type = "cli"
timeout_ms = 5000

[[capture.normalization_rules]]
type = "normalize_timestamps"

[[capture.normalization_rules]]
type = "normalize_uuids"
```

**File: `scenarios/capture-full.toml`**

```toml
name = "full-capture-demo"
claude_version = "2.1.12"

[capture]
name = "tui-interaction-capture"
capture_type = "tui"
retry_count = 2
timeout_ms = 30000
output_file = "capture-output.jsonl"

[[capture.key_sequences]]
name = "enter prompt"
keys = ["H", "i", "Enter"]
delay_ms = 50

[capture.key_sequences.wait_for]
type = "prompt_ready"

[[capture.key_sequences]]
name = "wait for response"
keys = []  # No keys, just wait

[capture.key_sequences.wait_for]
type = "response_complete"

[[capture.expected_states]]
name = "verify response shown"
after_sequence = 1

[[capture.expected_states.conditions]]
type = "text_visible"
pattern = ".*"

[[capture.normalization_rules]]
type = "strip_ansi"

[[capture.normalization_rules]]
type = "normalize_timestamps"

[[capture.normalization_rules]]
type = "normalize_uuids"

[[capture.normalization_rules]]
type = "replace"
pattern = "/Users/[^/]+/"
replacement = "/Users/[USER]/"
```

**Milestone**: Example scenarios load and validate successfully.

## Key Implementation Details

### Serde Patterns

Follow existing patterns from `config.rs`:
- Use `#[serde(deny_unknown_fields)]` for strict validation
- Use `#[serde(tag = "type", rename_all = "snake_case")]` for enums
- Use `#[serde(default)]` for optional fields with sensible defaults
- Use `#[serde(untagged)]` sparingly (only for simple/detailed variants)

### Error Handling

Use `thiserror` for typed errors matching existing `ScenarioError` pattern:
```rust
#[derive(Debug, Error)]
pub enum CaptureSpecError { ... }
```

### Regex Compilation

Compile regexes once during validation, not on each apply. Consider caching with `once_cell::sync::Lazy` for built-in patterns.

### Key Sequence Format

Keys use string representation compatible with crossterm:
- Single characters: `"a"`, `"A"`, `"1"`
- Named keys: `"Enter"`, `"Escape"`, `"Tab"`, `"Backspace"`
- Modifiers: `"Ctrl+c"`, `"Alt+Enter"`, `"Shift+Tab"`

## Verification Plan

### Unit Tests
- [ ] `cargo test capture_spec` - all capture_spec_tests pass
- [ ] Serialization round-trip (TOML -> struct -> TOML)
- [ ] Validation catches all error cases
- [ ] Normalization rules apply correctly

### Integration Tests
- [ ] Load `scenarios/capture-basic.toml` successfully
- [ ] Load `scenarios/capture-full.toml` successfully
- [ ] Invalid capture configs rejected with clear errors

### Manual Verification
```bash
# Run full check suite
make check

# Verify example scenarios load
cargo run -- --scenario scenarios/capture-basic.toml --help
cargo run -- --scenario scenarios/capture-full.toml --help
```

### Checklist (from CLAUDE.md)
- [ ] Unit tests in sibling `_tests.rs` files
- [ ] `make lint` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --all` passes
- [ ] `cargo build --all` passes
