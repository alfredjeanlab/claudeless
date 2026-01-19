// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI testing utilities.

use super::app::{AppMode, ExitReason};
use crate::time::FakeClock;

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
        match self.app_state.mode {
            AppMode::Input if self.app_state.input_buffer.is_empty() => {
                self.app_state.should_exit = true;
                self.app_state.exit_reason = Some(ExitReason::Interrupted);
            }
            AppMode::Input => {
                self.app_state.input_buffer.clear();
                self.app_state.cursor_pos = 0;
            }
            _ => {}
        }
    }

    /// Simulate pressing Ctrl+D
    pub fn press_ctrl_d(&mut self) {
        if self.app_state.input_buffer.is_empty() {
            self.app_state.should_exit = true;
            self.app_state.exit_reason = Some(ExitReason::UserQuit);
        }
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
}

impl Default for TuiTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = TuiTestHarness::new();
        assert!(matches!(harness.app_state.mode, AppMode::Input));
        assert!(harness.app_state.input_buffer.is_empty());
    }

    #[test]
    fn test_harness_type_input() {
        let mut harness = TuiTestHarness::new();
        harness.type_input("test");
        assert_eq!(harness.app_state.input_buffer, "test");
        assert_eq!(harness.app_state.cursor_pos, 4);
    }

    #[test]
    fn test_harness_enter_adds_to_history() {
        let mut harness = TuiTestHarness::new();
        harness.type_input("command");
        harness.press_enter();
        assert_eq!(harness.app_state.history.len(), 1);
        assert_eq!(harness.app_state.history[0], "command");
        assert!(harness.app_state.input_buffer.is_empty());
    }

    #[test]
    fn test_ctrl_c_clears_input() {
        let mut harness = TuiTestHarness::new();

        harness.type_input("Some text");
        assert!(!harness.app_state.input_buffer.is_empty());

        harness.press_ctrl_c();
        assert!(harness.app_state.input_buffer.is_empty());
        assert!(!harness.app_state.should_exit);
    }

    #[test]
    fn test_ctrl_c_exits_on_empty() {
        let mut harness = TuiTestHarness::new();

        harness.press_ctrl_c();
        assert!(harness.app_state.should_exit);
        assert!(matches!(
            harness.app_state.exit_reason,
            Some(ExitReason::Interrupted)
        ));
    }

    #[test]
    fn test_history_navigation() {
        let mut harness = TuiTestHarness::new();

        harness.type_input("first");
        harness.press_enter();

        harness.type_input("second");
        harness.press_enter();

        assert!(harness.app_state.input_buffer.is_empty());
        assert_eq!(harness.app_state.history.len(), 2);
    }

    #[test]
    fn test_backspace() {
        let mut harness = TuiTestHarness::new();

        harness.type_input("hello");
        harness.press_backspace();
        assert_eq!(harness.app_state.input_buffer, "hell");
        assert_eq!(harness.app_state.cursor_pos, 4);
    }

    #[test]
    fn test_cursor_movement() {
        let mut harness = TuiTestHarness::new();

        harness.type_input("hello");
        assert_eq!(harness.app_state.cursor_pos, 5);

        harness.press_left();
        assert_eq!(harness.app_state.cursor_pos, 4);

        harness.press_left();
        harness.press_left();
        assert_eq!(harness.app_state.cursor_pos, 2);

        harness.press_right();
        assert_eq!(harness.app_state.cursor_pos, 3);

        harness.press_home();
        assert_eq!(harness.app_state.cursor_pos, 0);

        harness.press_end();
        assert_eq!(harness.app_state.cursor_pos, 5);
    }
}
