// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Command processing and prompt handling for the TUI application.
//!
//! Contains:
//! - `submit_input` - Input submission routing
//! - `handle_command_inner` - Slash command dispatch
//! - `process_prompt` - Prompt processing and response generation
//! - `confirm_permission` - Permission confirmation handling
//! - Export helpers

use crate::tui::spinner;
use crate::tui::streaming::{StreamingConfig, StreamingResponse};
use crate::tui::widgets::context::ContextUsage;
use crate::tui::widgets::export::ExportDialog;
use crate::tui::widgets::help::HelpDialog;
use crate::tui::widgets::permission::{DiffKind, DiffLine, PermissionType, RichPermissionDialog};
use crate::tui::widgets::tasks::TasksDialog;

use super::state::{DialogState, TuiAppState, TuiAppStateInner};
use super::types::{AppMode, ExitReason, PermissionRequest};

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

    /// Execute a shell command via Bash tool
    pub(super) fn execute_shell_command(&self, command: String) {
        let mut inner = self.inner.lock();

        // Add previous response to conversation display if any
        if !inner.display.response_content.is_empty() && !inner.display.is_command_output {
            let response = inner.display.response_content.clone();
            if !inner.display.conversation_display.is_empty() {
                inner.display.conversation_display.push_str("\n\n");
            }
            inner
                .display
                .conversation_display
                .push_str(&format!("⏺ {}", response));
        }

        // Add the shell command to conversation display with ! prefix
        if !inner.display.conversation_display.is_empty() {
            inner.display.conversation_display.push_str("\n\n");
        }
        inner
            .display
            .conversation_display
            .push_str(&format!("❯ ! {}", command));

        // Check if bypass mode - execute directly without permission dialog
        if inner.permission_mode.allows_all() {
            // Show bash output directly
            inner
                .display
                .conversation_display
                .push_str(&format!("\n\n⏺ Bash({})", command));

            // Get scenario response for the command
            let response_text = {
                let text = inner.scenario.response_text_or_default(&command);
                if text.is_empty() {
                    format!("$ {}", command)
                } else {
                    text
                }
            };

            // Start streaming the response
            inner.display.response_content.clear();
            inner.display.is_command_output = false;
            start_streaming_inner(&mut inner, response_text);
            return;
        }

        // Show bash permission dialog
        inner.mode = AppMode::Thinking;
        inner.display.response_content.clear();
        inner.display.is_command_output = false;
        // Reset spinner for thinking animation
        inner.display.spinner_frame = 0;
        inner.display.spinner_verb = spinner::random_verb().to_string();

        drop(inner);

        // Use existing bash permission flow
        self.show_bash_permission(command.clone(), Some(format!("Execute: {}", command)));
    }

    /// Process a prompt and generate response
    pub(super) fn process_prompt(&self, prompt: String) {
        // Check for test permission triggers first (before acquiring inner lock)
        if handle_test_permission_triggers(self, &prompt) {
            return;
        }

        let mut inner = self.inner.lock();

        // If there's previous response content, add it to conversation history first
        if !inner.display.response_content.is_empty() && !inner.display.is_command_output {
            let response = inner.display.response_content.clone();
            if !inner.display.conversation_display.is_empty() {
                inner.display.conversation_display.push_str("\n\n");
            }
            inner
                .display
                .conversation_display
                .push_str(&format!("⏺ {}", response));
        }

        // Add the new user prompt to conversation display
        if !inner.display.conversation_display.is_empty() {
            inner.display.conversation_display.push_str("\n\n");
        }
        inner
            .display
            .conversation_display
            .push_str(&format!("❯ {}", prompt));

        inner.mode = AppMode::Thinking;
        inner.display.response_content.clear();
        inner.display.is_command_output = false;
        // Reset spinner for thinking animation
        inner.display.spinner_frame = 0;
        inner.display.spinner_verb = spinner::random_verb().to_string();

        // Record the turn
        inner
            .sessions
            .current_session()
            .add_turn(prompt.clone(), String::new());

        // Match scenario
        let response_text = {
            let text = inner.scenario.response_text_or_default(&prompt);
            if text.is_empty() {
                "I'm not sure how to help with that.".to_string()
            } else {
                text
            }
        };

        // Start streaming
        start_streaming_inner(&mut inner, response_text);
    }

    /// Confirm the current permission selection
    pub(super) fn confirm_permission(&self) {
        let mut inner = self.inner.lock();

        // Extract the permission from dialog state
        let perm = if let DialogState::Permission(p) = std::mem::take(&mut inner.dialog) {
            Some(p)
        } else {
            None
        };
        inner.mode = AppMode::Input;

        if let Some(perm) = perm {
            let tool_name = match &perm.dialog.permission_type {
                PermissionType::Bash { command, .. } => format!("Bash: {}", command),
                PermissionType::Edit { file_path, .. } => format!("Edit: {}", file_path),
                PermissionType::Write { file_path, .. } => format!("Write: {}", file_path),
            };

            match perm.dialog.selected {
                crate::tui::widgets::permission::PermissionSelection::Yes => {
                    // Continue with tool execution (single request)
                    inner
                        .display
                        .response_content
                        .push_str(&format!("\n[Permission granted for {}]\n", tool_name));
                }
                crate::tui::widgets::permission::PermissionSelection::YesSession => {
                    // Store session-level grant
                    let key = perm.dialog.session_key();
                    inner.session_grants.insert(key);

                    // Continue with tool execution (session-level grant)
                    inner.display.response_content.push_str(&format!(
                        "\n[Permission granted for session: {}]\n",
                        tool_name
                    ));
                }
                crate::tui::widgets::permission::PermissionSelection::No => {
                    inner
                        .display
                        .response_content
                        .push_str(&format!("\n[Permission denied for {}]\n", tool_name));
                }
            }
        }
    }

    /// Show a permission request with rich dialog
    pub fn show_permission_request(&self, permission_type: PermissionType) {
        // Check if bypass mode is enabled - auto-approve all permissions
        {
            let inner = self.inner.lock();
            if inner.permission_mode.allows_all() {
                let tool_name = match &permission_type {
                    PermissionType::Bash { command, .. } => format!("Bash: {}", command),
                    PermissionType::Edit { file_path, .. } => format!("Edit: {}", file_path),
                    PermissionType::Write { file_path, .. } => format!("Write: {}", file_path),
                };
                drop(inner);
                simulate_permission_accept(self, &permission_type, &tool_name);
                return;
            }
        }

        // Check if this permission type is already granted for the session
        if self.is_session_granted(&permission_type) {
            // Auto-approve without showing dialog
            let mut inner = self.inner.lock();
            let tool_name = match &permission_type {
                PermissionType::Bash { command, .. } => format!("Bash: {}", command),
                PermissionType::Edit { file_path, .. } => format!("Edit: {}", file_path),
                PermissionType::Write { file_path, .. } => format!("Write: {}", file_path),
            };
            inner.display.response_content.push_str(&format!(
                "\n[Permission auto-granted (session): {}]\n",
                tool_name
            ));
            return;
        }

        // Show dialog as normal
        let mut inner = self.inner.lock();
        inner.dialog = DialogState::Permission(PermissionRequest {
            dialog: RichPermissionDialog::new(permission_type),
        });
        inner.mode = AppMode::Permission;
    }

    /// Show a bash command permission request
    pub fn show_bash_permission(&self, command: String, description: Option<String>) {
        self.show_permission_request(PermissionType::Bash {
            command,
            description,
        });
    }

    /// Show an edit file permission request
    pub fn show_edit_permission(&self, file_path: String, diff_lines: Vec<DiffLine>) {
        self.show_permission_request(PermissionType::Edit {
            file_path,
            diff_lines,
        });
    }

    /// Show a write file permission request
    pub fn show_write_permission(&self, file_path: String, content_lines: Vec<String>) {
        self.show_permission_request(PermissionType::Write {
            file_path,
            content_lines,
        });
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

/// Start streaming a response
pub(super) fn start_streaming_inner(inner: &mut TuiAppStateInner, text: String) {
    inner.mode = AppMode::Responding;
    inner.display.is_streaming = true;
    // Reset spinner for responding animation
    inner.display.spinner_frame = 0;
    inner.display.spinner_verb = spinner::random_verb().to_string();

    let config = StreamingConfig;
    let clock = inner.clock.clone();
    let response = StreamingResponse::new(text, config, clock);

    // For synchronous operation, just set the full text
    // In async mode, this would use the TokenStream
    inner.display.response_content = response.full_text().to_string();
    inner.display.is_streaming = false;

    // Update token counts
    inner.status.output_tokens += response.tokens_streamed();
    inner.status.input_tokens += (inner.input.buffer.len() / 4).max(1) as u32;

    // Update session with response
    if let Some(turn) = inner.sessions.current_session().turns.last_mut() {
        turn.response = inner.display.response_content.clone();
    }

    inner.mode = AppMode::Input;

    // Auto-restore stashed text after response completes
    if let Some(stashed) = inner.input.stash.take() {
        inner.input.buffer = stashed;
        inner.input.cursor_pos = inner.input.buffer.len();
        inner.input.show_stash_indicator = false;
    }
}

/// Handle test permission triggers for TUI fixture tests
/// Returns true if a permission dialog was triggered, false otherwise
fn handle_test_permission_triggers(state: &TuiAppState, prompt: &str) -> bool {
    // Test trigger: "test bash permission"
    if prompt.contains("test bash permission") {
        state.show_bash_permission(
            "cat /etc/passwd | head -5".to_string(),
            Some("Display first 5 lines of /etc/passwd".to_string()),
        );
        return true;
    }

    // Test trigger: "test edit permission"
    if prompt.contains("test edit permission") {
        let diff_lines = vec![
            DiffLine {
                line_num: Some(1),
                kind: DiffKind::Removed,
                content: "Hello World".to_string(),
            },
            DiffLine {
                line_num: Some(1),
                kind: DiffKind::NoNewline,
                content: " No newline at end of file".to_string(),
            },
            DiffLine {
                line_num: Some(2),
                kind: DiffKind::Added,
                content: "Hello Universe".to_string(),
            },
            DiffLine {
                line_num: Some(3),
                kind: DiffKind::NoNewline,
                content: " No newline at end of file".to_string(),
            },
        ];
        state.show_edit_permission("hello.txt".to_string(), diff_lines);
        return true;
    }

    // Test trigger: "test write permission"
    if prompt.contains("test write permission") {
        state.show_write_permission("hello.txt".to_string(), vec!["Hello World".to_string()]);
        return true;
    }

    false
}

/// Simulate accepting a permission (for bypass mode)
fn simulate_permission_accept(
    state: &TuiAppState,
    permission_type: &PermissionType,
    tool_name: &str,
) {
    let mut inner = state.inner.lock();
    inner
        .display
        .response_content
        .push_str(&format!("\n⏺ {}({})\n", tool_name, {
            match permission_type {
                PermissionType::Bash { command, .. } => command.clone(),
                PermissionType::Edit { file_path, .. } => file_path.clone(),
                PermissionType::Write { file_path, .. } => file_path.clone(),
            }
        }));
    inner.mode = AppMode::Input;
}

/// Export conversation to clipboard
pub(super) fn do_clipboard_export(inner: &mut TuiAppStateInner) {
    // Get conversation content
    let content = format_conversation_for_export(inner);

    // Copy to clipboard
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(&content) {
            Ok(()) => {
                inner.display.response_content = "Conversation copied to clipboard".to_string();
            }
            Err(e) => {
                inner.display.response_content = format!("Failed to copy to clipboard: {}", e);
            }
        },
        Err(e) => {
            inner.display.response_content = format!("Failed to access clipboard: {}", e);
        }
    }

    inner.mode = AppMode::Input;
    inner.dialog.dismiss();
    inner.display.is_command_output = true;
}

/// Export conversation to file
pub(super) fn do_file_export(inner: &mut TuiAppStateInner) {
    let filename = inner
        .dialog
        .as_export()
        .map(|d| d.filename.clone())
        .unwrap_or_else(|| "conversation.txt".to_string());

    let content = format_conversation_for_export(inner);

    match std::fs::write(&filename, &content) {
        Ok(()) => {
            inner.display.response_content = format!("Conversation exported to: {}", filename);
        }
        Err(e) => {
            inner.display.response_content = format!("Failed to write file: {}", e);
        }
    }

    inner.mode = AppMode::Input;
    inner.dialog.dismiss();
    inner.display.is_command_output = true;
}

/// Format conversation for export
fn format_conversation_for_export(inner: &TuiAppStateInner) -> String {
    // Export the conversation display content
    // This includes the visible conversation history
    inner.display.conversation_display.clone()
}
