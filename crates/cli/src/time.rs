// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Time abstraction for deterministic testing.
//!
//! This module provides a `Clock` trait and `FakeClock` implementation that allows
//! tests to control time progression without wall-clock delays.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Clock trait for time abstraction
pub trait Clock: Send + Sync {
    /// Get current time as milliseconds since epoch
    fn now_millis(&self) -> u64;

    /// Sleep for a duration
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;

    /// Get current time as Duration since epoch
    fn now(&self) -> Duration {
        Duration::from_millis(self.now_millis())
    }
}

/// Real clock using system time
#[derive(Clone, Debug, Default)]
pub struct SystemClock;

impl SystemClock {
    /// Create a new system clock
    pub fn new() -> Self {
        Self
    }
}

impl Clock for SystemClock {
    fn now_millis(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(tokio::time::sleep(duration))
    }
}

/// Fake clock for testing with controllable time
#[derive(Clone, Debug)]
pub struct FakeClock {
    /// Current time in milliseconds
    current_millis: Arc<AtomicU64>,

    /// Whether to auto-advance on sleep
    auto_advance: bool,
}

impl FakeClock {
    /// Create a new fake clock starting at a given time
    pub fn new(start_millis: u64) -> Self {
        Self {
            current_millis: Arc::new(AtomicU64::new(start_millis)),
            auto_advance: true,
        }
    }

    /// Create a fake clock starting at Unix epoch
    pub fn at_epoch() -> Self {
        Self::new(0)
    }

    /// Create a fake clock starting at "now"
    pub fn at_now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self::new(now)
    }

    /// Set whether sleep auto-advances time
    pub fn set_auto_advance(&mut self, auto_advance: bool) {
        self.auto_advance = auto_advance;
    }

    /// Create a clone with auto-advance disabled
    pub fn without_auto_advance(&self) -> Self {
        Self {
            current_millis: Arc::clone(&self.current_millis),
            auto_advance: false,
        }
    }

    /// Advance time by a duration
    pub fn advance(&self, duration: Duration) {
        self.current_millis
            .fetch_add(duration.as_millis() as u64, Ordering::SeqCst);
    }

    /// Advance time by milliseconds
    pub fn advance_ms(&self, ms: u64) {
        self.current_millis.fetch_add(ms, Ordering::SeqCst);
    }

    /// Advance time by seconds
    pub fn advance_secs(&self, secs: u64) {
        self.advance_ms(secs * 1000);
    }

    /// Set absolute time
    pub fn set(&self, millis: u64) {
        self.current_millis.store(millis, Ordering::SeqCst);
    }

    /// Set absolute time from a Duration
    pub fn set_duration(&self, duration: Duration) {
        self.set(duration.as_millis() as u64);
    }

    /// Check if auto-advance is enabled
    pub fn auto_advance(&self) -> bool {
        self.auto_advance
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::at_epoch()
    }
}

impl Clock for FakeClock {
    fn now_millis(&self) -> u64 {
        self.current_millis.load(Ordering::SeqCst)
    }

    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        if self.auto_advance {
            self.advance(duration);
        }
        // No actual sleep - return immediately
        Box::pin(async {})
    }
}

/// Clock handle that can be either real or fake
#[derive(Clone)]
pub enum ClockHandle {
    System(SystemClock),
    Fake(FakeClock),
}

impl ClockHandle {
    /// Create a system clock handle
    pub fn system() -> Self {
        Self::System(SystemClock)
    }

    /// Create a fake clock handle starting at "now"
    pub fn fake() -> Self {
        Self::Fake(FakeClock::at_now())
    }

    /// Create a fake clock handle at epoch
    pub fn fake_at_epoch() -> Self {
        Self::Fake(FakeClock::at_epoch())
    }

    /// Create a fake clock handle at a specific time
    pub fn fake_at(millis: u64) -> Self {
        Self::Fake(FakeClock::new(millis))
    }

    /// Get as fake clock for manipulation (returns None for system clock)
    pub fn as_fake(&self) -> Option<&FakeClock> {
        match self {
            Self::Fake(f) => Some(f),
            Self::System(_) => None,
        }
    }

    /// Check if this is a fake clock
    pub fn is_fake(&self) -> bool {
        matches!(self, Self::Fake(_))
    }

    /// Check if this is a system clock
    pub fn is_system(&self) -> bool {
        matches!(self, Self::System(_))
    }
}

impl Clock for ClockHandle {
    fn now_millis(&self) -> u64 {
        match self {
            Self::System(c) => c.now_millis(),
            Self::Fake(c) => c.now_millis(),
        }
    }

    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        match self {
            Self::System(c) => c.sleep(duration),
            Self::Fake(c) => c.sleep(duration),
        }
    }
}

impl Default for ClockHandle {
    fn default() -> Self {
        Self::system()
    }
}

#[cfg(test)]
#[path = "time_tests.rs"]
mod tests;
