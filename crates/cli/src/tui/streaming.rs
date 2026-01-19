// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Token streaming simulation.

use std::time::Duration;
use tokio::sync::mpsc;

use crate::time::{Clock, ClockHandle};

/// Configuration for streaming simulation
#[derive(Clone, Debug)]
pub struct StreamingConfig {
    /// Tokens per second (0 = instant)
    pub tokens_per_second: u32,

    /// Thinking delay before streaming starts (ms)
    pub thinking_delay_ms: u64,

    /// Minimum chunk size for streaming
    pub min_chunk_size: usize,

    /// Maximum chunk size for streaming
    pub max_chunk_size: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            tokens_per_second: 50,
            thinking_delay_ms: 500,
            min_chunk_size: 1,
            max_chunk_size: 5,
        }
    }
}

impl StreamingConfig {
    /// Instant streaming (no delays)
    pub fn instant() -> Self {
        Self {
            tokens_per_second: 0,
            thinking_delay_ms: 0,
            min_chunk_size: 100,
            max_chunk_size: 100,
        }
    }

    /// Slow streaming for visual testing
    pub fn slow() -> Self {
        Self {
            tokens_per_second: 10,
            thinking_delay_ms: 1000,
            min_chunk_size: 1,
            max_chunk_size: 3,
        }
    }
}

/// Streaming response state
pub struct StreamingResponse {
    /// Full response text
    full_text: String,

    /// Current character position
    position: usize,

    /// Streaming configuration
    config: StreamingConfig,

    /// Tokens streamed so far
    tokens_streamed: u32,

    /// Whether streaming is complete
    complete: bool,

    /// Clock for timing
    clock: ClockHandle,
}

impl StreamingResponse {
    /// Create a new streaming response
    pub fn new(text: String, config: StreamingConfig, clock: ClockHandle) -> Self {
        // Calculate token count based on text length (rough: 4 chars per token)
        let tokens_streamed = (text.len() / 4).max(1) as u32;

        Self {
            full_text: text,
            position: 0,
            config,
            tokens_streamed,
            complete: false,
            clock,
        }
    }

    /// Get the next chunk of text
    pub async fn next_chunk(&mut self) -> Option<String> {
        if self.complete {
            return None;
        }

        if self.position >= self.full_text.len() {
            self.complete = true;
            return None;
        }

        // Calculate chunk size
        let remaining = self.full_text.len() - self.position;
        let chunk_size = if self.config.tokens_per_second == 0 {
            remaining // Instant mode
        } else {
            // Vary chunk size slightly for natural feel
            let base = (self.config.min_chunk_size + self.config.max_chunk_size) / 2;
            base.min(remaining)
        };

        // Extract chunk
        let end = self.position + chunk_size;
        let chunk = self.full_text[self.position..end].to_string();
        self.position = end;

        // Delay for streaming effect
        if self.config.tokens_per_second > 0 {
            let delay_ms = 1000 / self.config.tokens_per_second as u64;
            self.clock.sleep(Duration::from_millis(delay_ms)).await;
        }

        Some(chunk)
    }

    /// Get the number of tokens streamed
    pub fn tokens_streamed(&self) -> u32 {
        self.tokens_streamed
    }

    /// Check if streaming is complete
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Get the full text (for immediate display)
    pub fn full_text(&self) -> &str {
        &self.full_text
    }

    /// Skip to end (for interrupt handling)
    pub fn skip_to_end(&mut self) {
        self.position = self.full_text.len();
        self.complete = true;
    }
}

/// Token stream for async iteration
pub struct TokenStream {
    response: StreamingResponse,
}

impl TokenStream {
    pub fn new(response: StreamingResponse) -> Self {
        Self { response }
    }

    /// Convert to a channel-based stream for integration with event loop
    pub fn into_channel(mut self) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(async move {
            while let Some(chunk) = self.response.next_chunk().await {
                if tx.send(chunk).await.is_err() {
                    break;
                }
            }
        });

        rx
    }
}

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
