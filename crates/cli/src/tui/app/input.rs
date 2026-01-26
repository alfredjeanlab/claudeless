// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Input mode key handling for the TUI application.

use iocraft::prelude::*;

use crate::time::Clock;
use crate::tui::slash_menu::SlashMenuState;
use crate::tui::widgets::permission::PermissionSelection;
use crate::tui::widgets::thinking::ThinkingDialog;

use super::state::{TuiAppState, TuiAppStateInner};
use super::types::{AppMode, ExitHint, ExitReason, EXIT_HINT_TIMEOUT_MS};

/// Matches a control key that may be encoded as raw ASCII or as modifier+char.
///
/// Terminal encoding varies - some send raw ASCII codes (e.g., Ctrl+S as 0x13),
/// while others send the character with CONTROL modifier. This macro handles both.
macro_rules! ctrl_key {
    // Ctrl+Z: ASCII 0x1A or 'z' with CONTROL
    (z, $modifiers:expr, $code:expr) => {
        matches!($code, KeyCode::Char('\x1a'))
            || (matches!($code, KeyCode::Char('z')) && $modifiers.contains(KeyModifiers::CONTROL))
    };
    // Ctrl+S: ASCII 0x13 or 's' with CONTROL
    (s, $modifiers:expr, $code:expr) => {
        matches!($code, KeyCode::Char('\x13'))
            || (matches!($code, KeyCode::Char('s')) && $modifiers.contains(KeyModifiers::CONTROL))
    };
    // Ctrl+_: ASCII 0x1F or '_' with CONTROL or '/' with CONTROL (same ASCII)
    (underscore, $modifiers:expr, $code:expr) => {
        matches!($code, KeyCode::Char('\x1f'))
            || (matches!($code, KeyCode::Char('_')) && $modifiers.contains(KeyModifiers::CONTROL))
            || (matches!($code, KeyCode::Char('/')) && $modifiers.contains(KeyModifiers::CONTROL))
    };
}

impl TuiAppState {
    /// Handle key event based on current mode
    pub fn handle_key_event(&self, key: KeyEvent) {
        let mode = self.mode();
        match mode {
            AppMode::Trust => self.handle_trust_key(key),
            AppMode::Input => self.handle_input_key(key),
            AppMode::Permission => self.handle_permission_key(key),
            AppMode::Responding | AppMode::Thinking => self.handle_responding_key(key),
            AppMode::ThinkingToggle => self.handle_thinking_key(key),
            AppMode::TasksDialog => self.handle_tasks_key(key),
            AppMode::ModelPicker => self.handle_model_picker_key(key),
            AppMode::ExportDialog => self.handle_export_dialog_key(key),
            AppMode::HelpDialog => self.handle_help_dialog_key(key),
            AppMode::HooksDialog => self.handle_hooks_dialog_key(key),
            AppMode::MemoryDialog => self.handle_memory_dialog_key(key),
        }
    }

    /// Handle key events in input mode
    pub(super) fn handle_input_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        // Handle slash menu navigation when menu is open
        if inner.slash_menu.is_some() {
            match key.code {
                KeyCode::Down => {
                    if let Some(ref mut menu) = inner.slash_menu {
                        menu.select_next();
                    }
                    return;
                }
                KeyCode::Up => {
                    if let Some(ref mut menu) = inner.slash_menu {
                        menu.select_prev();
                    }
                    return;
                }
                KeyCode::Tab => {
                    // Complete the selected command
                    if let Some(ref menu) = inner.slash_menu {
                        if let Some(cmd) = menu.selected_command() {
                            inner.input_buffer = cmd.full_name();
                            inner.cursor_pos = inner.input_buffer.len();
                        }
                    }
                    inner.slash_menu = None; // Close menu
                    return;
                }
                KeyCode::Esc => {
                    // Close menu but keep text, show "Esc to clear again" hint
                    inner.slash_menu = None;
                    let now = inner.clock.now_millis();
                    inner.exit_hint = Some(ExitHint::Escape);
                    inner.exit_hint_shown_at = Some(now);
                    return;
                }
                _ => {
                    // Fall through to normal key handling
                }
            }
        }

