// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Scenario matching and loading.

use crate::config::{PatternSpec, ResponseRule, ResponseSpec, ScenarioConfig, ToolCallSpec};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur when working with scenarios
#[derive(Debug, Error)]
pub enum ScenarioError {
    #[error("Failed to read scenario file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse TOML: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid regex pattern: {0}")]
    Regex(#[from] regex::Error),

    #[error("Invalid glob pattern: {0}")]
    Glob(#[from] glob::PatternError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Failed to resolve file reference '{path}': {source}")]
    FileReference {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Compiled scenario ready for matching
#[derive(Debug)]
pub struct Scenario {
    config: ScenarioConfig,
    compiled_patterns: Vec<CompiledRule>,
    match_counts: Vec<u32>,
}

/// Compiled matcher type for pattern matching
type Matcher = Arc<dyn Fn(&str) -> bool + Send + Sync>;

struct CompiledRule {
    matcher: Matcher,
    rule_index: usize,
}

impl std::fmt::Debug for CompiledRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledRule")
            .field("rule_index", &self.rule_index)
            .finish_non_exhaustive()
    }
}

impl Scenario {
    /// Load a scenario from a TOML or JSON file
    ///
    /// Supports file references in tool call inputs using the `$file` key:
    /// ```toml
    /// [default_response.tool_calls.input]
    /// plan_content = { "$file" = "plan.md" }
    /// ```
    ///
    /// File paths are resolved relative to the scenario file's directory.
    pub fn load(path: &Path) -> Result<Self, ScenarioError> {
        let content = std::fs::read_to_string(path)?;
        let mut config: ScenarioConfig = if path.extension().is_some_and(|e| e == "json") {
            serde_json::from_str(&content)?
        } else {
            toml::from_str(&content)?
        };

        // Resolve file references relative to scenario directory
        let scenario_dir = path.parent().unwrap_or(Path::new("."));
        resolve_file_references_in_config(&mut config, scenario_dir)?;

        Self::from_config(config)
    }

    /// Create a scenario from a config object
    pub fn from_config(config: ScenarioConfig) -> Result<Self, ScenarioError> {
        // Validate session_id format if provided
        if let Some(ref id) = config.session_id {
            if uuid::Uuid::parse_str(id).is_err() {
                return Err(ScenarioError::Validation(format!(
                    "Invalid session_id '{}': must be a valid UUID",
                    id
                )));
            }
        }

        // Validate launch_timestamp format if provided
        if let Some(ref ts) = config.launch_timestamp {
            if chrono::DateTime::parse_from_rfc3339(ts).is_err() {
                return Err(ScenarioError::Validation(format!(
                    "Invalid launch_timestamp '{}': must be ISO 8601 format (e.g., 2025-01-15T10:30:00Z)",
                    ts
                )));
            }
        }

        // Validate permission_mode if provided
        if let Some(ref mode) = config.permission_mode {
            let valid = [
                "default",
                "plan",
                "bypass-permissions",
                "accept-edits",
                "dont-ask",
                "delegate",
            ];
            if !valid.contains(&mode.to_lowercase().as_str()) {
                return Err(ScenarioError::Validation(format!(
                    "Invalid permission_mode '{}': must be one of {:?}",
                    mode, valid
                )));
            }
        }

        // Compile response patterns
        let mut compiled = Vec::new();
        for (idx, rule) in config.responses.iter().enumerate() {
            let matcher = compile_pattern(&rule.pattern)?;
            compiled.push(CompiledRule {
                matcher,
                rule_index: idx,
            });
        }
        let match_counts = vec![0; config.responses.len()];
        Ok(Self {
            config,
            compiled_patterns: compiled,
            match_counts,
        })
    }

    /// Find matching response for a prompt
    pub fn match_prompt(&mut self, prompt: &str) -> Option<&ResponseRule> {
        for compiled in &self.compiled_patterns {
            let rule = &self.config.responses[compiled.rule_index];

            // Check max_matches limit
            if let Some(max) = rule.max_matches {
                if self.match_counts[compiled.rule_index] >= max {
                    continue;
                }
            }

            if (compiled.matcher)(prompt) {
                self.match_counts[compiled.rule_index] += 1;
                return Some(rule);
            }
        }

        None
    }

    /// Get the default response if configured
    pub fn default_response(&self) -> Option<&ResponseSpec> {
        self.config.default_response.as_ref()
    }

    /// Get the scenario name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get the scenario configuration
    pub fn config(&self) -> &ScenarioConfig {
        &self.config
    }

    /// Reset match counts (useful for tests)
    pub fn reset_counts(&mut self) {
        for count in &mut self.match_counts {
            *count = 0;
        }
    }
}

/// Resolve file references in the scenario config.
///
/// File references use the `$file` key to load content from external files:
/// ```json
/// { "$file": "relative/path.md" }
/// ```
///
/// The file content replaces the entire object containing `$file`.
/// For JSON files (`.json`), content is parsed as JSON; otherwise loaded as string.
fn resolve_file_references_in_config(
    config: &mut ScenarioConfig,
    base_dir: &Path,
) -> Result<(), ScenarioError> {
    // Resolve in default_response
    if let Some(ref mut response) = config.default_response {
        resolve_file_references_in_response(response, base_dir)?;
    }

    // Resolve in responses
    for rule in &mut config.responses {
        if let Some(ref mut response) = rule.response {
            resolve_file_references_in_response(response, base_dir)?;
        }
    }

    // Resolve in conversations
    for conv in config.conversations.values_mut() {
        for turn in &mut conv.turns {
            resolve_file_references_in_response(&mut turn.response, base_dir)?;
        }
    }

    Ok(())
}

fn resolve_file_references_in_response(
    response: &mut ResponseSpec,
    base_dir: &Path,
) -> Result<(), ScenarioError> {
    if let ResponseSpec::Detailed { tool_calls, .. } = response {
        for tool_call in tool_calls {
            resolve_file_references_in_tool_call(tool_call, base_dir)?;
        }
    }
    Ok(())
}

fn resolve_file_references_in_tool_call(
    tool_call: &mut ToolCallSpec,
    base_dir: &Path,
) -> Result<(), ScenarioError> {
    tool_call.input = resolve_file_references_in_value(tool_call.input.take(), base_dir)?;
    Ok(())
}

fn resolve_file_references_in_value(
    value: serde_json::Value,
    base_dir: &Path,
) -> Result<serde_json::Value, ScenarioError> {
    match value {
        serde_json::Value::Object(mut map) => {
            // Check if this object is a file reference
            if let Some(file_path) = map.get("$file").and_then(|v| v.as_str()) {
                let full_path = base_dir.join(file_path);
                let content = std::fs::read_to_string(&full_path).map_err(|e| {
                    ScenarioError::FileReference {
                        path: file_path.to_string(),
                        source: e,
                    }
                })?;

                // Parse as JSON if it's a .json file, otherwise return as string
                if full_path.extension().is_some_and(|e| e == "json") {
                    return serde_json::from_str(&content).map_err(ScenarioError::Json);
                } else {
                    return Ok(serde_json::Value::String(content));
                }
            }

            // Otherwise, recursively resolve file references in all values
            for value in map.values_mut() {
                *value = resolve_file_references_in_value(value.take(), base_dir)?;
            }
            Ok(serde_json::Value::Object(map))
        }
        serde_json::Value::Array(arr) => {
            let resolved: Result<Vec<_>, _> = arr
                .into_iter()
                .map(|v| resolve_file_references_in_value(v, base_dir))
                .collect();
            Ok(serde_json::Value::Array(resolved?))
        }
        // Primitives pass through unchanged
        other => Ok(other),
    }
}

fn compile_pattern(spec: &PatternSpec) -> Result<Matcher, ScenarioError> {
    match spec {
        PatternSpec::Exact { text } => {
            let text = text.clone();
            Ok(Arc::new(move |prompt| prompt == text))
        }
        PatternSpec::Regex { pattern } => {
            let re = regex::Regex::new(pattern)?;
            Ok(Arc::new(move |prompt| re.is_match(prompt)))
        }
        PatternSpec::Glob { pattern } => {
            let glob = glob::Pattern::new(pattern)?;
            Ok(Arc::new(move |prompt| glob.matches(prompt)))
        }
        PatternSpec::Contains { text } => {
            let text = text.clone();
            Ok(Arc::new(move |prompt| prompt.contains(&text)))
        }
        PatternSpec::Any => Ok(Arc::new(|_| true)),
    }
}

#[cfg(test)]
#[path = "scenario_tests.rs"]
mod tests;
