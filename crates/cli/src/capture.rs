// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Interaction capture and recording for test assertions.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::Arc;
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
        let mut interactions = self.interactions.lock();
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
            let mut w = writer.lock();
            if let Ok(json) = serde_json::to_string(&interaction) {
                let _ = writeln!(w, "{}", json);
                let _ = w.flush();
            }
        }
    }

    /// Get all captured interactions
    pub fn interactions(&self) -> Vec<CapturedInteraction> {
        self.interactions.lock().clone()
    }

    /// Get the last N interactions
    pub fn last(&self, n: usize) -> Vec<CapturedInteraction> {
        let all = self.interactions.lock();
        all.iter().rev().take(n).rev().cloned().collect()
    }

    /// Count interactions matching a predicate
    pub fn count<F: Fn(&CapturedInteraction) -> bool>(&self, pred: F) -> usize {
        self.interactions.lock().iter().filter(|i| pred(i)).count()
    }

    /// Find interactions by prompt pattern
    pub fn find_by_prompt(&self, pattern: &str) -> Vec<CapturedInteraction> {
        self.interactions
            .lock()
            .iter()
            .filter(|i| i.args.prompt.as_ref().is_some_and(|p| p.contains(pattern)))
            .cloned()
            .collect()
    }

    /// Find interactions with successful responses
    pub fn find_responses(&self) -> Vec<CapturedInteraction> {
        self.interactions
            .lock()
            .iter()
            .filter(|i| matches!(i.outcome, CapturedOutcome::Response { .. }))
            .cloned()
            .collect()
    }

    /// Find interactions with failures
    pub fn find_failures(&self) -> Vec<CapturedInteraction> {
        self.interactions
            .lock()
            .iter()
            .filter(|i| matches!(i.outcome, CapturedOutcome::Failure { .. }))
            .cloned()
            .collect()
    }

    /// Get the total number of interactions
    pub fn len(&self) -> usize {
        self.interactions.lock().len()
    }

    /// Check if the log is empty
    pub fn is_empty(&self) -> bool {
        self.interactions.lock().is_empty()
    }

    /// Clear all recorded interactions
    pub fn clear(&self) {
        self.interactions.lock().clear();
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
#[path = "capture_tests.rs"]
mod tests;
