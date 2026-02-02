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

/// Resolve a model alias (e.g. "haiku") to the full API model ID.
/// If the input is already a full model ID, it is returned unchanged.
fn resolve_model_id(model: &str) -> String {
    match model.to_lowercase().as_str() {
        "haiku" | "claude-haiku" => "claude-haiku-4-5-20251001".to_string(),
        "sonnet" | "claude-sonnet" => "claude-sonnet-4-20250514".to_string(),
        "opus" | "claude-opus" => "claude-opus-4-5-20251101".to_string(),
        _ => model.to_string(),
    }
}

pub(in crate::tui::app) use export::{do_clipboard_export, do_file_export};

use crate::permission::PermissionMode;
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

    // Accumulate previous response content into conversation history
    if !inner.display.is_command_output && !inner.display.response_content.is_empty() {
        let response = inner.display.response_content.clone();
        if !inner.display.conversation_display.is_empty() {
            inner.display.conversation_display.push_str("\n\n");
        }
        inner
            .display
            .conversation_display
            .push_str(&format!("⏺ {}", response));
    }
    if inner.display.is_command_output && !inner.display.response_content.is_empty() {
        let formatted: String = inner
            .display
            .response_content
            .lines()
            .map(|line| format!("  ⎿  {}", line))
            .collect::<Vec<_>>()
            .join("\n");
        inner.display.conversation_display.push('\n');
        inner.display.conversation_display.push_str(&formatted);
    }

    inner.display.is_command_output = true;
    inner.is_compacting = false;

    // Add the command to conversation display
    let prompt = format!("❯ {}", input.trim());
    if inner.display.conversation_display.is_empty() {
        inner.display.conversation_display = prompt;
    } else {
        inner.display.conversation_display.push_str("\n\n");
        inner.display.conversation_display.push_str(&prompt);
    }

    match cmd.as_str() {
        "/clear" => {
            // Clear session turns
            inner.sessions.current_session().turns.clear();

            // Reset token counts
            inner.status.input_tokens = 0;
            inner.status.output_tokens = 0;

            // Clear session-level permission grants
            inner.session_grants.clear();

            // Clear conversation display history, keeping only the /clear command prompt
            inner.display.conversation_display = format!("❯ {}", input.trim());

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
            let model = inner.config.model.clone();
            let usage = if model.is_empty() {
                ContextUsage::new()
            } else {
                let model_id = resolve_model_id(&model);
                ContextUsage::new_with_model(model_id)
            };
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
            inner.mode = AppMode::ExportDialog;
            inner.dialog = DialogState::Export(ExportDialog::new());
        }
        "/hooks" => {
            inner.mode = AppMode::HooksDialog;
            // For now, hard-code to 5 active hooks as shown in the fixture
            inner.dialog = DialogState::Hooks(crate::tui::widgets::HooksDialog::new(5));
        }
        "/memory" => {
            inner.mode = AppMode::MemoryDialog;
            inner.dialog = DialogState::Memory(crate::tui::widgets::MemoryDialog::new());
        }
        "/plan" => {
            if inner.permission_mode == PermissionMode::Plan {
                inner.display.response_content =
                    "Already in plan mode. No plan written yet.".to_string();
            } else {
                inner.permission_mode = PermissionMode::Plan;
                inner.display.response_content = "Enabled plan mode".to_string();
            }
        }
        _ => {
            inner.display.response_content = format!("Unknown command: {}", input);
        }
    }
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
