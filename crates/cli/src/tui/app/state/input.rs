// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Input editing state for the TUI application.

/// Input editing state
#[derive(Clone, Debug, Default)]
pub struct InputState {
    /// Current input buffer
    pub buffer: String,
    /// Cursor position in input
    pub cursor_pos: usize,
    /// Command history
    pub history: Vec<String>,
    /// Current history navigation index
    pub history_index: Option<usize>,
    /// Undo stack for input changes
    pub undo_stack: Vec<String>,
    /// Stashed input for later restoration
    pub stash: Option<String>,
    /// Show stash indicator
    pub show_stash_indicator: bool,
    /// Shell mode active
    pub shell_mode: bool,
}

impl InputState {
    /// Clear input buffer and cursor
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_pos = 0;
        self.undo_stack.clear();
    }

    /// Submit input and return the submitted text
    pub fn submit(&mut self) -> String {
        let input = std::mem::take(&mut self.buffer);
        self.cursor_pos = 0;
        self.undo_stack.clear();
        if !input.is_empty() {
            self.history.push(input.clone());
        }
        self.history_index = None;
        input
    }

    /// Insert a character at the current cursor position
    pub fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
    }

    /// Delete the character before the cursor (backspace)
    pub fn backspace(&mut self) -> bool {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.buffer.remove(self.cursor_pos);
            true
        } else {
            false
        }
    }

    /// Delete the character at the cursor position
    pub fn delete(&mut self) -> bool {
        if self.cursor_pos < self.buffer.len() {
            self.buffer.remove(self.cursor_pos);
            true
        } else {
            false
        }
    }

    /// Move cursor left
    pub fn move_left(&mut self) -> bool {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            true
        } else {
            false
        }
    }

    /// Move cursor right
    pub fn move_right(&mut self) -> bool {
        if self.cursor_pos < self.buffer.len() {
            self.cursor_pos += 1;
            true
        } else {
            false
        }
    }

    /// Move cursor to start
    pub fn move_to_start(&mut self) {
        self.cursor_pos = 0;
    }

    /// Move cursor to end
    pub fn move_to_end(&mut self) {
        self.cursor_pos = self.buffer.len();
    }

    /// Push current state to undo stack
    pub fn push_undo_snapshot(&mut self) {
        if self.undo_stack.last() != Some(&self.buffer) {
            self.undo_stack.push(self.buffer.clone());
        }
    }

    /// Pop from undo stack and restore
    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo_stack.pop() {
            self.buffer = previous;
            self.cursor_pos = self.cursor_pos.min(self.buffer.len());
            true
        } else {
            false
        }
    }

    /// Stash current input
    pub fn stash(&mut self) -> bool {
        if !self.buffer.is_empty() {
            self.stash = Some(std::mem::take(&mut self.buffer));
            self.cursor_pos = 0;
            self.show_stash_indicator = true;
            true
        } else {
            false
        }
    }

    /// Restore stashed input
    pub fn restore_stash(&mut self) -> bool {
        if let Some(stashed) = self.stash.take() {
            self.buffer = stashed;
            self.cursor_pos = self.buffer.len();
            self.show_stash_indicator = false;
            true
        } else {
            false
        }
    }

    /// Navigate history
    pub fn navigate_history(&mut self, direction: i32) -> bool {
        if self.history.is_empty() {
            return false;
        }

        let new_index = match self.history_index {
            None if direction < 0 => Some(self.history.len() - 1),
            None => return false,
            Some(i) if direction < 0 && i > 0 => Some(i - 1),
            Some(i) if direction > 0 && i < self.history.len() - 1 => Some(i + 1),
            Some(_) if direction > 0 => {
                // Past end of history, clear input
                self.history_index = None;
                self.buffer.clear();
                self.cursor_pos = 0;
                self.undo_stack.clear();
                return true;
            }
            Some(i) => Some(i),
        };

        if let Some(idx) = new_index {
            self.history_index = Some(idx);
            self.buffer = self.history[idx].clone();
            self.cursor_pos = self.buffer.len();
            self.undo_stack.clear();
            true
        } else {
            false
        }
    }

    /// Delete word before cursor (Ctrl+W behavior)
    pub fn delete_word_before_cursor(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }

        let before = &self.buffer[..self.cursor_pos];
        let trimmed = before.trim_end();
        let word_start = trimmed
            .rfind(char::is_whitespace)
            .map(|i| i + 1)
            .unwrap_or(0);

        self.buffer = format!(
            "{}{}",
            &self.buffer[..word_start],
            &self.buffer[self.cursor_pos..]
        );
        self.cursor_pos = word_start;
    }

    /// Clear line before cursor (Ctrl+U behavior)
    pub fn clear_before_cursor(&mut self) {
        self.buffer = self.buffer[self.cursor_pos..].to_string();
        self.cursor_pos = 0;
    }

    /// Clear line after cursor (Ctrl+K behavior)
    pub fn clear_after_cursor(&mut self) {
        self.buffer.truncate(self.cursor_pos);
    }
}
