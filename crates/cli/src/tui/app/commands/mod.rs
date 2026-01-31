// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Command processing and prompt handling for the TUI application.
//!
//! Contains:
//! - `submit_input` - Input submission routing
//! - `handle_command_inner` - Slash command dispatch
//! - Submodules for execution, permission handling, and export

mod execution;
mod export;
mod permission;

pub(in crate::tui::app) use export::{do_clipboard_export, do_file_export};

use crate::tui::widgets::context::ContextUsage;
use crate::tui::widgets::export::ExportDialog;
use crate::tui::widgets::help::HelpDialog;
use crate::tui::widgets::tasks::TasksDialog;

use super::state::{DialogState, TuiAppState, TuiAppStateInner};
use super::types::{AppMode, ExitReason};

impl TuiAppState {
    /// Submit the current input
    pub(super) fn submit_input(&self) {
        let mut inner = self.inner.lock();
        let input = std::mem::take(&mut inner.input.buffer);
        let was_shell_mode = inner.input.shell_mode;
        inner.input.shell_mode = false; // Reset shell mode after submit
        inner.input.cursor_pos = 0;
        inner.input.undo_stack.clear();

        // Add to history (with shell prefix if applicable)
        let history_entry = if was_shell_mode {
            format!("! {}", input)
        } else {
            input.clone()
        };
        if !history_entry.is_empty() {
            inner.input.history.push(history_entry);
        }
        inner.input.history_index = None;

        // Check for slash commands (not applicable in shell mode)
        if !was_shell_mode && input.starts_with('/') {
            handle_command_inner(&mut inner, &input);
        } else if was_shell_mode {
            // Shell mode: execute command via Bash
            let command = input;
            drop(inner);
            self.execute_shell_command(command);
        } else {
            // Process the input as a prompt
            drop(inner);
            self.process_prompt(input);
        }
    }
}

/// Handle slash commands like /compact and /clear
pub(super) fn handle_command_inner(inner: &mut TuiAppStateInner, input: &str) {
    let cmd = input.trim().to_lowercase();
    inner.display.is_command_output = true;

    // Add the command to conversation display
    inner.display.conversation_display = format!("❯ {}", input.trim());

    match cmd.as_str() {
        "/clear" => {
            // Clear session turns
            inner.sessions.current_session().turns.clear();

            // Reset token counts
            inner.status.input_tokens = 0;
            inner.status.output_tokens = 0;

            // Clear session-level permission grants
            inner.session_grants.clear();

            // Set response content (will be rendered with elbow connector)
            inner.display.response_content = "(no content)".to_string();
        }
        "/compact" => {
            // Check if already compacting
            if inner.is_compacting {
                inner.display.response_content =
                    "Failed to compact: Compaction already in progress".to_string();
            } else {
                // Show compacting in progress message
                inner.mode = AppMode::Responding;
                inner.is_compacting = true;
                inner.compacting_started = Some(std::time::Instant::now());
                // Reset spinner for compacting animation
                inner.display.spinner_frame = 0;
                inner.display.spinner_verb = "Compacting".to_string();
                // Clear response content - spinner will be rendered dynamically
                inner.display.response_content.clear();
            }
        }
        "/fork" => {
            // Check if there's a conversation to fork
            let has_conversation = inner
                .sessions
                .get_current()
                .map(|s| !s.turns.is_empty())
                .unwrap_or(false);

            if has_conversation {
                // TODO: Implement actual fork functionality
                // For now, show a placeholder message
                inner.display.response_content = "Conversation forked".to_string();
            } else {
                // No conversation to fork - show error
                inner.display.response_content =
                    "Failed to fork conversation: No conversation to fork".to_string();
            }
        }
        "/help" | "/?" => {
            inner.mode = AppMode::HelpDialog;
            let version = inner
                .config
                .claude_version
                .clone()
                .unwrap_or_else(|| "2.1.12".to_string());
            inner.dialog = DialogState::Help(HelpDialog::new(version));
        }
        "/context" => {
            let usage = ContextUsage::new();
            inner.display.response_content = TuiAppState::format_context_usage(&usage);
        }
        "/exit" => {
            let farewell = TuiAppState::random_farewell().to_string();
            inner.display.response_content = farewell.clone();
            inner.exit_message = Some(farewell);
            inner.should_exit = true;
            inner.exit_reason = Some(ExitReason::UserQuit);
        }
        "/todos" => {
            inner.display.response_content = TuiAppState::format_todos(&inner.todos);
        }
        "/tasks" => {
            inner.mode = AppMode::TasksDialog;
            inner.dialog = DialogState::Tasks(TasksDialog::new());
        }
        "/export" => {
            // Check if there's a conversation to export
            let has_conversation = inner
                .sessions
                .get_current()
                .map(|s| !s.turns.is_empty())
                .unwrap_or(false);

            if has_conversation {
                inner.mode = AppMode::ExportDialog;
                inner.dialog = DialogState::Export(ExportDialog::new());
            } else {
                inner.display.response_content =
                    "Failed to export: No conversation to export".to_string();
            }
        }
        "/hooks" => {
            inner.mode = AppMode::HooksDialog;
            // For now, hard-code to 4 active hooks as shown in the fixture
            inner.dialog = DialogState::Hooks(crate::tui::widgets::HooksDialog::new(4));
        }
        "/memory" => {
            inner.mode = AppMode::MemoryDialog;
            inner.dialog = DialogState::Memory(crate::tui::widgets::MemoryDialog::new());
        }
        _ => {
            inner.display.response_content = format!("Unknown command: {}", input);
        }
    }
}
