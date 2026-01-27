// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Dialog-specific key handlers for the TUI application.
//!
//! Contains key event handlers for various dialogs:
//! - Permission dialog
//! - Trust prompt
//! - Thinking toggle
//! - Tasks dialog
//! - Model picker
//! - Export dialog
//! - Help dialog
//! - Hooks dialog
//! - Memory dialog

use iocraft::prelude::*;

use crate::tui::slash_menu::COMMANDS;
use crate::tui::widgets::export::ExportStep;
use crate::tui::widgets::permission::PermissionSelection;
use crate::tui::widgets::thinking::ThinkingMode;
use crate::tui::widgets::trust::TrustChoice;
use crate::tui::widgets::HooksView;

use super::commands::{do_clipboard_export, do_file_export};
use super::state::TuiAppState;
use super::types::{AppMode, ExitReason};

impl TuiAppState {
    /// Handle key events in permission mode
    pub(super) fn handle_permission_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();
        match key.code {
            // Up - Move selection up
            KeyCode::Up => {
                if let Some(ref mut perm) = inner.pending_permission {
                    perm.dialog.selected = perm.dialog.selected.prev();
                }
            }

            // Down - Move selection down
            KeyCode::Down => {
                if let Some(ref mut perm) = inner.pending_permission {
                    perm.dialog.selected = perm.dialog.selected.next();
                }
            }

            // Enter - Confirm selection
            KeyCode::Enter => {
                drop(inner);
                self.confirm_permission();
            }

            // 1 - Select Yes and confirm
            KeyCode::Char('1') => {
                if let Some(ref mut perm) = inner.pending_permission {
                    perm.dialog.selected = PermissionSelection::Yes;
                }
                drop(inner);
                self.confirm_permission();
            }

            // Y/y - Select Yes and confirm
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(ref mut perm) = inner.pending_permission {
                    perm.dialog.selected = PermissionSelection::Yes;
                }
                drop(inner);
                self.confirm_permission();
            }

            // 2 - Select Yes for session and confirm
            KeyCode::Char('2') => {
                if let Some(ref mut perm) = inner.pending_permission {
                    perm.dialog.selected = PermissionSelection::YesSession;
                }
                drop(inner);
                self.confirm_permission();
            }

            // 3 - Select No and confirm
            KeyCode::Char('3') => {
                if let Some(ref mut perm) = inner.pending_permission {
                    perm.dialog.selected = PermissionSelection::No;
                }
                drop(inner);
                self.confirm_permission();
            }

