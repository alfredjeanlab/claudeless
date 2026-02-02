// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Thinking toggle dialog widget.
//!
//! Shown when user presses Meta+t to toggle extended thinking mode.
//!
//! Note: Rendering has been moved to app.rs as part of the iocraft migration.
//! This module keeps the types used by the app component.

/// Thinking mode selection
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum ThinkingMode {
    Enabled = 0,
    Disabled = 1,
}

/// Thinking toggle dialog state
#[derive(Clone, Debug)]
pub struct ThinkingDialog {
    pub selected: ThinkingMode,
    pub current: ThinkingMode,
}

impl ThinkingDialog {
    pub fn new(current_enabled: bool) -> Self {
        let current = if current_enabled {
            ThinkingMode::Enabled
        } else {
            ThinkingMode::Disabled
        };
        Self {
            selected: current,
            current,
        }
    }
}

#[cfg(test)]
#[path = "thinking_tests.rs"]
mod tests;
