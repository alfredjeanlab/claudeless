// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Interaction capture and recording for test assertions.
//!
//! This crate provides utilities for capturing and recording CLI interactions,
//! useful for testing and debugging claudeless scenarios.

mod duration_serde;
mod interaction;
mod log;

pub use interaction::{CapturedArgs, CapturedInteraction, CapturedOutcome};
pub use log::CaptureLog;
