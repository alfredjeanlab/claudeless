// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Claude CLI Simulator
//!
//! A test crate that simulates the `claude` CLI for integration testing.
//! Provides a controllable test double that responds to the same CLI interface
//! as real Claude, enabling deterministic integration testing without API costs.
//!
//! For scenario authoring, see the **[Scenario Reference](crate::docs::scenarios)** for complete scenario file documentation.
//!
#![doc = include_str!("../docs/USAGE.md")]

/// Documentation modules for docs.rs
pub mod docs {
    /// Scenario file reference - patterns, responses, failures, and tool execution.
    #[doc = include_str!("../docs/SCENARIOS.md")]
    pub mod scenarios {}
}

// Internal modules - pub for binary access, hidden from docs
#[doc(hidden)]
pub mod ansi;
#[doc(hidden)]
pub mod api;
/// Interaction capture and recording for test assertions.
pub mod capture;
#[doc(hidden)]
pub mod cli;
#[doc(hidden)]
pub mod config;
/// Event type string constants for JSONL and stream-json output formats.
pub mod event_types;
#[doc(hidden)]
pub mod failure;
#[doc(hidden)]
pub mod hooks;
#[doc(hidden)]
pub mod inspect;
#[doc(hidden)]
pub mod mcp;
#[doc(hidden)]
pub mod output;
#[doc(hidden)]
pub mod permission;
#[doc(hidden)]
pub mod scenario;
#[doc(hidden)]
pub mod session;
#[doc(hidden)]
pub mod state;
#[doc(hidden)]
pub mod time;
#[doc(hidden)]
pub mod tools;
#[doc(hidden)]
pub mod tui;
#[doc(hidden)]
pub mod usage;
#[doc(hidden)]
pub mod validation;
