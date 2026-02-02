// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Rust API for configuring and controlling the simulator in tests.

use crate::config::{PatternSpec, ResponseRule, ResponseSpec, ScenarioConfig};
use crate::scenario::{Scenario, ScenarioError};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use thiserror::Error;

/// Errors that can occur when building or running a simulator.
#[derive(Debug, Error)]
pub enum SimulatorError {
    /// Failed to read scenario file.
    #[error("Failed to read scenario file: {0}")]
    ScenarioRead(#[from] std::io::Error),

    /// Failed to parse scenario TOML.
    #[error("Failed to parse scenario: {0}")]
    ScenarioParse(#[from] toml::de::Error),

    /// Failed to compile scenario patterns.
    #[error("Failed to compile scenario: {0}")]
    ScenarioCompile(#[from] ScenarioError),

    /// Failed to serialize scenario.
    #[error("Failed to serialize scenario: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// A recorded interaction from SimulatorHandle::execute.
#[derive(Clone, Debug)]
pub struct RecordedInteraction {
    /// The prompt that was sent.
    pub prompt: String,
    /// The model used.
    pub model: String,
    /// The response text.
    pub response: String,
}

/// Builder for configuring a simulator instance
#[derive(Default)]
pub struct SimulatorBuilder {
    scenario: ScenarioConfig,
}

impl SimulatorBuilder {
    /// Create a new simulator builder with default configuration
    pub fn new() -> Self {
        Self {
            scenario: ScenarioConfig::default(),
        }
    }

    /// Load scenario from file
    pub fn scenario_file(mut self, path: impl AsRef<Path>) -> Result<Self, SimulatorError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        self.scenario = toml::from_str(&content)?;
        Ok(self)
    }

    /// Set scenario from config
    pub fn scenario(mut self, config: ScenarioConfig) -> Self {
        self.scenario = config;
        self
    }

    /// Add a simple response rule (matches substring)
    pub fn respond_to(mut self, pattern: &str, response: &str) -> Self {
        self.scenario.responses.push(ResponseRule {
            pattern: PatternSpec::Contains {
                text: pattern.to_string(),
            },
            response: Some(ResponseSpec::Simple(response.to_string())),
            failure: None,
            max_matches: None,
            turns: Vec::new(),
        });
        self
    }

    /// Add an exact match response rule
    pub fn respond_to_exact(mut self, pattern: &str, response: &str) -> Self {
        self.scenario.responses.push(ResponseRule {
            pattern: PatternSpec::Exact {
                text: pattern.to_string(),
            },
            response: Some(ResponseSpec::Simple(response.to_string())),
            failure: None,
            max_matches: None,
            turns: Vec::new(),
        });
        self
    }

    /// Add a regex-matched response
    pub fn respond_to_regex(mut self, pattern: &str, response: &str) -> Self {
        self.scenario.responses.push(ResponseRule {
            pattern: PatternSpec::Regex {
                pattern: pattern.to_string(),
            },
            response: Some(ResponseSpec::Simple(response.to_string())),
            failure: None,
            max_matches: None,
            turns: Vec::new(),
        });
        self
    }

    /// Set default response for unmatched prompts
    pub fn default_response(mut self, response: &str) -> Self {
        self.scenario.default_response = Some(ResponseSpec::Simple(response.to_string()));
        self
    }

    /// Build an in-process simulator handle
    pub fn build_in_process(self) -> Result<SimulatorHandle, SimulatorError> {
        let scenario = Scenario::from_config(self.scenario)?;
        Ok(SimulatorHandle {
            scenario: Arc::new(Mutex::new(scenario)),
            interactions: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Build a simulator that spawns a separate binary
    pub fn build_binary(self) -> Result<BinarySimulatorHandle, SimulatorError> {
        let temp_dir = TempDir::new()?;

        // Write scenario config
        let scenario_path = temp_dir.path().join("scenario.toml");
        let scenario_toml = toml::to_string(&self.scenario)?;
        std::fs::write(&scenario_path, scenario_toml)?;

        Ok(BinarySimulatorHandle {
            _temp_dir: temp_dir,
            scenario_path,
        })
    }
}

/// Handle to a running simulator
pub struct SimulatorHandle {
    scenario: Arc<Mutex<Scenario>>,
    interactions: Arc<Mutex<Vec<RecordedInteraction>>>,
}

impl SimulatorHandle {
    /// Execute a simulated request (in-process mode)
    pub fn execute(&self, prompt: &str) -> String {
        self.execute_with_args(prompt, None)
    }

    /// Execute a simulated request with additional args
    pub fn execute_with_args(&self, prompt: &str, model: Option<&str>) -> String {
        let mut s = self.scenario.lock();
        let model = model.unwrap_or("claude-test").to_string();

        let text = if let Some(result) = s.match_prompt(prompt) {
            if s.get_failure(&result).is_some() {
                String::new()
            } else {
                let response = s.get_response(&result);
                match response {
                    Some(ResponseSpec::Simple(text)) => text.clone(),
                    Some(ResponseSpec::Detailed { text, .. }) => text.clone(),
                    None => String::new(),
                }
            }
        } else if let Some(default) = s.default_response() {
            match default {
                ResponseSpec::Simple(text) => text.clone(),
                ResponseSpec::Detailed { text, .. } => text.clone(),
            }
        } else {
            String::new()
        };

        self.interactions.lock().push(RecordedInteraction {
            prompt: prompt.to_string(),
            model,
            response: text.clone(),
        });

        text
    }

    /// Assert that a prompt was received
    pub fn assert_received(&self, pattern: &str) {
        let interactions = self.interactions.lock();
        let found = interactions.iter().any(|i| i.prompt.contains(pattern));
        assert!(
            found,
            "Expected prompt containing '{}' but none found",
            pattern
        );
    }

    /// Assert that a prompt was NOT received
    pub fn assert_not_received(&self, pattern: &str) {
        let interactions = self.interactions.lock();
        let count = interactions
            .iter()
            .filter(|i| i.prompt.contains(pattern))
            .count();
        assert!(
            count == 0,
            "Expected no prompt containing '{}' but found {}",
            pattern,
            count
        );
    }

    /// Assert interaction count
    pub fn assert_count(&self, expected: usize) {
        let actual = self.interactions.lock().len();
        assert_eq!(
            actual, expected,
            "Expected {} interactions, got {}",
            expected, actual
        );
    }

    /// Assert that the last response contains a pattern
    pub fn assert_last_response_contains(&self, pattern: &str) {
        let interactions = self.interactions.lock();
        assert!(!interactions.is_empty(), "No interactions recorded");
        let last = &interactions[interactions.len() - 1];
        assert!(
            last.response.contains(pattern),
            "Expected last response to contain '{}', got '{}'",
            pattern,
            last.response
        );
    }

    /// Reset the scenario match counts
    pub fn reset(&self) {
        self.scenario.lock().reset_counts();
        self.interactions.lock().clear();
    }
}

/// Handle for binary-mode simulator
pub struct BinarySimulatorHandle {
    _temp_dir: TempDir,
    scenario_path: std::path::PathBuf,
}

impl BinarySimulatorHandle {
    /// Get environment variables to set for subprocess
    pub fn env_vars(&self) -> Vec<(&str, String)> {
        vec![(
            "CLAUDELESS_SCENARIO",
            self.scenario_path.to_string_lossy().to_string(),
        )]
    }

    /// Get the path to use for the simulator binary
    pub fn binary_path() -> std::path::PathBuf {
        if let Ok(path) = std::env::var("CARGO_BIN_EXE_claudeless") {
            return std::path::PathBuf::from(path);
        }

        let target_dir = std::env::var("CARGO_TARGET_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("target"));

        let debug_path = target_dir.join("debug/claudeless");
        if debug_path.exists() {
            return debug_path;
        }

        let release_path = target_dir.join("release/claudeless");
        if release_path.exists() {
            return release_path;
        }

        std::path::PathBuf::from("claudeless")
    }

    /// Get the scenario file path
    pub fn scenario_path(&self) -> &Path {
        &self.scenario_path
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
