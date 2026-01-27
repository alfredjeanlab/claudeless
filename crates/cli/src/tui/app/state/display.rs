// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Display/rendering state for the TUI application.

use crate::tui::slash_menu::SlashMenuState;

use super::super::types::{ExitHint, DEFAULT_TERMINAL_WIDTH};

/// Display/rendering state
#[derive(Clone, Debug, Default)]
pub struct DisplayState {
    /// Current response content
    pub response_content: String,
    /// Whether response is streaming
    pub is_streaming: bool,
    /// Whether current content is command output
    pub is_command_output: bool,
    /// Conversation history display
    pub conversation_display: String,
    /// Whether conversation was compacted
    pub is_compacted: bool,
    /// Terminal width
    pub terminal_width: u16,
    /// Show shortcuts panel
    pub show_shortcuts_panel: bool,
    /// Slash menu state
    pub slash_menu: Option<SlashMenuState>,
    /// Exit hint
    pub exit_hint: Option<ExitHint>,
    /// When exit hint was shown (milliseconds from clock)
    pub exit_hint_shown_at: Option<u64>,
    /// Current spinner animation frame index
    pub spinner_frame: usize,
    /// Current spinner verb (e.g., "Thinking", "Pondering")
    pub spinner_verb: String,
}

impl DisplayState {
    /// Create new display state with default terminal width
    pub fn new() -> Self {
        Self {
            terminal_width: crossterm::terminal::size()
                .map(|(w, _)| w)
                .unwrap_or(DEFAULT_TERMINAL_WIDTH),
            ..Default::default()
        }
    }

    /// Clear response content
    pub fn clear_response(&mut self) {
        self.response_content.clear();
        self.is_streaming = false;
        self.is_command_output = false;
    }

    /// Set response content
    pub fn set_response(&mut self, content: String, is_command_output: bool) {
        self.response_content = content;
        self.is_command_output = is_command_output;
    }

    /// Append to conversation display
    pub fn append_to_conversation(&mut self, content: &str) {
        if !self.conversation_display.is_empty() {
            self.conversation_display.push_str("\n\n");
        }
        self.conversation_display.push_str(content);
    }

    /// Clear conversation display
    pub fn clear_conversation(&mut self) {
        self.conversation_display.clear();
        self.is_compacted = false;
    }

    /// Show exit hint
    pub fn show_exit_hint(&mut self, hint: ExitHint, timestamp: u64) {
        self.exit_hint = Some(hint);
        self.exit_hint_shown_at = Some(timestamp);
    }

    /// Clear exit hint
    pub fn clear_exit_hint(&mut self) {
        self.exit_hint = None;
        self.exit_hint_shown_at = None;
    }

    /// Update slash menu based on input buffer
    pub fn update_slash_menu(&mut self, input_buffer: &str) {
        if let Some(suffix) = input_buffer.strip_prefix('/') {
            let filter = suffix.to_string();
            if let Some(menu) = self.slash_menu.as_mut() {
                menu.set_filter(filter);
            } else {
                let mut menu = SlashMenuState::new();
                menu.set_filter(filter);
                self.slash_menu = Some(menu);
            }
        } else {
            self.slash_menu = None;
        }
    }

    /// Close slash menu
    pub fn close_slash_menu(&mut self) {
        self.slash_menu = None;
    }
}
