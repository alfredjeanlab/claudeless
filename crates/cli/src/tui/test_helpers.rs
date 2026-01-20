// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI testing utilities.

use super::app::{AppMode, ExitHint, ExitReason};
use crate::time::{Clock, FakeClock};

/// Exit hint timeout in milliseconds (2 seconds)
const EXIT_HINT_TIMEOUT_MS: u64 = 2000;

/// App state without terminal (for headless testing)
pub struct TuiAppState {
    pub mode: AppMode,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub response_content: String,
    pub is_streaming: bool,
    pub history: Vec<String>,
    pub should_exit: bool,
    pub exit_reason: Option<ExitReason>,
    pub exit_hint: Option<ExitHint>,
    pub exit_hint_shown_at: Option<u64>,
}

/// Test harness for TUI testing
pub struct TuiTestHarness {
    /// Fake clock for deterministic timing
    clock: FakeClock,

    /// App state (without real terminal)
    pub app_state: TuiAppState,
}

impl TuiTestHarness {
    /// Create a new test harness
    pub fn new() -> Self {
        Self {
            clock: FakeClock::at_epoch(),
            app_state: TuiAppState {
                mode: AppMode::Input,
                input_buffer: String::new(),
                cursor_pos: 0,
                response_content: String::new(),
                is_streaming: false,
                history: Vec::new(),
                should_exit: false,
                exit_reason: None,
                exit_hint: None,
                exit_hint_shown_at: None,
            },
        }
    }

    /// Type a string into the input
    pub fn type_input(&mut self, text: &str) {
        for c in text.chars() {
            self.app_state
                .input_buffer
                .insert(self.app_state.cursor_pos, c);
            self.app_state.cursor_pos += 1;
        }
    }

    /// Simulate pressing Enter
    pub fn press_enter(&mut self) {
        if !self.app_state.input_buffer.is_empty() {
            let input = std::mem::take(&mut self.app_state.input_buffer);
            self.app_state.cursor_pos = 0;
            self.app_state.history.push(input);
        }
    }

    /// Simulate pressing Ctrl+C
    pub fn press_ctrl_c(&mut self) {
        if self.app_state.mode != AppMode::Input {
            return;
        }

        let now = self.clock.now_millis();
        let within_timeout = self.app_state.exit_hint == Some(ExitHint::CtrlC)
            && self
                .app_state
                .exit_hint_shown_at
                .map(|t| now.saturating_sub(t) < EXIT_HINT_TIMEOUT_MS)
                .unwrap_or(false);

        if within_timeout {
            // Second Ctrl+C within timeout - exit
            self.app_state.should_exit = true;
            self.app_state.exit_reason = Some(ExitReason::Interrupted);
        } else {
            // First Ctrl+C - clear input (if any) and show hint
            self.app_state.input_buffer.clear();
            self.app_state.cursor_pos = 0;
            self.app_state.exit_hint = Some(ExitHint::CtrlC);
            self.app_state.exit_hint_shown_at = Some(now);
        }
    }

    /// Simulate pressing Ctrl+D
    pub fn press_ctrl_d(&mut self) {
        if self.app_state.input_buffer.is_empty() {
            let now = self.clock.now_millis();
            let within_timeout = self.app_state.exit_hint == Some(ExitHint::CtrlD)
                && self
                    .app_state
                    .exit_hint_shown_at
                    .map(|t| now.saturating_sub(t) < EXIT_HINT_TIMEOUT_MS)
                    .unwrap_or(false);

            if within_timeout {
                // Second Ctrl+D within timeout - exit
                self.app_state.should_exit = true;
                self.app_state.exit_reason = Some(ExitReason::UserQuit);
            } else {
                // First Ctrl+D - show hint
                self.app_state.exit_hint = Some(ExitHint::CtrlD);
                self.app_state.exit_hint_shown_at = Some(now);
            }
        }
        // With text in input: ignored (do nothing)
    }

    /// Simulate pressing Backspace
    pub fn press_backspace(&mut self) {
        if self.app_state.cursor_pos > 0 {
            self.app_state.cursor_pos -= 1;
            self.app_state
                .input_buffer
                .remove(self.app_state.cursor_pos);
        }
    }

    /// Simulate pressing Left arrow
    pub fn press_left(&mut self) {
        if self.app_state.cursor_pos > 0 {
            self.app_state.cursor_pos -= 1;
        }
    }

    /// Simulate pressing Right arrow
    pub fn press_right(&mut self) {
        if self.app_state.cursor_pos < self.app_state.input_buffer.len() {
            self.app_state.cursor_pos += 1;
        }
    }

    /// Simulate pressing Home
    pub fn press_home(&mut self) {
        self.app_state.cursor_pos = 0;
    }

    /// Simulate pressing End
    pub fn press_end(&mut self) {
        self.app_state.cursor_pos = self.app_state.input_buffer.len();
    }

    /// Simulate pressing Escape
    pub fn press_escape(&mut self) {
        self.app_state.input_buffer.clear();
        self.app_state.cursor_pos = 0;
    }

    /// Set response content for testing
    pub fn set_response(&mut self, content: &str) {
        self.app_state.response_content = content.to_string();
    }

    /// Get the fake clock for time manipulation
    pub fn clock(&self) -> &FakeClock {
        &self.clock
    }

    /// Advance time
    pub fn advance_time(&self, ms: u64) {
        self.clock.advance_ms(ms);
    }

    /// Check if exit hint has timed out and clear it
    pub fn check_exit_hint_timeout(&mut self) {
        if let (Some(_hint), Some(shown_at)) =
            (&self.app_state.exit_hint, self.app_state.exit_hint_shown_at)
        {
            let now = self.clock.now_millis();
            if now.saturating_sub(shown_at) >= EXIT_HINT_TIMEOUT_MS {
                self.app_state.exit_hint = None;
                self.app_state.exit_hint_shown_at = None;
            }
        }
    }

    /// Get current exit hint
    pub fn exit_hint(&self) -> Option<ExitHint> {
        self.app_state.exit_hint.clone()
    }
}

impl Default for TuiTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "test_helpers_tests.rs"]
mod tests;
