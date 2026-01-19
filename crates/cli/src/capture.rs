// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Interaction capture and recording for test assertions.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// Captured interaction record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapturedInteraction {
    /// Sequence number
    pub seq: u64,

    /// Wall-clock timestamp
    pub timestamp: SystemTime,

    /// Elapsed time since capture started
    #[serde(with = "duration_serde")]
    pub elapsed: Duration,

    /// CLI arguments received
    pub args: CapturedArgs,

    /// Response returned (or error)
    pub outcome: CapturedOutcome,
}

/// Captured CLI arguments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapturedArgs {
    pub prompt: Option<String>,
    pub model: String,
    pub output_format: String,
    pub print_mode: bool,
    pub continue_conversation: bool,
    pub resume: Option<String>,
    pub allowed_tools: Vec<String>,
    pub cwd: Option<String>,
}

/// Captured outcome (response or failure)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CapturedOutcome {
    Response {
        text: String,
        matched_rule: Option<String>,
        delay_ms: u64,
    },
    Failure {
        failure_type: String,
        message: String,
    },
    NoMatch {
        used_default: bool,
    },
}

/// Capture log for recording interactions
pub struct CaptureLog {
    start: Instant,
    interactions: Arc<Mutex<Vec<CapturedInteraction>>>,
    file_writer: Option<Arc<Mutex<BufWriter<File>>>>,
}

impl CaptureLog {
    /// Create a new in-memory capture log
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            interactions: Arc::new(Mutex::new(Vec::new())),
            file_writer: None,
        }
    }

    /// Create a capture log that writes to a file (JSONL format)
    pub fn with_file(path: &Path) -> std::io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            start: Instant::now(),
            interactions: Arc::new(Mutex::new(Vec::new())),
            file_writer: Some(Arc::new(Mutex::new(BufWriter::new(file)))),
        })
    }

    /// Record an interaction
    pub fn record(&self, args: CapturedArgs, outcome: CapturedOutcome) {
        let mut interactions = self.interactions.lock().expect("capture lock");
        let seq = interactions.len() as u64;
        let interaction = CapturedInteraction {
            seq,
            timestamp: SystemTime::now(),
            elapsed: self.start.elapsed(),
            args,
            outcome,
        };

        interactions.push(interaction.clone());

        // Write to file if configured
        if let Some(ref writer) = self.file_writer {
            use std::io::Write;
            let mut w = writer.lock().expect("file lock");
            if let Ok(json) = serde_json::to_string(&interaction) {
                let _ = writeln!(w, "{}", json);
                let _ = w.flush();
            }
        }
    }

    /// Get all captured interactions
    pub fn interactions(&self) -> Vec<CapturedInteraction> {
        self.interactions.lock().expect("capture lock").clone()
    }

    /// Get the last N interactions
    pub fn last(&self, n: usize) -> Vec<CapturedInteraction> {
        let all = self.interactions.lock().expect("capture lock");
        all.iter().rev().take(n).rev().cloned().collect()
    }

    /// Count interactions matching a predicate
    pub fn count<F: Fn(&CapturedInteraction) -> bool>(&self, pred: F) -> usize {
        self.interactions
            .lock()
            .expect("capture lock")
            .iter()
            .filter(|i| pred(i))
            .count()
    }

    /// Find interactions by prompt pattern
    pub fn find_by_prompt(&self, pattern: &str) -> Vec<CapturedInteraction> {
        self.interactions
            .lock()
            .expect("capture lock")
            .iter()
            .filter(|i| i.args.prompt.as_ref().is_some_and(|p| p.contains(pattern)))
            .cloned()
            .collect()
    }

    /// Find interactions with successful responses
    pub fn find_responses(&self) -> Vec<CapturedInteraction> {
        self.interactions
            .lock()
            .expect("capture lock")
            .iter()
            .filter(|i| matches!(i.outcome, CapturedOutcome::Response { .. }))
            .cloned()
            .collect()
    }

    /// Find interactions with failures
    pub fn find_failures(&self) -> Vec<CapturedInteraction> {
        self.interactions
            .lock()
            .expect("capture lock")
            .iter()
            .filter(|i| matches!(i.outcome, CapturedOutcome::Failure { .. }))
            .cloned()
            .collect()
    }

    /// Get the total number of interactions
    pub fn len(&self) -> usize {
        self.interactions.lock().expect("capture lock").len()
    }

    /// Check if the log is empty
    pub fn is_empty(&self) -> bool {
        self.interactions.lock().expect("capture lock").is_empty()
    }

    /// Clear all recorded interactions
    pub fn clear(&self) {
        self.interactions.lock().expect("capture lock").clear();
    }
}

impl Default for CaptureLog {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CaptureLog {
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            interactions: Arc::clone(&self.interactions),
            file_writer: self.file_writer.as_ref().map(Arc::clone),
        }
    }
}

