// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Token usage types for tracking API consumption.

use serde::{Deserialize, Serialize};

/// Basic token counts (input/output only).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenCounts {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl TokenCounts {
    pub fn new(input: u32, output: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
        }
    }

    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Extended token counts including cache metrics.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExtendedTokenCounts {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    #[serde(default)]
    pub cache_read_input_tokens: u32,
}

impl ExtendedTokenCounts {
    pub fn new(input: u32, output: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        }
    }

    pub fn with_cache(mut self, creation: u32, read: u32) -> Self {
        self.cache_creation_input_tokens = creation;
        self.cache_read_input_tokens = read;
        self
    }
}

impl From<TokenCounts> for ExtendedTokenCounts {
    fn from(counts: TokenCounts) -> Self {
        Self::new(counts.input_tokens, counts.output_tokens)
    }
}

impl From<&ExtendedTokenCounts> for TokenCounts {
    fn from(ext: &ExtendedTokenCounts) -> Self {
        Self::new(ext.input_tokens, ext.output_tokens)
    }
}

/// Token usage with cost calculation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageWithCost {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
    /// Cost breakdown for this request
    pub cost_usd: f64,
}

impl UsageWithCost {
    pub fn from_tokens(input: u32, output: u32) -> Self {
        let cost_usd = estimate_cost(input, output);
        Self {
            input_tokens: input,
            output_tokens: output,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cost_usd,
        }
    }
}

/// Estimate cost based on Claude Sonnet pricing ($3/M input, $15/M output).
pub fn estimate_cost(input_tokens: u32, output_tokens: u32) -> f64 {
    let input_cost = (input_tokens as f64) * 0.000003;
    let output_cost = (output_tokens as f64) * 0.000015;
    input_cost + output_cost
}