        match (key.modifiers, key.code) {
            // Ctrl+C - Interrupt
            (m, KeyCode::Char('c')) if m.contains(KeyModifiers::CONTROL) => {
                drop(inner);
                self.handle_interrupt();
            }

            // Ctrl+Z - Suspend process
            // Note: Ctrl+Z is encoded as ASCII 0x1A (substitute) or Char('z') with CONTROL
            _ if ctrl_key!(z, key.modifiers, key.code) => {
                inner.should_exit = true;
                inner.exit_reason = Some(ExitReason::Suspended);
            }

            // Ctrl+D - Exit (only on empty input)
            (m, KeyCode::Char('d')) if m.contains(KeyModifiers::CONTROL) => {
                if inner.input_buffer.is_empty() {
                    let now = inner.clock.now_millis();
                    let within_timeout = inner.exit_hint == Some(ExitHint::CtrlD)
                        && inner
                            .exit_hint_shown_at
                            .map(|t| now.saturating_sub(t) < EXIT_HINT_TIMEOUT_MS)
                            .unwrap_or(false);

                    if within_timeout {
                        // Second Ctrl+D within timeout - exit
                        inner.should_exit = true;
                        inner.exit_reason = Some(ExitReason::UserQuit);
                    } else {
                        // First Ctrl+D - show hint
                        inner.exit_hint = Some(ExitHint::CtrlD);
                        inner.exit_hint_shown_at = Some(now);
                    }
                }
                // With text in input: ignored (do nothing)
            }

            // Ctrl+L - Clear screen (keep input)
            (m, KeyCode::Char('l')) if m.contains(KeyModifiers::CONTROL) => {
                inner.response_content.clear();
            }

            // Meta+t (Alt+t) - Toggle thinking mode
            (m, KeyCode::Char('t'))
                if m.contains(KeyModifiers::META) || m.contains(KeyModifiers::ALT) =>
            {
                inner.thinking_dialog = Some(ThinkingDialog::new(inner.thinking_enabled));
                inner.mode = AppMode::ThinkingToggle;
            }

            // Meta+p (Alt+p) - Open model picker
            (m, KeyCode::Char('p'))
                if m.contains(KeyModifiers::META) || m.contains(KeyModifiers::ALT) =>
            {
                inner.model_picker_dialog = Some(crate::tui::widgets::ModelPickerDialog::new(
                    &inner.status.model,
                ));
                inner.mode = AppMode::ModelPicker;
            }

            // Ctrl+T - Show todos (only when todos exist)
            (m, KeyCode::Char('t')) if m.contains(KeyModifiers::CONTROL) => {
                if !inner.todos.is_empty() {
                    inner.response_content = Self::format_todos(&inner.todos);
                    inner.is_command_output = true;
                    inner.conversation_display = "Todo List".to_string();
                }
                // When no todos, do nothing (no visible change)
            }

            // Shift+Tab - Cycle permission mode
            (m, KeyCode::BackTab) if m.contains(KeyModifiers::SHIFT) => {
                inner.permission_mode = inner
                    .permission_mode
                    .cycle_next(inner.allow_bypass_permissions);
            }

            // Enter - Submit input
            (_, KeyCode::Enter) => {
                // Close slash menu on enter
                inner.slash_menu = None;
                // Clear exit hint on Enter
                inner.exit_hint = None;
                inner.exit_hint_shown_at = None;
                if !inner.input_buffer.is_empty() {
                    drop(inner);
                    self.submit_input();
                }
            }

            // Escape - Dismiss shortcuts panel first, then exit shell mode, then check for clear
            // Note: slash menu escape is handled above in the slash_menu.is_some() block
            (_, KeyCode::Esc) => {
                if inner.show_shortcuts_panel {
                    // First priority: dismiss shortcuts panel
                    inner.show_shortcuts_panel = false;
                } else if inner.shell_mode {
                    // Second priority: exit shell mode
                    inner.shell_mode = false;
                    inner.input_buffer.clear();
                    inner.cursor_pos = 0;
                } else if !inner.input_buffer.is_empty() {
                    // Input has text - check for double-tap
                    let now = inner.clock.now_millis();
                    let within_timeout = inner.exit_hint == Some(ExitHint::Escape)
                        && inner
                            .exit_hint_shown_at
                            .map(|t| now.saturating_sub(t) < EXIT_HINT_TIMEOUT_MS)
                            .unwrap_or(false);

                    if within_timeout {
                        // Second Escape within timeout - clear input
                        inner.input_buffer.clear();
                        inner.cursor_pos = 0;
                        inner.exit_hint = None;
                        inner.exit_hint_shown_at = None;
                    } else {
                        // First Escape - show hint
                        inner.exit_hint = Some(ExitHint::Escape);
                        inner.exit_hint_shown_at = Some(now);
                    }
                }
                // Empty input: do nothing (no else branch)
            }

            // Backspace - Delete character before cursor, or exit shell mode if empty
            (_, KeyCode::Backspace) => {
                if inner.cursor_pos > 0 {
                    let pos = inner.cursor_pos - 1;
                    inner.cursor_pos = pos;
                    inner.input_buffer.remove(pos);
                } else if inner.shell_mode && inner.input_buffer.is_empty() {
                    // Backspace on empty input in shell mode: exit shell mode
                    inner.shell_mode = false;
                }
                // Update slash menu state
                update_slash_menu_inner(&mut inner);
            }

            // Delete - Delete character at cursor
            (_, KeyCode::Delete) => {
                let pos = inner.cursor_pos;
                if pos < inner.input_buffer.len() {
                    inner.input_buffer.remove(pos);
                }
                // Update slash menu state
                update_slash_menu_inner(&mut inner);
            }

            // Left arrow - Move cursor left
            (_, KeyCode::Left) => {
                if inner.cursor_pos > 0 {
                    inner.cursor_pos -= 1;
                }
            }

            // Right arrow - Move cursor right
            (_, KeyCode::Right) => {
                if inner.cursor_pos < inner.input_buffer.len() {
                    inner.cursor_pos += 1;
                }
            }

            // Up arrow - Previous history (only when slash menu is closed)
            (_, KeyCode::Up) => {
                navigate_history_inner(&mut inner, -1);
            }

            // Down arrow - Next history (only when slash menu is closed)
            (_, KeyCode::Down) => {
                navigate_history_inner(&mut inner, 1);
            }

            // Home - Move cursor to start
            (_, KeyCode::Home) => {
                inner.cursor_pos = 0;
            }

            // Ctrl+A - Move cursor to start
            (m, KeyCode::Char('a')) if m.contains(KeyModifiers::CONTROL) => {
                inner.cursor_pos = 0;
            }

            // End - Move cursor to end
            (_, KeyCode::End) => {
                inner.cursor_pos = inner.input_buffer.len();
            }

            // Ctrl+E - Move cursor to end
            (m, KeyCode::Char('e')) if m.contains(KeyModifiers::CONTROL) => {
                inner.cursor_pos = inner.input_buffer.len();
            }

            // Ctrl+U - Clear line before cursor
            (m, KeyCode::Char('u')) if m.contains(KeyModifiers::CONTROL) => {
                inner.input_buffer = inner.input_buffer[inner.cursor_pos..].to_string();
                inner.cursor_pos = 0;
                // Update slash menu state
                update_slash_menu_inner(&mut inner);
            }

            // Ctrl+K - Clear line after cursor
            (m, KeyCode::Char('k')) if m.contains(KeyModifiers::CONTROL) => {
                let pos = inner.cursor_pos;
                inner.input_buffer.truncate(pos);
                // Update slash menu state
                update_slash_menu_inner(&mut inner);
            }

            // Ctrl+W - Delete word before cursor
            (m, KeyCode::Char('w')) if m.contains(KeyModifiers::CONTROL) => {
                delete_word_before_cursor_inner(&mut inner);
                // Update slash menu state
                update_slash_menu_inner(&mut inner);
            }

            // Ctrl+_ - Undo last input segment
            // Note: Ctrl+_ is encoded as ASCII 0x1F, Char('_') with CONTROL, or Char('/') with CONTROL
            _ if ctrl_key!(underscore, key.modifiers, key.code) => {
                if let Some(previous) = inner.undo_stack.pop() {
                    inner.input_buffer = previous;
                    inner.cursor_pos = inner.cursor_pos.min(inner.input_buffer.len());
                }
            }

            // Ctrl+S - Stash/restore prompt
            // Note: Ctrl+S is encoded as ASCII 0x13 or Char('s') with CONTROL
            _ if ctrl_key!(s, key.modifiers, key.code) => {
                if let Some(stashed) = inner.stash_buffer.take() {
                    // Restore: stash exists, restore it to input
                    inner.input_buffer = stashed;
                    inner.cursor_pos = inner.input_buffer.len();
                    inner.show_stash_indicator = false;
                } else if !inner.input_buffer.is_empty() {
                    // Stash: input is not empty, save it
                    inner.stash_buffer = Some(std::mem::take(&mut inner.input_buffer));
                    inner.cursor_pos = 0;
                    inner.show_stash_indicator = true;
                }
                // If input is empty and no stash exists, do nothing
            }

            // '?' key - show shortcuts panel on empty input, otherwise type literal
            (m, KeyCode::Char('?')) if m.is_empty() || m == KeyModifiers::SHIFT => {
                if inner.input_buffer.is_empty() && !inner.show_shortcuts_panel {
                    // Empty input: show shortcuts panel
                    inner.show_shortcuts_panel = true;
                } else {
                    // Non-empty input or panel already showing: type literal '?'
                    let pos = inner.cursor_pos;
                    inner.input_buffer.insert(pos, '?');
                    inner.cursor_pos = pos + 1;
                    // Reset history browsing on new input
                    inner.history_index = None;
                    // Clear exit hint on typing
                    inner.exit_hint = None;
                    inner.exit_hint_shown_at = None;
                }
            }

            // '!' key - enter shell mode on empty input, otherwise type literal
            (m, KeyCode::Char('!')) if m.is_empty() || m == KeyModifiers::SHIFT => {
                if inner.input_buffer.is_empty() && !inner.shell_mode {
                    // Empty input: enter shell mode
                    inner.shell_mode = true;
                    // Clear any exit hint
                    inner.exit_hint = None;
                    inner.exit_hint_shown_at = None;
                } else {
                    // Already in shell mode or has input: type literal '!'
                    let pos = inner.cursor_pos;
                    inner.input_buffer.insert(pos, '!');
                    inner.cursor_pos = pos + 1;
                    // Reset history browsing on new input
                    inner.history_index = None;
                    // Clear exit hint on typing
                    inner.exit_hint = None;
                    inner.exit_hint_shown_at = None;
                }
            }

            // Regular character input
            (m, KeyCode::Char(c)) if m.is_empty() || m == KeyModifiers::SHIFT => {
                // Push snapshot at word boundaries (space typed or first character typed)
                let should_snapshot = c == ' ' || inner.input_buffer.is_empty();
                if should_snapshot {
                    push_undo_snapshot(&mut inner);
                }

                let pos = inner.cursor_pos;
                inner.input_buffer.insert(pos, c);
                inner.cursor_pos = pos + 1;
                // Reset history browsing on new input
                inner.history_index = None;
                // Clear exit hint on typing
                inner.exit_hint = None;
                inner.exit_hint_shown_at = None;
                // Update slash menu state
                update_slash_menu_inner(&mut inner);
            }

            _ => {}
        }
    }

    /// Handle key events in responding/thinking mode
    pub(super) fn handle_responding_key(&self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            // Ctrl+C - Interrupt current response
            (m, KeyCode::Char('c')) if m.contains(KeyModifiers::CONTROL) => {
                self.handle_interrupt();
            }

            // Escape - Also interrupt
            (_, KeyCode::Esc) => {
                self.handle_interrupt();
            }

            _ => {}
        }
    }

    /// Handle Ctrl+C interrupt
    pub(super) fn handle_interrupt(&self) {
        let mut inner = self.inner.lock();
        match inner.mode {
            AppMode::Input => {
                // Check if within exit hint timeout
                let now = inner.clock.now_millis();
                let within_timeout = inner.exit_hint == Some(ExitHint::CtrlC)
                    && inner
                        .exit_hint_shown_at
                        .map(|t| now.saturating_sub(t) < EXIT_HINT_TIMEOUT_MS)
                        .unwrap_or(false);

                if within_timeout {
                    // Second Ctrl+C within timeout - exit
                    inner.should_exit = true;
                    inner.exit_reason = Some(ExitReason::Interrupted);
                } else {
                    // First Ctrl+C - clear input (if any) and show hint
                    inner.input_buffer.clear();
                    inner.cursor_pos = 0;
                    inner.exit_hint = Some(ExitHint::CtrlC);
                    inner.exit_hint_shown_at = Some(now);
                }
            }
            AppMode::Responding | AppMode::Thinking => {
                // Cancel current response
                inner.is_streaming = false;
                inner.mode = AppMode::Input;
                inner.response_content.push_str("\n\n[Interrupted]");
            }
            AppMode::Permission => {
                // Deny and return to input
                if let Some(ref mut perm) = inner.pending_permission {
                    perm.dialog.selected = PermissionSelection::No;
                }
                drop(inner);
                self.confirm_permission();
            }
            AppMode::Trust => {
                // Exit on interrupt during trust prompt
                inner.should_exit = true;
                inner.exit_reason = Some(ExitReason::UserQuit);
            }
            AppMode::ThinkingToggle => {
                // Close dialog without changing
                inner.thinking_dialog = None;
                inner.mode = AppMode::Input;
            }
            AppMode::TasksDialog => {
                // Close dialog without action
                inner.tasks_dialog = None;
                inner.mode = AppMode::Input;
            }
            AppMode::ModelPicker => {
                // Close dialog without changing model
                inner.model_picker_dialog = None;
                inner.mode = AppMode::Input;
            }
            AppMode::ExportDialog => {
                // Close dialog with cancellation message
                inner.export_dialog = None;
                inner.mode = AppMode::Input;
                inner.response_content = "Export cancelled".to_string();
                inner.is_command_output = true;
            }
            AppMode::HelpDialog => {
                // Close dialog with dismissal message
                inner.help_dialog = None;
                inner.mode = AppMode::Input;
                inner.response_content = "Help dialog dismissed".to_string();
                inner.is_command_output = true;
            }
            AppMode::HooksDialog => {
                // Close dialog with dismissal message
                inner.hooks_dialog = None;
                inner.mode = AppMode::Input;
                inner.response_content = "Hooks dialog dismissed".to_string();
                inner.is_command_output = true;
            }
            AppMode::MemoryDialog => {
                // Close dialog with dismissal message
                inner.memory_dialog = None;
                inner.mode = AppMode::Input;
                inner.response_content = "Memory dialog dismissed".to_string();
                inner.is_command_output = true;
            }
        }
    }
}

