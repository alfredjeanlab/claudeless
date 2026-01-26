// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::time::FakeClock;

#[tokio::test]
async fn test_instant_streaming() {
    let clock = ClockHandle::Fake(FakeClock::at_epoch());
    let config = StreamingConfig::instant();
    let mut response = StreamingResponse::new("Hello, world!".to_string(), config, clock);

    let chunk = response.next_chunk().await;
    assert_eq!(chunk, Some("Hello, world!".to_string()));

    let chunk = response.next_chunk().await;
    assert!(chunk.is_none());
    assert!(response.is_complete());
}

#[tokio::test]
async fn test_chunked_streaming() {
    let clock = ClockHandle::Fake(FakeClock::at_epoch());
    let config = StreamingConfig {
        tokens_per_second: 100,
        thinking_delay_ms: 0,
        min_chunk_size: 2,
        max_chunk_size: 4,
    };
    let mut response = StreamingResponse::new("Hello!".to_string(), config, clock);

    let mut chunks = Vec::new();
    while let Some(chunk) = response.next_chunk().await {
        chunks.push(chunk);
    }

    let full: String = chunks.concat();
    assert_eq!(full, "Hello!");
}

#[test]
fn test_full_text() {
    let clock = ClockHandle::Fake(FakeClock::at_epoch());
    let config = StreamingConfig::default();
    let response = StreamingResponse::new("Test message".to_string(), config, clock);

    assert_eq!(response.full_text(), "Test message");
}

#[test]
fn test_tokens_streamed() {
    let clock = ClockHandle::Fake(FakeClock::at_epoch());
    let config = StreamingConfig::default();
    let response = StreamingResponse::new(
        "A longer message for token counting".to_string(),
        config,
        clock,
    );

    // ~35 chars / 4 = ~8 tokens
    assert!(response.tokens_streamed() >= 8);
}

#[tokio::test]
async fn test_skip_to_end() {
    let clock = ClockHandle::Fake(FakeClock::at_epoch());
    let config = StreamingConfig::default();
    let mut response = StreamingResponse::new("Hello, world!".to_string(), config, clock);

    response.skip_to_end();
    assert!(response.is_complete());

    let chunk = response.next_chunk().await;
    assert!(chunk.is_none());
}
