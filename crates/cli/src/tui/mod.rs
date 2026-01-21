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
mod screenshot;
pub mod separator;
pub mod shortcuts;
pub mod slash_menu;
mod streaming;
mod test_helpers;

#[cfg(test)]
#[path = "responsive_tests.rs"]
mod responsive_tests;

pub mod widgets;

pub use app::{
    AppMode, ExitHint, ExitReason, PermissionChoice, PermissionRequest, RenderState, StatusInfo,
    TuiApp, TuiConfig,
};
pub use screenshot::{LineDiff, Screenshot, ScreenshotCapture, ScreenshotMetadata};
pub use slash_menu::{filter_commands, fuzzy_matches, SlashCommand, SlashMenuState, COMMANDS};
pub use streaming::{StreamingConfig, StreamingResponse, TokenStream};
pub use test_helpers::{TuiAppState, TuiTestHarness};
