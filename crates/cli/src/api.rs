// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Rust API for configuring and controlling the simulator in tests.

use crate::capture::{CaptureLog, CapturedArgs, CapturedInteraction, CapturedOutcome};
use crate::config::{PatternSpec, ResponseRule, ResponseSpec, ScenarioConfig};
use crate::scenario::Scenario;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// Builder for configuring a simulator instance
pub struct SimulatorBuilder {
    scenario: ScenarioConfig,
    capture: Option<PathBuf>,
    delay_ms: Option<u64>,
}

impl SimulatorBuilder {
    /// Create a new simulator builder with default configuration
    pub fn new() -> Self {
        Self {
            scenario: ScenarioConfig::default(),
            capture: None,
            delay_ms: None,
        }
    }

    /// Load scenario from file
    pub fn scenario_file(mut self, path: impl AsRef<Path>) -> Self {
        let content = std::fs::read_to_string(path.as_ref()).expect("read scenario");
        self.scenario = toml::from_str(&content).expect("parse scenario");
        self
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
            response: ResponseSpec::Simple(response.to_string()),
            failure: None,
            max_matches: None,
        });
        self
    }

    /// Add an exact match response rule
    pub fn respond_to_exact(mut self, pattern: &str, response: &str) -> Self {
        self.scenario.responses.push(ResponseRule {
            pattern: PatternSpec::Exact {
                text: pattern.to_string(),
            },
            response: ResponseSpec::Simple(response.to_string()),
            failure: None,
            max_matches: None,
        });
        self
    }

    /// Add a regex-matched response
    pub fn respond_to_regex(mut self, pattern: &str, response: &str) -> Self {
        self.scenario.responses.push(ResponseRule {
            pattern: PatternSpec::Regex {
                pattern: pattern.to_string(),
            },
            response: ResponseSpec::Simple(response.to_string()),
            failure: None,
            max_matches: None,
        });
        self
    }

    /// Set default response for unmatched prompts
    pub fn default_response(mut self, response: &str) -> Self {
        self.scenario.default_response = Some(ResponseSpec::Simple(response.to_string()));
        self
    }

    /// Enable capture to file
    pub fn capture_to(mut self, path: impl Into<PathBuf>) -> Self {
        self.capture = Some(path.into());
        self
    }

    /// Set response delay for all responses
    pub fn delay_ms(mut self, ms: u64) -> Self {
        self.delay_ms = Some(ms);
        self
    }

    /// Build an in-process simulator handle
    pub fn build_in_process(self) -> SimulatorHandle {
        let scenario = Scenario::from_config(self.scenario).expect("compile scenario");
        let capture = match self.capture {
            Some(path) => CaptureLog::with_file(&path).expect("open capture file"),
            None => CaptureLog::new(),
        };
        SimulatorHandle::InProcess {
            scenario: Arc::new(Mutex::new(scenario)),
            capture: Arc::new(capture),
            delay_ms: self.delay_ms,
        }
    }

    /// Build a simulator that spawns a separate binary
    pub fn build_binary(self) -> std::io::Result<BinarySimulatorHandle> {
        let temp_dir = TempDir::new()?;

        // Write scenario config
        let scenario_path = temp_dir.path().join("scenario.toml");
        let scenario_toml = toml::to_string(&self.scenario).expect("serialize");
        std::fs::write(&scenario_path, scenario_toml)?;

        // Capture file path
        let capture_path = self
            .capture
            .unwrap_or_else(|| temp_dir.path().join("capture.jsonl"));

        Ok(BinarySimulatorHandle {
            _temp_dir: temp_dir,
            scenario_path,
            capture_path,
            delay_ms: self.delay_ms,
        })
    }
}

impl Default for SimulatorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a running simulator
pub enum SimulatorHandle {
    InProcess {
        scenario: Arc<Mutex<Scenario>>,
        capture: Arc<CaptureLog>,
        delay_ms: Option<u64>,
    },
}

impl SimulatorHandle {
    /// Get the capture log
    pub fn capture(&self) -> &CaptureLog {
        match self {
            Self::InProcess { capture, .. } => capture,
        }
    }

    /// Execute a simulated request (in-process mode)
    pub fn execute(&self, prompt: &str) -> String {
        self.execute_with_args(prompt, None)
    }

