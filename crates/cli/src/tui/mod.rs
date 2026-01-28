// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Terminal user interface for Claudeless.
//!
//! This module provides a simplified TUI that matches Claude Code's
//! visual layout for testing oj's integration with Claude's interactive mode.
//!
//! The TUI is built using the iocraft framework with a declarative component model.
//! All rendering is handled in app.rs via the element! macro.

mod app;
mod colors;
pub mod separator;
pub mod shortcuts;
pub mod slash_menu;
pub mod spinner;
mod streaming;

#[cfg(test)]
#[path = "responsive_tests.rs"]
mod responsive_tests;

pub mod widgets;

pub use app::{ExitReason, TuiApp, TuiConfig};