/// Update slash menu state based on current input buffer
pub(super) fn update_slash_menu_inner(inner: &mut TuiAppStateInner) {
    if inner.input_buffer.starts_with('/') {
        let filter = inner.input_buffer[1..].to_string();
        if let Some(ref mut menu) = inner.slash_menu {
            menu.set_filter(filter);
        } else {
            let mut menu = SlashMenuState::new();
            menu.set_filter(filter);
            inner.slash_menu = Some(menu);
        }
    } else {
        inner.slash_menu = None;
    }
}

/// Navigate through command history
pub(super) fn navigate_history_inner(inner: &mut TuiAppStateInner, direction: i32) {
    if inner.history.is_empty() {
        return;
    }

    let new_index = match inner.history_index {
        None if direction < 0 => Some(inner.history.len() - 1),
        None => return,
        Some(i) if direction < 0 && i > 0 => Some(i - 1),
        Some(i) if direction > 0 && i < inner.history.len() - 1 => Some(i + 1),
        Some(_) if direction > 0 => {
            // Past end of history, clear input
            inner.history_index = None;
            inner.input_buffer.clear();
            inner.cursor_pos = 0;
            inner.undo_stack.clear();
            return;
        }
        Some(i) => Some(i),
    };

    if let Some(idx) = new_index {
        inner.history_index = Some(idx);
        inner.input_buffer = inner.history[idx].clone();
        inner.cursor_pos = inner.input_buffer.len();
        inner.undo_stack.clear();
    }
}

/// Delete word before cursor (Ctrl+W behavior)
pub(super) fn delete_word_before_cursor_inner(inner: &mut TuiAppStateInner) {
    if inner.cursor_pos == 0 {
        return;
    }

    let before = &inner.input_buffer[..inner.cursor_pos];
    let trimmed = before.trim_end();
    let word_start = trimmed
        .rfind(char::is_whitespace)
        .map(|i| i + 1)
        .unwrap_or(0);

    inner.input_buffer = format!(
        "{}{}",
        &inner.input_buffer[..word_start],
        &inner.input_buffer[inner.cursor_pos..]
    );
    inner.cursor_pos = word_start;
}

/// Push current input state to undo stack if appropriate
pub(super) fn push_undo_snapshot(inner: &mut TuiAppStateInner) {
    // Push if stack is empty or last snapshot differs from current
    if inner.undo_stack.last() != Some(&inner.input_buffer) {
        inner.undo_stack.push(inner.input_buffer.clone());
    }
}

/// Clear undo stack (e.g., when submitting input or navigating history)
pub(super) fn clear_undo_stack(inner: &mut TuiAppStateInner) {
    inner.undo_stack.clear();
}
