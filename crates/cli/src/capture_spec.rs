// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Capture specification types for TOML/JSON capture configuration.
//!
//! Defines the structure for declarative capture behavior including
//! what to capture, expected states to validate, key sequences to send,
//! and normalization rules for deterministic output comparison.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use thiserror::Error;

/// Static regex for matching ANSI escape sequences
static ANSI_REGEX: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").ok());

/// Static regex for matching ISO 8601 timestamps
static TIMESTAMP_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?").ok()
});

/// Static regex for matching UUIDs
static UUID_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}").ok()
});

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
    NormalizeTimestamps {
        #[serde(default)]
        format: Option<String>,
    },
    /// Normalize UUIDs to placeholder
    NormalizeUuids,
    /// Normalize file paths
    NormalizePaths {
        #[serde(default)]
        base: Option<String>,
    },
}

/// Main capture specification
#[derive(Clone, Debug, Deserialize, Serialize)]
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

impl Default for CaptureSpec {
    fn default() -> Self {
        Self {
            name: String::new(),
            claude_version: None,
            capture_type: CaptureType::default(),
            key_sequences: Vec::new(),
            expected_states: Vec::new(),
            normalization_rules: Vec::new(),
            retry_count: 0,
            timeout_ms: default_timeout_ms(),
            output_file: None,
            metadata: HashMap::new(),
        }
    }
}

/// Errors that can occur when working with capture specifications
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
                regex::Regex::new(pattern).map_err(|e| CaptureSpecError::InvalidRegex {
                    pattern: pattern.clone(),
                    error: e.to_string(),
                })?;
            }
        }

        Ok(())
    }

    /// Apply all normalization rules to input
    pub fn normalize(&self, input: &str) -> String {
        self.normalization_rules
            .iter()
            .fold(input.to_string(), |acc, rule| rule.apply(&acc))
    }
}

impl NormalizationRule {
    /// Apply this rule to input text
    pub fn apply(&self, input: &str) -> String {
        match self {
            NormalizationRule::Replace {
                pattern,
                replacement,
                flags,
            } => {
                let case_insensitive = flags.as_ref().is_some_and(|f| f.contains('i'));
                // Pattern was validated, so this should succeed; if not, return input unchanged
                let Ok(re) = regex::RegexBuilder::new(pattern)
                    .case_insensitive(case_insensitive)
                    .build()
                else {
                    return input.to_string();
                };
                re.replace_all(input, replacement.as_str()).into_owned()
            }
            NormalizationRule::RemoveLines { pattern } => {
                // Pattern was validated, so this should succeed; if not, return input unchanged
                let Ok(re) = regex::Regex::new(pattern) else {
                    return input.to_string();
                };
                input
                    .lines()
                    .filter(|line| !re.is_match(line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            NormalizationRule::StripAnsi => {
                if let Some(re) = ANSI_REGEX.as_ref() {
                    re.replace_all(input, "").into_owned()
                } else {
                    input.to_string()
                }
            }
            NormalizationRule::NormalizeTimestamps { .. } => {
                if let Some(re) = TIMESTAMP_REGEX.as_ref() {
                    re.replace_all(input, "[TIMESTAMP]").into_owned()
                } else {
                    input.to_string()
                }
            }
            NormalizationRule::NormalizeUuids => {
                if let Some(re) = UUID_REGEX.as_ref() {
                    re.replace_all(input, "[UUID]").into_owned()
                } else {
                    input.to_string()
                }
            }
            NormalizationRule::NormalizePaths { base } => {
                let mut result = input.to_string();
                // Replace home directory with ~ using HOME env var
                if let Ok(home) = std::env::var("HOME") {
                    result = result.replace(&home, "~");
                }
                if let Some(base_path) = base {
                    result = result.replace(base_path, "[PROJECT]");
                }
                result
            }
        }
    }
}

#[cfg(test)]
#[path = "capture_spec_tests.rs"]
mod tests;