    /// Execute a simulated request with additional args
    pub fn execute_with_args(&self, prompt: &str, model: Option<&str>) -> String {
        match self {
            Self::InProcess {
                scenario, capture, ..
            } => {
                let mut s = scenario.lock().expect("scenario lock");

                let args = CapturedArgs {
                    prompt: Some(prompt.to_string()),
                    model: model.unwrap_or("claude-test").to_string(),
                    output_format: "text".to_string(),
                    print_mode: true,
                    continue_conversation: false,
                    resume: None,
                    allowed_tools: vec![],
                    cwd: None,
                };

                let (text, matched_rule) = if let Some(rule) = s.match_prompt(prompt) {
                    let text = match &rule.response {
                        ResponseSpec::Simple(text) => text.clone(),
                        ResponseSpec::Detailed { text, .. } => text.clone(),
                    };
                    (text, Some("matched".to_string()))
                } else if let Some(default) = s.default_response() {
                    let text = match default {
                        ResponseSpec::Simple(text) => text.clone(),
                        ResponseSpec::Detailed { text, .. } => text.clone(),
                    };
                    (text, Some("default".to_string()))
                } else {
                    (String::new(), None)
                };

                capture.record(
                    args,
                    CapturedOutcome::Response {
                        text: text.clone(),
                        matched_rule,
                        delay_ms: 0,
                    },
                );

                text
            }
        }
    }

    /// Assert that a prompt was received
    pub fn assert_received(&self, pattern: &str) {
        let matches = self.capture().find_by_prompt(pattern);
        assert!(
            !matches.is_empty(),
            "Expected prompt containing '{}' but none found",
            pattern
        );
    }

    /// Assert that a prompt was NOT received
    pub fn assert_not_received(&self, pattern: &str) {
        let matches = self.capture().find_by_prompt(pattern);
        assert!(
            matches.is_empty(),
            "Expected no prompt containing '{}' but found {}",
            pattern,
            matches.len()
        );
    }

    /// Assert interaction count
    pub fn assert_count(&self, expected: usize) {
        let actual = self.capture().len();
        assert_eq!(
            actual, expected,
            "Expected {} interactions, got {}",
            expected, actual
        );
    }

    /// Assert that the last response contains a pattern
    pub fn assert_last_response_contains(&self, pattern: &str) {
        let last = self.capture().last(1);
        assert!(!last.is_empty(), "No interactions recorded");

        if let CapturedOutcome::Response { text, .. } = &last[0].outcome {
            assert!(
                text.contains(pattern),
                "Expected last response to contain '{}', got '{}'",
                pattern,
                text
            );
        } else {
            panic!("Last interaction was not a response");
        }
    }

    /// Reset the scenario match counts
    pub fn reset(&self) {
        match self {
            Self::InProcess {
                scenario, capture, ..
            } => {
                scenario.lock().expect("scenario lock").reset_counts();
                capture.clear();
            }
        }
    }
}

/// Handle for binary-mode simulator
pub struct BinarySimulatorHandle {
    _temp_dir: TempDir,
    scenario_path: PathBuf,
    capture_path: PathBuf,
    delay_ms: Option<u64>,
}

impl BinarySimulatorHandle {
    /// Get environment variables to set for subprocess
    pub fn env_vars(&self) -> Vec<(&str, String)> {
        let mut vars = vec![
            (
                "CLAUDELESS_SCENARIO",
                self.scenario_path.to_string_lossy().to_string(),
            ),
            (
                "CLAUDELESS_CAPTURE",
                self.capture_path.to_string_lossy().to_string(),
            ),
        ];
        if let Some(delay) = self.delay_ms {
            vars.push(("CLAUDELESS_DELAY_MS", delay.to_string()));
        }
        vars
    }

