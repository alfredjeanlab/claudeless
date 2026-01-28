// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Captured interaction data types.

use super::duration_serde;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

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

#[cfg(test)]
#[path = "interaction_tests.rs"]
mod tests;