            // N/n - Select No and confirm
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if let Some(ref mut perm) = inner.pending_permission {
                    perm.dialog.selected = PermissionSelection::No;
                }
                drop(inner);
                self.confirm_permission();
            }

            // Escape - Cancel (select No)
            KeyCode::Esc => {
                if let Some(ref mut perm) = inner.pending_permission {
                    perm.dialog.selected = PermissionSelection::No;
                }
                drop(inner);
                self.confirm_permission();
            }

            _ => {}
        }
    }

    /// Handle key events in trust prompt mode
    pub(super) fn handle_trust_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();
        match key.code {
            // Left/Right/Tab - Toggle selection
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                if let Some(ref mut prompt) = inner.trust_prompt {
                    prompt.selected = match prompt.selected {
                        TrustChoice::Yes => TrustChoice::No,
                        TrustChoice::No => TrustChoice::Yes,
                    };
                }
            }

            // Enter - Confirm selection
            KeyCode::Enter => {
                if let Some(ref prompt) = inner.trust_prompt {
                    match prompt.selected {
                        TrustChoice::Yes => {
                            inner.trust_granted = true;
                            inner.trust_prompt = None;
                            inner.mode = AppMode::Input;
                        }
                        TrustChoice::No => {
                            inner.should_exit = true;
                            inner.exit_reason = Some(ExitReason::UserQuit);
                        }
                    }
                }
            }

            // Y/y - Yes (trust)
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                inner.trust_granted = true;
                inner.trust_prompt = None;
                inner.mode = AppMode::Input;
            }

            // N/n or Escape - No (exit)
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                inner.should_exit = true;
                inner.exit_reason = Some(ExitReason::UserQuit);
            }

            _ => {}
        }
    }

    /// Handle key events in thinking toggle mode
    pub(super) fn handle_thinking_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();
        match key.code {
            // Up/Down arrows, Tab - Toggle selection
            KeyCode::Up | KeyCode::Down | KeyCode::Tab => {
                if let Some(ref mut dialog) = inner.thinking_dialog {
                    dialog.selected = match dialog.selected {
                        ThinkingMode::Enabled => ThinkingMode::Disabled,
                        ThinkingMode::Disabled => ThinkingMode::Enabled,
                    };
                }
            }

            // Enter - Confirm selection
            KeyCode::Enter => {
                if let Some(ref dialog) = inner.thinking_dialog {
                    inner.thinking_enabled = dialog.selected == ThinkingMode::Enabled;
                }
                inner.thinking_dialog = None;
                inner.mode = AppMode::Input;
            }

            // Escape - Cancel (close without changing)
            KeyCode::Esc => {
                inner.thinking_dialog = None;
                inner.mode = AppMode::Input;
            }

            _ => {}
        }
    }

    /// Handle key events in tasks dialog mode
    pub(super) fn handle_tasks_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();
        match key.code {
            KeyCode::Esc => {
                // Close dialog with dismissal message
                inner.mode = AppMode::Input;
                inner.tasks_dialog = None;
                inner.response_content = "Background tasks dialog dismissed".to_string();
                inner.is_command_output = true;
            }
            KeyCode::Up => {
                if let Some(ref mut dialog) = inner.tasks_dialog {
                    dialog.move_selection_up();
                }
            }
            KeyCode::Down => {
                if let Some(ref mut dialog) = inner.tasks_dialog {
                    dialog.move_selection_down();
                }
            }
            KeyCode::Enter => {
                // Future: view selected task details
                // For now, just close the dialog
                inner.mode = AppMode::Input;
                inner.tasks_dialog = None;
            }
            _ => {}
        }
    }

    /// Handle key events in model picker mode
    pub(super) fn handle_model_picker_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut dialog) = inner.model_picker_dialog {
                    dialog.move_up();
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                if let Some(ref mut dialog) = inner.model_picker_dialog {
                    dialog.move_down();
                }
            }
            KeyCode::Enter => {
                if let Some(ref dialog) = inner.model_picker_dialog {
                    // Apply selection
                    inner.status.model = dialog.selected.model_id().to_string();
                }
                inner.model_picker_dialog = None;
                inner.mode = AppMode::Input;
            }
            KeyCode::Esc => {
                // Cancel without changes
                inner.model_picker_dialog = None;
                inner.mode = AppMode::Input;
            }
            _ => {}
        }
    }

    /// Handle key events in export dialog mode
    pub(super) fn handle_export_dialog_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        let Some(ref mut dialog) = inner.export_dialog else {
            return;
        };

        match dialog.step {
            ExportStep::MethodSelection => match key.code {
                KeyCode::Esc => {
                    inner.mode = AppMode::Input;
                    inner.export_dialog = None;
                    inner.response_content = "Export cancelled".to_string();
                    inner.is_command_output = true;
                }
                KeyCode::Up => dialog.move_selection_up(),
                KeyCode::Down => dialog.move_selection_down(),
                KeyCode::Enter => {
                    if dialog.confirm_selection() {
                        // Clipboard export
                        do_clipboard_export(&mut inner);
                    }
                    // else: moved to filename input, dialog updated
                }
                _ => {}
            },
            ExportStep::FilenameInput => match key.code {
                KeyCode::Esc => {
                    dialog.go_back();
                }
                KeyCode::Enter => {
                    do_file_export(&mut inner);
                }
                KeyCode::Backspace => dialog.pop_char(),
                KeyCode::Char(c) => dialog.push_char(c),
                _ => {}
            },
        }
    }

    /// Handle key events in help dialog mode
    pub(super) fn handle_help_dialog_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        let Some(ref mut dialog) = inner.help_dialog else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                inner.mode = AppMode::Input;
                inner.help_dialog = None;
                inner.response_content = "Help dialog dismissed".to_string();
                inner.is_command_output = true;
            }
            KeyCode::Tab | KeyCode::Right => dialog.next_tab(),
            KeyCode::Left | KeyCode::BackTab => dialog.prev_tab(),
            KeyCode::Up => dialog.select_prev(COMMANDS.len()),
            KeyCode::Down => dialog.select_next(COMMANDS.len()),
            _ => {}
        }
    }

    /// Handle key events in hooks dialog mode
    pub(super) fn handle_hooks_dialog_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        let Some(ref mut dialog) = inner.hooks_dialog else {
            return;
        };

        match dialog.view {
            HooksView::HookList => match key.code {
                KeyCode::Esc => {
                    inner.mode = AppMode::Input;
                    inner.hooks_dialog = None;
                    inner.response_content = "Hooks dialog dismissed".to_string();
                    inner.is_command_output = true;
                }
                KeyCode::Up => dialog.select_prev(),
                KeyCode::Down => dialog.select_next(),
                KeyCode::Enter => dialog.open_matchers(),
                _ => {}
            },
            HooksView::Matchers => match key.code {
                KeyCode::Esc => dialog.close_matchers(),
                KeyCode::Up => {
                    // Navigate matchers (when implemented)
                }
                KeyCode::Down => {
                    // Navigate matchers (when implemented)
                }
                KeyCode::Enter => {
                    // Add new matcher (when implemented)
                }
                _ => {}
            },
        }
    }

    /// Handle key events in memory dialog mode
    pub(super) fn handle_memory_dialog_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        let Some(ref mut dialog) = inner.memory_dialog else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                inner.mode = AppMode::Input;
                inner.memory_dialog = None;
                inner.response_content = "Memory dialog dismissed".to_string();
                inner.is_command_output = true;
            }
            KeyCode::Up => dialog.select_prev(),
            KeyCode::Down => dialog.select_next(),
            KeyCode::Enter => {
                // Open selected memory file for viewing/editing
                // For now, just show the path of the selected entry
                if let Some(entry) = dialog.selected_entry() {
                    let path = entry.path.as_deref().unwrap_or("(not configured)");
                    inner.response_content =
                        format!("Selected: {} - {}", entry.source.name(), path);
                    inner.is_command_output = true;
                    inner.memory_dialog = None;
                    inner.mode = AppMode::Input;
                }
            }
            _ => {}
        }
    }
}