    /// Get the path to use for the simulator binary
    ///
    /// This returns the path to the claudeless binary in the target directory.
    /// The binary should be built before running tests that use it.
    pub fn binary_path() -> PathBuf {
        // Try to get from CARGO_BIN_EXE_claudeless (set during cargo test)
        if let Ok(path) = std::env::var("CARGO_BIN_EXE_claudeless") {
            return PathBuf::from(path);
        }

        // Fallback: look for it in target/debug or target/release
        let target_dir = std::env::var("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("target"));

        let debug_path = target_dir.join("debug/claudeless");
        if debug_path.exists() {
            return debug_path;
        }

        let release_path = target_dir.join("release/claudeless");
        if release_path.exists() {
            return release_path;
        }

        // Last resort: assume it's in PATH
        PathBuf::from("claudeless")
    }

    /// Get the scenario file path
    pub fn scenario_path(&self) -> &Path {
        &self.scenario_path
    }

    /// Get the capture file path
    pub fn capture_path(&self) -> &Path {
        &self.capture_path
    }

    /// Read capture log from file
    pub fn read_capture(&self) -> Vec<CapturedInteraction> {
        let content = std::fs::read_to_string(&self.capture_path).unwrap_or_default();
        content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_respond_to() {
        let sim = SimulatorBuilder::new()
            .respond_to("hello", "Hello back!")
            .build_in_process();

        let response = sim.execute("hello world");
        assert_eq!(response, "Hello back!");
    }

    #[test]
    fn test_builder_respond_to_exact() {
        let sim = SimulatorBuilder::new()
            .respond_to_exact("hello", "Exact match!")
            .respond_to("hello", "Contains match!")
            .build_in_process();

        // Exact match
        let response = sim.execute("hello");
        assert_eq!(response, "Exact match!");

        // Contains match (exact doesn't match)
        let response = sim.execute("hello world");
        assert_eq!(response, "Contains match!");
    }

    #[test]
    fn test_builder_respond_to_regex() {
        let sim = SimulatorBuilder::new()
            .respond_to_regex(r"^test\d+$", "Matched number pattern!")
            .build_in_process();

        assert_eq!(sim.execute("test123"), "Matched number pattern!");
        assert_eq!(sim.execute("test"), ""); // No match
    }

    #[test]
    fn test_builder_default_response() {
        let sim = SimulatorBuilder::new()
            .respond_to("specific", "Specific response")
            .default_response("Default fallback")
            .build_in_process();

        assert_eq!(sim.execute("specific"), "Specific response");
        assert_eq!(sim.execute("anything else"), "Default fallback");
    }

    #[test]
    fn test_assertions() {
        let sim = SimulatorBuilder::new()
            .respond_to("test", "response")
            .build_in_process();

        sim.execute("test prompt");

        sim.assert_received("test");
        sim.assert_not_received("other");
        sim.assert_count(1);
        sim.assert_last_response_contains("response");
    }

    #[test]
    fn test_reset() {
        let sim = SimulatorBuilder::new()
            .respond_to("test", "response")
            .build_in_process();

        sim.execute("test");
        sim.assert_count(1);

        sim.reset();
        sim.assert_count(0);
    }

    #[test]
    fn test_capture_recorded() {
        let sim = SimulatorBuilder::new()
            .respond_to("hello", "Hi!")
            .build_in_process();

        sim.execute("hello world");

        let interactions = sim.capture().interactions();
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].args.prompt, Some("hello world".to_string()));
    }

    #[test]
    fn test_execute_with_args() {
        let sim = SimulatorBuilder::new()
            .respond_to("test", "ok")
            .build_in_process();

        sim.execute_with_args("test", Some("claude-opus"));

        let interactions = sim.capture().interactions();
        assert_eq!(interactions[0].args.model, "claude-opus");
    }

    #[test]
    fn test_binary_handle_env_vars() {
        let handle = SimulatorBuilder::new()
            .respond_to("test", "ok")
            .delay_ms(100)
            .build_binary()
            .unwrap();

        let vars = handle.env_vars();
        assert!(vars.iter().any(|(k, _)| *k == "CLAUDELESS_SCENARIO"));
        assert!(vars.iter().any(|(k, _)| *k == "CLAUDELESS_CAPTURE"));
        assert!(vars
            .iter()
            .any(|(k, v)| *k == "CLAUDELESS_DELAY_MS" && v == "100"));
    }

    #[test]
    fn test_binary_handle_paths() {
        let handle = SimulatorBuilder::new()
            .respond_to("test", "ok")
            .build_binary()
            .unwrap();

        assert!(handle.scenario_path().exists());
        // Capture file may not exist yet
    }

    #[test]
    fn test_multiple_rules_order() {
        let sim = SimulatorBuilder::new()
            .respond_to("error", "Error response")
            .respond_to("warning", "Warning response")
            .respond_to("info", "Info response")
            .build_in_process();

        assert_eq!(sim.execute("an error occurred"), "Error response");
        assert_eq!(sim.execute("warning message"), "Warning response");
        assert_eq!(sim.execute("info log"), "Info response");
    }

    #[test]
    fn test_scenario_from_config() {
        let config = ScenarioConfig {
            name: "test".to_string(),
            default_response: Some(ResponseSpec::Simple("default".to_string())),
            responses: vec![],
            conversations: Default::default(),
            tool_execution: None,
            ..Default::default()
        };

        let sim = SimulatorBuilder::new().scenario(config).build_in_process();

        assert_eq!(sim.execute("anything"), "default");
    }
}