/// Serde helpers for Duration
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    #[derive(Serialize, Deserialize)]
    struct DurationDef {
        secs: u64,
        nanos: u32,
    }

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        DurationDef {
            secs: duration.as_secs(),
            nanos: duration.subsec_nanos(),
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = DurationDef::deserialize(deserializer)?;
        Ok(Duration::new(def.secs, def.nanos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_args(prompt: Option<&str>) -> CapturedArgs {
        CapturedArgs {
            prompt: prompt.map(|s| s.to_string()),
            model: "claude-test".to_string(),
            output_format: "text".to_string(),
            print_mode: true,
            continue_conversation: false,
            resume: None,
            allowed_tools: vec![],
            cwd: None,
        }
    }

    #[test]
    fn test_record_and_retrieve() {
        let log = CaptureLog::new();

        log.record(
            make_args(Some("hello")),
            CapturedOutcome::Response {
                text: "Hi!".to_string(),
                matched_rule: None,
                delay_ms: 0,
            },
        );

        assert_eq!(log.len(), 1);
        let interactions = log.interactions();
        assert_eq!(interactions[0].seq, 0);
        assert_eq!(interactions[0].args.prompt, Some("hello".to_string()));
    }

    #[test]
    fn test_last_n() {
        let log = CaptureLog::new();

        for i in 0..5 {
            log.record(
                make_args(Some(&format!("prompt {}", i))),
                CapturedOutcome::Response {
                    text: format!("response {}", i),
                    matched_rule: None,
                    delay_ms: 0,
                },
            );
        }

        let last2 = log.last(2);
        assert_eq!(last2.len(), 2);
        assert_eq!(last2[0].args.prompt, Some("prompt 3".to_string()));
        assert_eq!(last2[1].args.prompt, Some("prompt 4".to_string()));
    }

    #[test]
    fn test_count() {
        let log = CaptureLog::new();

        log.record(
            make_args(Some("test")),
            CapturedOutcome::Response {
                text: "ok".to_string(),
                matched_rule: None,
                delay_ms: 0,
            },
        );
        log.record(
            make_args(Some("other")),
            CapturedOutcome::Failure {
                failure_type: "test".to_string(),
                message: "error".to_string(),
            },
        );
        log.record(
            make_args(Some("test again")),
            CapturedOutcome::Response {
                text: "ok".to_string(),
                matched_rule: None,
                delay_ms: 0,
            },
        );

        let response_count = log.count(|i| matches!(i.outcome, CapturedOutcome::Response { .. }));
        assert_eq!(response_count, 2);

        let failure_count = log.count(|i| matches!(i.outcome, CapturedOutcome::Failure { .. }));
        assert_eq!(failure_count, 1);
    }

    #[test]
    fn test_find_by_prompt() {
        let log = CaptureLog::new();

        log.record(
            make_args(Some("hello world")),
            CapturedOutcome::Response {
                text: "hi".to_string(),
                matched_rule: None,
                delay_ms: 0,
            },
        );
        log.record(
            make_args(Some("goodbye")),
            CapturedOutcome::Response {
                text: "bye".to_string(),
                matched_rule: None,
                delay_ms: 0,
            },
        );
        log.record(
            make_args(Some("hello again")),
            CapturedOutcome::Response {
                text: "hi again".to_string(),
                matched_rule: None,
                delay_ms: 0,
            },
        );

        let matches = log.find_by_prompt("hello");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_find_responses_and_failures() {
        let log = CaptureLog::new();

        log.record(
            make_args(Some("a")),
            CapturedOutcome::Response {
                text: "ok".to_string(),
                matched_rule: None,
                delay_ms: 0,
            },
        );
        log.record(
            make_args(Some("b")),
            CapturedOutcome::Failure {
                failure_type: "error".to_string(),
                message: "failed".to_string(),
            },
        );

        assert_eq!(log.find_responses().len(), 1);
        assert_eq!(log.find_failures().len(), 1);
    }

    #[test]
    fn test_clear() {
        let log = CaptureLog::new();

        log.record(
            make_args(Some("test")),
            CapturedOutcome::Response {
                text: "ok".to_string(),
                matched_rule: None,
                delay_ms: 0,
            },
        );

        assert!(!log.is_empty());
        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn test_serialization() {
        let interaction = CapturedInteraction {
            seq: 0,
            timestamp: SystemTime::now(),
            elapsed: Duration::from_millis(100),
            args: make_args(Some("test")),
            outcome: CapturedOutcome::Response {
                text: "response".to_string(),
                matched_rule: Some("rule1".to_string()),
                delay_ms: 50,
            },
        };

        let json = serde_json::to_string(&interaction).unwrap();
        let parsed: CapturedInteraction = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.seq, 0);
        assert_eq!(parsed.args.prompt, Some("test".to_string()));
    }

    #[test]
    fn test_file_capture() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("capture.jsonl");

        {
            let log = CaptureLog::with_file(&path).unwrap();
            log.record(
                make_args(Some("prompt1")),
                CapturedOutcome::Response {
                    text: "response1".to_string(),
                    matched_rule: None,
                    delay_ms: 0,
                },
            );
            log.record(
                make_args(Some("prompt2")),
                CapturedOutcome::Response {
                    text: "response2".to_string(),
                    matched_rule: None,
                    delay_ms: 0,
                },
            );
        }

        // Read back the file
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        // Parse each line
        let i1: CapturedInteraction = serde_json::from_str(lines[0]).unwrap();
        let i2: CapturedInteraction = serde_json::from_str(lines[1]).unwrap();

        assert_eq!(i1.args.prompt, Some("prompt1".to_string()));
        assert_eq!(i2.args.prompt, Some("prompt2".to_string()));
    }
}
