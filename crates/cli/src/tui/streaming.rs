// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Token streaming simulation.

/// Configuration for streaming simulation
#[derive(Clone, Debug, Default)]
pub struct StreamingConfig;

/// Streaming response state
pub struct StreamingResponse {
    /// Full response text
    full_text: String,

    /// Tokens streamed (estimated from text length)
    tokens_streamed: u32,
}

impl StreamingResponse {
    /// Create a new streaming response
    pub fn new(text: String, _config: StreamingConfig, _clock: crate::time::ClockHandle) -> Self {
        // Calculate token count based on text length (rough: 4 chars per token)
        let tokens_streamed = (text.len() / 4).max(1) as u32;

        Self {
            full_text: text,
            tokens_streamed,
        }
    }

    /// Get the number of tokens streamed
    pub fn tokens_streamed(&self) -> u32 {
        self.tokens_streamed
    }

    /// Get the full text (for immediate display)
    pub fn full_text(&self) -> &str {
        &self.full_text
    }
}
