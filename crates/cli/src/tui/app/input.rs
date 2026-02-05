// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Input mode key handling for the TUI application.

use iocraft::prelude::*;

use crate::time::Clock;
use crate::tui::widgets::permission::PermissionSelection;
use crate::tui::widgets::thinking::ThinkingDialog;

use super::state::{DialogState, TuiAppState};
use super::types::{AppMode, ExitHint, ExitReason};

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
            AppMode::Setup => self.handle_setup_key(key),
            AppMode::Trust => self.handle_trust_key(key),
            AppMode::BypassConfirm => self.handle_bypass_confirm_key(key),
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
            AppMode::Elicitation => self.handle_elicitation_key(key),
            AppMode::PlanApproval => self.handle_plan_approval_key(key),
        }
    }

    /// Handle key events in input mode
    pub(super) fn handle_input_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        // Handle slash menu navigation when menu is open
        if inner.display.slash_menu.is_some() {
            match key.code {
                KeyCode::Down => {
                    if let Some(ref mut menu) = inner.display.slash_menu {
                        menu.select_next();
                    }
                    return;
                }
                KeyCode::Up => {
                    if let Some(ref mut menu) = inner.display.slash_menu {
                        menu.select_prev();
                    }
                    return;
                }
                KeyCode::Tab => {
                    // Complete the selected command
                    if let Some(ref menu) = inner.display.slash_menu {
                        if let Some(cmd) = menu.selected_command() {
                            inner.input.buffer = format!("{} ", cmd.full_name());
                            inner.input.cursor_pos = inner.input.buffer.len();
                        }
                    }
                    inner.display.slash_menu = None; // Close menu
                    return;
                }
                KeyCode::Esc => {
                    // Close menu but keep text, show "Esc to clear again" hint
                    inner.display.slash_menu = None;
                    let now = inner.clock.now_millis();
                    inner.display.show_exit_hint(ExitHint::Escape, now);
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
                if inner.input.buffer.is_empty() {
                    let now = inner.clock.now_millis();
                    let exit_hint_timeout = inner.config.timeouts.exit_hint_ms;
                    let within_timeout = inner.display.exit_hint == Some(ExitHint::CtrlD)
                        && inner
                            .display
                            .exit_hint_shown_at
                            .map(|t| now.saturating_sub(t) < exit_hint_timeout)
                            .unwrap_or(false);

                    if within_timeout {
                        // Second Ctrl+D within timeout - exit
                        inner.should_exit = true;
                        inner.exit_reason = Some(ExitReason::UserQuit);
                    } else {
                        // First Ctrl+D - show hint
                        inner.display.show_exit_hint(ExitHint::CtrlD, now);
                    }
                }
                // With text in input: ignored (do nothing)
            }

            // Ctrl+L - Clear screen (keep input)
            (m, KeyCode::Char('l')) if m.contains(KeyModifiers::CONTROL) => {
                inner.display.response_content.clear();
            }

            // Meta+t (Alt+t) - Toggle thinking mode
            // Also matches Escape followed by 't' within 100ms (PTY escape sequence)
            (m, KeyCode::Char('t'))
                if m.contains(KeyModifiers::META)
                    || m.contains(KeyModifiers::ALT)
                    || inner.display.is_escape_sequence(inner.clock.now_millis()) =>
            {
                inner.display.clear_escape_sequence();
                inner.dialog = DialogState::Thinking(ThinkingDialog::new(inner.thinking_enabled));
                inner.mode = AppMode::ThinkingToggle;
            }

            // Meta+p (Alt+p) - Open model picker
            // Also matches Escape followed by 'p' within 100ms (PTY escape sequence)
            (m, KeyCode::Char('p'))
                if m.contains(KeyModifiers::META)
                    || m.contains(KeyModifiers::ALT)
                    || inner.display.is_escape_sequence(inner.clock.now_millis()) =>
            {
                inner.display.clear_escape_sequence();
                inner.dialog = DialogState::ModelPicker(
                    crate::tui::widgets::ModelPickerDialog::new(&inner.status.model),
                );
                inner.mode = AppMode::ModelPicker;
            }

            // Ctrl+T - Show todos (only when todos exist)
            (m, KeyCode::Char('t')) if m.contains(KeyModifiers::CONTROL) => {
                if !inner.todos.is_empty() {
                    inner.display.response_content = Self::format_todos(&inner.todos);
                    inner.display.is_command_output = true;
                    inner.display.conversation_display = "Todo List".to_string();
                }
                // When no todos, do nothing (no visible change)
            }

            // Shift+Tab - Cycle permission mode
            (_, KeyCode::BackTab) => {
                inner.permission_mode = inner
                    .permission_mode
                    .cycle_next(inner.allow_bypass_permissions);
            }

            // Enter - Submit input
            (_, KeyCode::Enter) => {
                // Close slash menu on enter
                inner.display.slash_menu = None;
                // Clear exit hint on Enter
                inner.display.clear_exit_hint();
                if !inner.input.buffer.is_empty() {
                    drop(inner);
                    self.submit_input();
                }
            }

            // Escape - Dismiss shortcuts panel first, then exit shell mode, then check for clear
            // Note: slash menu escape is handled above in the slash_menu.is_some() block
            (_, KeyCode::Esc) => {
                // Always record escape timestamp for Esc+letter sequence detection
                // (PTY environments send Alt/Meta keys as Escape followed by the letter)
                let now = inner.clock.now_millis();
                inner.display.escape_pressed_at = Some(now);

                if inner.display.show_shortcuts_panel {
                    // First priority: dismiss shortcuts panel
                    inner.display.show_shortcuts_panel = false;
                } else if inner.input.shell_mode {
                    // Second priority: exit shell mode
                    inner.input.shell_mode = false;
                    inner.input.clear();
                } else if !inner.input.buffer.is_empty() {
                    // Input has text - check for double-tap
                    let exit_hint_timeout = inner.config.timeouts.exit_hint_ms;
                    let within_timeout = inner.display.exit_hint == Some(ExitHint::Escape)
                        && inner
                            .display
                            .exit_hint_shown_at
                            .map(|t| now.saturating_sub(t) < exit_hint_timeout)
                            .unwrap_or(false);

                    if within_timeout {
                        // Second Escape within timeout - clear input
                        inner.input.clear();
                        inner.display.clear_exit_hint();
                    } else {
                        // First Escape - show hint
                        inner.display.show_exit_hint(ExitHint::Escape, now);
                    }
                }
                // Empty input: do nothing (no else branch)
            }

            // Backspace - Delete character before cursor, or exit shell mode if empty
            (_, KeyCode::Backspace) => {
                if inner.input.cursor_pos > 0 {
                    inner.input.backspace();
                } else if inner.input.shell_mode && inner.input.buffer.is_empty() {
                    // Backspace on empty input in shell mode: exit shell mode
                    inner.input.shell_mode = false;
                }
                // Update slash menu state
                {
                    let buffer = inner.input.buffer.clone();
                    inner.display.update_slash_menu(&buffer);
                }
            }

            // Delete - Delete character at cursor
            (_, KeyCode::Delete) => {
                inner.input.delete();
                // Update slash menu state
                {
                    let buffer = inner.input.buffer.clone();
                    inner.display.update_slash_menu(&buffer);
                }
            }

            // Left arrow - Move cursor left
            (_, KeyCode::Left) => {
                inner.input.move_left();
            }

            // Right arrow - Move cursor right
            (_, KeyCode::Right) => {
                inner.input.move_right();
            }

            // Up arrow - Previous history (only when slash menu is closed)
            (_, KeyCode::Up) => {
                inner.input.navigate_history(-1);
            }

            // Down arrow - Next history (only when slash menu is closed)
            (_, KeyCode::Down) => {
                inner.input.navigate_history(1);
            }

            // Home - Move cursor to start
            (_, KeyCode::Home) => {
                inner.input.move_to_start();
            }

            // Ctrl+A - Move cursor to start
            (m, KeyCode::Char('a')) if m.contains(KeyModifiers::CONTROL) => {
                inner.input.move_to_start();
            }

            // End - Move cursor to end
            (_, KeyCode::End) => {
                inner.input.move_to_end();
            }

            // Ctrl+E - Move cursor to end
            (m, KeyCode::Char('e')) if m.contains(KeyModifiers::CONTROL) => {
                inner.input.move_to_end();
            }

            // Ctrl+U - Clear line before cursor
            (m, KeyCode::Char('u')) if m.contains(KeyModifiers::CONTROL) => {
                inner.input.clear_before_cursor();
                // Update slash menu state
                {
                    let buffer = inner.input.buffer.clone();
                    inner.display.update_slash_menu(&buffer);
                }
            }

            // Ctrl+K - Clear line after cursor
            (m, KeyCode::Char('k')) if m.contains(KeyModifiers::CONTROL) => {
                inner.input.clear_after_cursor();
                // Update slash menu state
                {
                    let buffer = inner.input.buffer.clone();
                    inner.display.update_slash_menu(&buffer);
                }
            }

            // Ctrl+W - Delete word before cursor
            (m, KeyCode::Char('w')) if m.contains(KeyModifiers::CONTROL) => {
                inner.input.delete_word_before_cursor();
                // Update slash menu state
                {
                    let buffer = inner.input.buffer.clone();
                    inner.display.update_slash_menu(&buffer);
                }
            }

            // Ctrl+_ - Undo last input segment
            // Note: Ctrl+_ is encoded as ASCII 0x1F, Char('_') with CONTROL, or Char('/') with CONTROL
            _ if ctrl_key!(underscore, key.modifiers, key.code) => {
                inner.input.undo();
            }

            // Ctrl+S - Stash/restore prompt
            // Note: Ctrl+S is encoded as ASCII 0x13 or Char('s') with CONTROL
            _ if ctrl_key!(s, key.modifiers, key.code) => {
                if inner.input.stash.is_some() {
                    // Restore: stash exists, restore it to input
                    inner.input.restore_stash();
                } else if !inner.input.buffer.is_empty() {
                    // Stash: input is not empty, save it
                    inner.input.stash();
                }
                // If input is empty and no stash exists, do nothing
            }

            // '?' key - show shortcuts panel on empty input, otherwise type literal
            (m, KeyCode::Char('?')) if m.is_empty() || m == KeyModifiers::SHIFT => {
                if inner.input.buffer.is_empty() && !inner.display.show_shortcuts_panel {
                    // Empty input: show shortcuts panel
                    inner.display.show_shortcuts_panel = true;
                } else {
                    // Non-empty input or panel already showing: type literal '?'
                    inner.input.insert_char('?');
                    // Reset history browsing on new input
                    inner.input.history_index = None;
                    // Clear exit hint on typing
                    inner.display.clear_exit_hint();
                }
            }

            // '!' key - enter shell mode on empty input, otherwise type literal
            (m, KeyCode::Char('!')) if m.is_empty() || m == KeyModifiers::SHIFT => {
                if inner.input.buffer.is_empty() && !inner.input.shell_mode {
                    // Empty input: enter shell mode
                    inner.input.shell_mode = true;
                    // Clear any exit hint
                    inner.display.clear_exit_hint();
                } else {
                    // Already in shell mode or has input: type literal '!'
                    inner.input.insert_char('!');
                    // Reset history browsing on new input
                    inner.input.history_index = None;
                    // Clear exit hint on typing
                    inner.display.clear_exit_hint();
                }
            }

            // Regular character input
            (m, KeyCode::Char(c)) if m.is_empty() || m == KeyModifiers::SHIFT => {
                // Push snapshot at word boundaries (space typed or first character typed)
                let should_snapshot = c == ' ' || inner.input.buffer.is_empty();
                if should_snapshot {
                    inner.input.push_undo_snapshot();
                }

                inner.input.insert_char(c);
                // Reset history browsing on new input
                inner.input.history_index = None;
                // Clear exit hint on typing
                inner.display.clear_exit_hint();
                // Update slash menu state
                {
                    let buffer = inner.input.buffer.clone();
                    inner.display.update_slash_menu(&buffer);
                }
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
                let exit_hint_timeout = inner.config.timeouts.exit_hint_ms;
                let within_timeout = inner.display.exit_hint == Some(ExitHint::CtrlC)
                    && inner
                        .display
                        .exit_hint_shown_at
                        .map(|t| now.saturating_sub(t) < exit_hint_timeout)
                        .unwrap_or(false);

                if within_timeout {
                    // Second Ctrl+C within timeout - exit
                    inner.should_exit = true;
                    inner.exit_reason = Some(ExitReason::Interrupted);
                } else {
                    // First Ctrl+C - clear input (if any) and show hint
                    inner.input.clear();
                    inner.display.show_exit_hint(ExitHint::CtrlC, now);
                }
            }
            AppMode::Responding | AppMode::Thinking => {
                // Cancel current response
                inner.display.is_streaming = false;
                inner.mode = AppMode::Input;
                inner.display.response_content.push_str("\n\n[Interrupted]");
            }
            AppMode::Permission => {
                // Deny and return to input
                if let Some(perm) = inner.dialog.as_permission_mut() {
                    perm.dialog.selected = PermissionSelection::No;
                }
                drop(inner);
                self.confirm_permission();
            }
            AppMode::Setup => {
                // Exit on interrupt during setup wizard
                inner.should_exit = true;
                inner.exit_reason = Some(ExitReason::UserQuit);
            }
            AppMode::Trust | AppMode::BypassConfirm => {
                // Exit on interrupt during trust/bypass prompt
                inner.should_exit = true;
                inner.exit_reason = Some(ExitReason::UserQuit);
            }
            AppMode::ThinkingToggle => {
                // Close dialog without changing
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
            }
            AppMode::TasksDialog => {
                // Close dialog without action
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
            }
            AppMode::ModelPicker => {
                // Close dialog without changing model
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
            }
            AppMode::ExportDialog => {
                // Close dialog with cancellation message
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
                inner.display.response_content = "Export cancelled".to_string();
                inner.display.is_command_output = true;
            }
            AppMode::HelpDialog => {
                // Close dialog with dismissal message
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
                inner.display.response_content = "Help dialog dismissed".to_string();
                inner.display.is_command_output = true;
            }
            AppMode::HooksDialog => {
                // Close dialog with dismissal message
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
                inner.display.response_content = "Hooks dialog dismissed".to_string();
                inner.display.is_command_output = true;
            }
            AppMode::MemoryDialog => {
                // Close dialog with dismissal message
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
                inner.display.response_content = "Memory dialog dismissed".to_string();
                inner.display.is_command_output = true;
            }
            AppMode::Elicitation => {
                // Cancel elicitation and return to input
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
                inner.display.response_content = "User declined to answer questions".to_string();
            }
            AppMode::PlanApproval => {
                // Cancel plan approval and return to input
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
                inner.display.response_content = "User rejected tool use".to_string();
            }
        }
    }
}
