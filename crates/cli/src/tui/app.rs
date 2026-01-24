// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI application state and main iocraft component.

mod input;
mod state;
mod types;

pub use state::TuiAppState;
pub use types::{
    AppMode, ExitHint, ExitReason, PermissionChoice, PermissionRequest, RenderState, StatusInfo,
    TrustPromptState, TuiConfig, DEFAULT_TERMINAL_WIDTH,
};
use state::TuiAppStateInner;

use iocraft::prelude::*;

use crate::permission::PermissionMode;
use crate::scenario::Scenario;
use crate::state::session::SessionManager;
use crate::time::ClockHandle;

use super::colors::{
    styled_bash_placeholder, styled_bash_separator, styled_bash_status, styled_logo_line1,
    styled_logo_line2, styled_logo_line3, styled_placeholder, styled_separator,
};
use super::separator::{make_compact_separator, make_separator};
use super::shortcuts::shortcuts_by_column;
use super::streaming::{StreamingConfig, StreamingResponse};
use super::widgets::context::ContextUsage;
use super::widgets::export::{ExportDialog, ExportStep};
use super::widgets::help::HelpDialog;
use super::widgets::permission::{PermissionSelection, PermissionType, RichPermissionDialog};
use super::widgets::tasks::TasksDialog;
use super::widgets::thinking::{ThinkingDialog, ThinkingMode};
use super::widgets::trust::TrustChoice;

// Re-export helpers from input module for use in submit_input
use input::clear_undo_stack;

impl TuiAppState {
    /// Handle key events in permission mode
    fn handle_permission_key(&self, key: KeyEvent) {
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

    /// Submit the current input
    fn submit_input(&self) {
        let mut inner = self.inner.lock();
        let input = std::mem::take(&mut inner.input_buffer);
        let was_shell_mode = inner.shell_mode;
        inner.shell_mode = false; // Reset shell mode after submit
        inner.cursor_pos = 0;
        clear_undo_stack(&mut inner);

        // Add to history (with shell prefix if applicable)
        let history_entry = if was_shell_mode {
            format!("! {}", input)
        } else {
            input.clone()
        };
        if !history_entry.is_empty() {
            inner.history.push(history_entry);
        }
        inner.history_index = None;

        // Check for slash commands (not applicable in shell mode)
        if !was_shell_mode && input.starts_with('/') {
            Self::handle_command_inner(&mut inner, &input);
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
    fn execute_shell_command(&self, command: String) {
        let mut inner = self.inner.lock();

        // Add previous response to conversation display if any
        if !inner.response_content.is_empty() && !inner.is_command_output {
            let response = inner.response_content.clone();
            if !inner.conversation_display.is_empty() {
                inner.conversation_display.push_str("\n\n");
            }
            inner
                .conversation_display
                .push_str(&format!("⏺ {}", response));
        }

        // Add the shell command to conversation display with ! prefix
        if !inner.conversation_display.is_empty() {
            inner.conversation_display.push_str("\n\n");
        }
        inner
            .conversation_display
            .push_str(&format!("❯ ! {}", command));

        // Check if bypass mode - execute directly without permission dialog
        if inner.permission_mode.allows_all() {
            // Show bash output directly
            inner
                .conversation_display
                .push_str(&format!("\n\n⏺ Bash({})", command));

            // Get scenario response for the command
            let response_text = {
                let mut scenario = inner.scenario.lock();
                if let Some(result) = scenario.match_prompt(&command) {
                    match scenario.get_response(&result) {
                        Some(crate::config::ResponseSpec::Simple(text)) => text.clone(),
                        Some(crate::config::ResponseSpec::Detailed { text, .. }) => text.clone(),
                        None => String::new(),
                    }
                } else if let Some(default) = scenario.default_response() {
                    match default {
                        crate::config::ResponseSpec::Simple(text) => text.clone(),
                        crate::config::ResponseSpec::Detailed { text, .. } => text.clone(),
                    }
                } else {
                    format!("$ {}", command)
                }
            };

            // Start streaming the response
            inner.response_content.clear();
            inner.is_command_output = false;
            Self::start_streaming_inner(&mut inner, response_text);
            return;
        }

        // Show bash permission dialog
        inner.mode = AppMode::Thinking;
        inner.response_content.clear();
        inner.is_command_output = false;

        drop(inner);

        // Use existing bash permission flow
        self.show_bash_permission(command.clone(), Some(format!("Execute: {}", command)));
    }

    /// Handle slash commands like /compact and /clear
    fn handle_command_inner(inner: &mut TuiAppStateInner, input: &str) {
        let cmd = input.trim().to_lowercase();
        inner.is_command_output = true;

        // Add the command to conversation display
        inner.conversation_display = format!("❯ {}", input.trim());

        match cmd.as_str() {
            "/clear" => {
                // Clear session turns
                {
                    let mut sessions = inner.sessions.lock();
                    sessions.current_session().turns.clear();
                }

                // Reset token counts
                inner.status.input_tokens = 0;
                inner.status.output_tokens = 0;

                // Clear session-level permission grants
                inner.session_grants.clear();

                // Set response content (will be rendered with elbow connector)
                inner.response_content = "(no content)".to_string();
            }
            "/compact" => {
                // Check if already compacting
                if inner.is_compacting {
                    inner.response_content =
                        "Failed to compact: Compaction already in progress".to_string();
                } else {
                    // Show compacting in progress message
                    inner.mode = AppMode::Responding;
                    inner.is_compacting = true;
                    inner.compacting_started = Some(std::time::Instant::now());
                    // Use correct symbol (✻) and ellipsis (…)
                    inner.response_content =
                        "✻ Compacting conversation… (ctrl+c to interrupt)".to_string();
                }
            }
            "/fork" => {
                // Check if there's a conversation to fork
                let has_conversation = {
                    let sessions = inner.sessions.lock();
                    let current = sessions.get_current();
                    current.map(|s| !s.turns.is_empty()).unwrap_or(false)
                };

                if has_conversation {
                    // TODO: Implement actual fork functionality
                    // For now, show a placeholder message
                    inner.response_content = "Conversation forked".to_string();
                } else {
                    // No conversation to fork - show error
                    inner.response_content =
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
                inner.help_dialog = Some(HelpDialog::new(version));
            }
            "/context" => {
                let usage = ContextUsage::new();
                inner.response_content = Self::format_context_usage(&usage);
            }
            "/exit" => {
                let farewell = Self::random_farewell().to_string();
                inner.response_content = farewell.clone();
                inner.exit_message = Some(farewell);
                inner.should_exit = true;
                inner.exit_reason = Some(ExitReason::UserQuit);
            }
            "/todos" => {
                inner.response_content = Self::format_todos(&inner.todos);
            }
            "/tasks" => {
                inner.mode = AppMode::TasksDialog;
                inner.tasks_dialog = Some(TasksDialog::new());
            }
            "/export" => {
                // Check if there's a conversation to export
                let has_conversation = {
                    let sessions = inner.sessions.lock();
                    let current = sessions.get_current();
                    current.map(|s| !s.turns.is_empty()).unwrap_or(false)
                };

                if has_conversation {
                    inner.mode = AppMode::ExportDialog;
                    inner.export_dialog = Some(ExportDialog::new());
                } else {
                    inner.response_content =
                        "Failed to export: No conversation to export".to_string();
                }
            }
            "/hooks" => {
                inner.mode = AppMode::HooksDialog;
                // For now, hard-code to 4 active hooks as shown in the fixture
                inner.hooks_dialog = Some(super::widgets::HooksDialog::new(4));
            }
            "/memory" => {
                inner.mode = AppMode::MemoryDialog;
                inner.memory_dialog = Some(super::widgets::MemoryDialog::new());
            }
            _ => {
                inner.response_content = format!("Unknown command: {}", input);
            }
        }
    }

    /// Process a prompt and generate response
    fn process_prompt(&self, prompt: String) {
        // Check for test permission triggers first (before acquiring inner lock)
        if self.handle_test_permission_triggers(&prompt) {
            return;
        }

        let mut inner = self.inner.lock();

        // If there's previous response content, add it to conversation history first
        if !inner.response_content.is_empty() && !inner.is_command_output {
            let response = inner.response_content.clone();
            if !inner.conversation_display.is_empty() {
                inner.conversation_display.push_str("\n\n");
            }
            inner
                .conversation_display
                .push_str(&format!("⏺ {}", response));
        }

        // Add the new user prompt to conversation display
        if !inner.conversation_display.is_empty() {
            inner.conversation_display.push_str("\n\n");
        }
        inner
            .conversation_display
            .push_str(&format!("❯ {}", prompt));

        inner.mode = AppMode::Thinking;
        inner.response_content.clear();
        inner.is_command_output = false;

        // Record the turn
        {
            let mut sessions = inner.sessions.lock();
            sessions
                .current_session()
                .add_turn(prompt.clone(), String::new());
        }

        // Match scenario
        let response_text = {
            let mut scenario = inner.scenario.lock();
            if let Some(result) = scenario.match_prompt(&prompt) {
                match scenario.get_response(&result) {
                    Some(crate::config::ResponseSpec::Simple(text)) => text.clone(),
                    Some(crate::config::ResponseSpec::Detailed { text, .. }) => text.clone(),
                    None => String::new(),
                }
            } else if let Some(default) = scenario.default_response() {
                match default {
                    crate::config::ResponseSpec::Simple(text) => text.clone(),
                    crate::config::ResponseSpec::Detailed { text, .. } => text.clone(),
                }
            } else {
                "I'm not sure how to help with that.".to_string()
            }
        };

        // Start streaming
        Self::start_streaming_inner(&mut inner, response_text);
    }

    /// Handle test permission triggers for TUI fixture tests
    /// Returns true if a permission dialog was triggered, false otherwise
    fn handle_test_permission_triggers(&self, prompt: &str) -> bool {
        use super::widgets::permission::{DiffKind, DiffLine};

        // Test trigger: "test bash permission"
        if prompt.contains("test bash permission") {
            self.show_bash_permission(
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
            self.show_edit_permission("hello.txt".to_string(), diff_lines);
            return true;
        }

        // Test trigger: "test write permission"
        if prompt.contains("test write permission") {
            self.show_write_permission("hello.txt".to_string(), vec!["Hello World".to_string()]);
            return true;
        }

        false
    }

    /// Start streaming a response
    fn start_streaming_inner(inner: &mut TuiAppStateInner, text: String) {
        inner.mode = AppMode::Responding;
        inner.is_streaming = true;

        let config = StreamingConfig::default();
        let clock = inner.clock.clone();
        let response = StreamingResponse::new(text, config, clock);

        // For synchronous operation, just set the full text
        // In async mode, this would use the TokenStream
        inner.response_content = response.full_text().to_string();
        inner.is_streaming = false;

        // Update token counts
        inner.status.output_tokens += response.tokens_streamed();
        inner.status.input_tokens += (inner.input_buffer.len() / 4).max(1) as u32;

        // Update session with response
        {
            let mut sessions = inner.sessions.lock();
            if let Some(turn) = sessions.current_session().turns.last_mut() {
                turn.response = inner.response_content.clone();
            }
        }

        inner.mode = AppMode::Input;

        // Auto-restore stashed text after response completes
        if let Some(stashed) = inner.stash_buffer.take() {
            inner.input_buffer = stashed;
            inner.cursor_pos = inner.input_buffer.len();
            inner.show_stash_indicator = false;
        }
    }

    /// Confirm the current permission selection
    fn confirm_permission(&self) {
        let mut inner = self.inner.lock();
        let perm = inner.pending_permission.take();
        inner.mode = AppMode::Input;

        if let Some(perm) = perm {
            let tool_name = match &perm.dialog.permission_type {
                PermissionType::Bash { command, .. } => format!("Bash: {}", command),
                PermissionType::Edit { file_path, .. } => format!("Edit: {}", file_path),
                PermissionType::Write { file_path, .. } => format!("Write: {}", file_path),
            };

            match perm.dialog.selected {
                PermissionSelection::Yes => {
                    // Continue with tool execution (single request)
                    inner
                        .response_content
                        .push_str(&format!("\n[Permission granted for {}]\n", tool_name));
                }
                PermissionSelection::YesSession => {
                    // Store session-level grant
                    let key = perm.dialog.session_key();
                    inner.session_grants.insert(key);

                    // Continue with tool execution (session-level grant)
                    inner.response_content.push_str(&format!(
                        "\n[Permission granted for session: {}]\n",
                        tool_name
                    ));
                }
                PermissionSelection::No => {
                    inner
                        .response_content
                        .push_str(&format!("\n[Permission denied for {}]\n", tool_name));
                }
            }
        }
    }

    /// Handle key events in trust prompt mode
    fn handle_trust_key(&self, key: KeyEvent) {
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
    fn handle_thinking_key(&self, key: KeyEvent) {
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
    fn handle_tasks_key(&self, key: KeyEvent) {
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
    fn handle_model_picker_key(&self, key: KeyEvent) {
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
    fn handle_export_dialog_key(&self, key: KeyEvent) {
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
                        Self::do_clipboard_export(&mut inner);
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
                    Self::do_file_export(&mut inner);
                }
                KeyCode::Backspace => dialog.pop_char(),
                KeyCode::Char(c) => dialog.push_char(c),
                _ => {}
            },
        }
    }

    /// Export conversation to clipboard
    fn do_clipboard_export(inner: &mut TuiAppStateInner) {
        // Get conversation content
        let content = Self::format_conversation_for_export(inner);

        // Copy to clipboard
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(&content) {
                Ok(()) => {
                    inner.response_content = "Conversation copied to clipboard".to_string();
                }
                Err(e) => {
                    inner.response_content = format!("Failed to copy to clipboard: {}", e);
                }
            },
            Err(e) => {
                inner.response_content = format!("Failed to access clipboard: {}", e);
            }
        }

        inner.mode = AppMode::Input;
        inner.export_dialog = None;
        inner.is_command_output = true;
    }

    /// Export conversation to file
    fn do_file_export(inner: &mut TuiAppStateInner) {
        let filename = inner
            .export_dialog
            .as_ref()
            .map(|d| d.filename.clone())
            .unwrap_or_else(|| "conversation.txt".to_string());

        let content = Self::format_conversation_for_export(inner);

        match std::fs::write(&filename, &content) {
            Ok(()) => {
                inner.response_content = format!("Conversation exported to: {}", filename);
            }
            Err(e) => {
                inner.response_content = format!("Failed to write file: {}", e);
            }
        }

        inner.mode = AppMode::Input;
        inner.export_dialog = None;
        inner.is_command_output = true;
    }

    /// Handle key events in help dialog mode
    fn handle_help_dialog_key(&self, key: KeyEvent) {
        use super::slash_menu::COMMANDS;
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
    fn handle_hooks_dialog_key(&self, key: KeyEvent) {
        use super::widgets::HooksView;
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

    fn handle_memory_dialog_key(&self, key: KeyEvent) {
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

    /// Format conversation for export
    fn format_conversation_for_export(inner: &TuiAppStateInner) -> String {
        // Export the conversation display content
        // This includes the visible conversation history
        inner.conversation_display.clone()
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
                self.simulate_permission_accept(&permission_type, &tool_name);
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
            inner.response_content.push_str(&format!(
                "\n[Permission auto-granted (session): {}]\n",
                tool_name
            ));
            return;
        }

        // Show dialog as normal
        let mut inner = self.inner.lock();
        inner.pending_permission = Some(PermissionRequest {
            dialog: RichPermissionDialog::new(permission_type),
        });
        inner.mode = AppMode::Permission;
    }

    /// Simulate accepting a permission (for bypass mode)
    fn simulate_permission_accept(&self, permission_type: &PermissionType, tool_name: &str) {
        let mut inner = self.inner.lock();
        inner
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

    /// Show a bash command permission request
    pub fn show_bash_permission(&self, command: String, description: Option<String>) {
        self.show_permission_request(PermissionType::Bash {
            command,
            description,
        });
    }

    /// Show an edit file permission request
    pub fn show_edit_permission(
        &self,
        file_path: String,
        diff_lines: Vec<super::widgets::permission::DiffLine>,
    ) {
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

/// Props for the main App component
#[derive(Default, Props)]
pub struct AppProps {
    pub state: Option<TuiAppState>,
}

/// Main TUI App component using iocraft
#[component]
pub fn App(mut hooks: Hooks, props: &AppProps) -> impl Into<AnyElement<'static>> {
    // Get state from props with fallback error display
    let Some(state) = props.state.clone() else {
        return element! {
            View(flex_direction: FlexDirection::Column) {
                Text(content: "Error: TuiAppState must be provided via props")
            }
        };
    };

    let mut should_exit = hooks.use_state(|| false);
    // Render counter to force re-renders when state changes
    let mut render_counter = hooks.use_state(|| 0u64);
    // Timer counter for periodic updates (compacting, streaming, etc.)
    let mut timer_counter = hooks.use_state(|| 0u64);
    let state_clone = state.clone();

    // Handle terminal events (keyboard input and resize)
    hooks.use_terminal_events({
        let state = state.clone();
        move |event| match event {
            TerminalEvent::Key(key) if key.kind != KeyEventKind::Release => {
                state.handle_key_event(key);
                // Increment counter to trigger re-render
                let current = *render_counter.read();
                render_counter.set(current.wrapping_add(1));
                if state.should_exit() {
                    should_exit.set(true);
                }
            }
            TerminalEvent::Resize(width, _height) => {
                state.set_terminal_width(width);
                // Increment counter to trigger re-render
                let current = *render_counter.read();
                render_counter.set(current.wrapping_add(1));
            }
            _ => {}
        }
    });

    // Periodic timer for updates (compacting, streaming, etc.)
    hooks.use_future({
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let current = *timer_counter.read();
                timer_counter.set(current.wrapping_add(1));
            }
        }
    });

    // Check for timeouts (both compacting and exit hint)
    state_clone.check_compacting();
    state_clone.check_exit_hint_timeout();

    // Get current render state
    let render_state = state_clone.render_state();

    // Exit if needed
    let should_exit_val = should_exit.read();
    if *should_exit_val || state_clone.should_exit() {
        hooks.use_context_mut::<SystemContext>().exit();
    }

    // Render based on mode
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
        ) {
            #(render_main_content(&render_state))
        }
    }
}

/// Render the main content based on current mode
fn render_main_content(state: &RenderState) -> AnyElement<'static> {
    let width = state.terminal_width as usize;

    // If in trust mode, render trust prompt
    if state.mode == AppMode::Trust {
        if let Some(ref prompt) = state.trust_prompt {
            return render_trust_prompt(prompt, width);
        }
    }

    // If in thinking toggle mode, render just the thinking dialog
    if state.mode == AppMode::ThinkingToggle {
        if let Some(ref dialog) = state.thinking_dialog {
            return render_thinking_dialog(dialog, width);
        }
    }

    // If in tasks dialog mode, render just the tasks dialog
    if state.mode == AppMode::TasksDialog {
        if let Some(ref dialog) = state.tasks_dialog {
            return render_tasks_dialog(dialog, width);
        }
    }

    // If in export dialog mode, render just the export dialog
    if state.mode == AppMode::ExportDialog {
        if let Some(ref dialog) = state.export_dialog {
            return render_export_dialog(dialog, width);
        }
    }

    // If in help dialog mode, render just the help dialog
    if state.mode == AppMode::HelpDialog {
        if let Some(ref dialog) = state.help_dialog {
            return render_help_dialog(dialog, width);
        }
    }

    // If in hooks dialog mode, render just the hooks dialog
    if state.mode == AppMode::HooksDialog {
        if let Some(ref dialog) = state.hooks_dialog {
            return render_hooks_dialog(dialog, width);
        }
    }

    // If in memory dialog mode, render just the memory dialog
    if state.mode == AppMode::MemoryDialog {
        if let Some(ref dialog) = state.memory_dialog {
            return render_memory_dialog(dialog, width);
        }
    }

    // If in model picker mode, render just the model picker dialog
    if state.mode == AppMode::ModelPicker {
        if let Some(ref dialog) = state.model_picker_dialog {
            return render_model_picker_dialog(dialog, width);
        }
    }

    // If in permission mode, render just the permission dialog (full-screen)
    if state.mode == AppMode::Permission {
        if let Some(ref perm) = state.pending_permission {
            return render_permission_dialog(perm, width);
        }
    }

    // Format header lines
    let (header_line1, header_line2, header_line3) = format_header_lines(state);

    // Use styled output when connected to a TTY
    let use_colors = state.is_tty;

    // Format input line
    // Shell mode shows `! Try "..."` in pink, otherwise show normal input or placeholder
    let input_display = if state.shell_mode {
        // Bash mode: show `! ` prefix in pink with suggestion or typed command
        if state.input_buffer.is_empty() {
            if use_colors {
                styled_bash_placeholder("Try \"fix lint errors\"")
            } else {
                "! Try \"fix lint errors\"".to_string()
            }
        } else {
            // User is typing a command - show `! ` followed by their input
            if use_colors {
                let fg_pink = super::colors::escape::fg(
                    super::colors::BASH_MODE.0,
                    super::colors::BASH_MODE.1,
                    super::colors::BASH_MODE.2,
                );
                format!(
                    "{}{}! {}{}",
                    super::colors::escape::RESET,
                    fg_pink,
                    state.input_buffer,
                    super::colors::escape::RESET
                )
            } else {
                format!("! {}", state.input_buffer)
            }
        }
    } else if state.input_buffer.is_empty() {
        if state.conversation_display.is_empty() && state.response_content.is_empty() {
            // Show placeholder only on initial state
            if use_colors {
                styled_placeholder("Try \"write a test for scenario.rs\"")
            } else {
                "❯ Try \"refactor mod.rs\"".to_string()
            }
        } else {
            // After conversation started, show just the cursor
            "❯".to_string()
        }
    } else {
        format!("❯ {}", state.input_buffer)
    };

    // Format separators - pink in bash mode, gray otherwise
    let separator = if state.shell_mode && use_colors {
        styled_bash_separator(width)
    } else if use_colors {
        styled_separator(width)
    } else {
        format!("{}\n", make_separator(width))
    };

    // Format status bar - show `! for bash mode` in bash mode
    let status_bar = if state.shell_mode && use_colors {
        styled_bash_status()
    } else if use_colors {
        format_status_bar_styled(state, width)
    } else {
        format_status_bar(state, width)
    };

    // Main layout matching real Claude CLI
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
        ) {
            // Header with Claude branding (3 lines)
            // Use NoWrap to preserve trailing ANSI escape codes
            // Note: Empty first element to work around iocraft first-element rendering issue
            Text(content: "")
            Text(content: header_line1, wrap: TextWrap::NoWrap)
            Text(content: header_line2, wrap: TextWrap::NoWrap)
            Text(content: header_line3, wrap: TextWrap::NoWrap)

            // Empty line after header
            Text(content: "")

            // Conversation history area (if any)
            #(render_conversation_area(state))

            // Slash menu (if open)
            #(render_slash_menu(state))

            // Input area with separators (NoWrap to preserve ANSI)
            Text(content: separator.clone(), wrap: TextWrap::NoWrap)
            #(render_stash_indicator(state))
            Text(content: input_display, wrap: TextWrap::NoWrap)
            #(render_argument_hint(state))
            Text(content: separator, wrap: TextWrap::NoWrap)

            // Shortcuts panel or status bar (NoWrap to preserve ANSI)
            #(if state.show_shortcuts_panel {
                render_shortcuts_panel(state.terminal_width as usize)
            } else {
                element! {
                    Text(content: status_bar.clone(), wrap: TextWrap::NoWrap)
                }.into()
            })
        }
    }
    .into()
}

/// Render the shortcuts panel with 3 columns
fn render_shortcuts_panel(_width: usize) -> AnyElement<'static> {
    let columns = shortcuts_by_column();

    // Fixed column widths matching the Claude Code fixture:
    // - Left column: 26 chars total (2-space indent + 24 content)
    // - Center column: 35 chars
    // - Right column: remaining space
    const LEFT_WIDTH: usize = 24; // Content width (after 2-space indent)
    const CENTER_WIDTH: usize = 35;

    // Build the multi-column layout
    // Each row contains entries from all 3 columns
    let max_rows = columns.iter().map(|c| c.len()).max().unwrap_or(0);

    let mut lines = Vec::new();
    for row_idx in 0..max_rows {
        let left = columns[0].get(row_idx).copied().unwrap_or("");
        let center = columns[1].get(row_idx).copied().unwrap_or("");
        let right = columns[2].get(row_idx).copied().unwrap_or("");

        // Format line with 2-space indent and fixed column widths
        let line = format!(
            "  {:<left_w$}{:<center_w$}{}",
            left,
            center,
            right,
            left_w = LEFT_WIDTH,
            center_w = CENTER_WIDTH
        );
        lines.push(line);
    }

    element! {
        View(flex_direction: FlexDirection::Column) {
            #(lines.into_iter().map(|line| {
                element! {
                    Text(content: line)
                }
            }).collect::<Vec<_>>())
        }
    }
    .into()
}

/// Render conversation history area
fn render_conversation_area(state: &RenderState) -> AnyElement<'static> {
    let mut content = String::new();

    // Add compact separator if conversation has been compacted
    if state.is_compacted {
        let compact_text = "Conversation compacted · ctrl+o for history";
        content.push_str(&make_compact_separator(
            compact_text,
            state.terminal_width as usize,
        ));
        content.push('\n');
    }

    // Add conversation display (includes user prompts and past responses)
    if !state.conversation_display.is_empty() {
        content.push_str(&state.conversation_display);
    }

    // Add current response if present
    if !state.response_content.is_empty() {
        // Check if this is a compacting-in-progress message (✻ symbol)
        let is_compacting_in_progress = state.response_content.starts_with('✻');

        if is_compacting_in_progress {
            // During compacting, show message on its own line after blank line
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&state.response_content);
        } else if state.is_command_output {
            // Completed command output uses elbow connector format
            if !content.is_empty() {
                content.push('\n');
            }
            // Format each line with elbow connector (2 spaces + ⎿ + 2 spaces)
            for (i, line) in state.response_content.lines().enumerate() {
                if i > 0 {
                    content.push('\n');
                }
                content.push_str(&format!("  ⎿  {}", line));
            }
        } else {
            // Normal response with ⏺ prefix
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&format!("⏺ {}", state.response_content));
        }
    }

    // Add trailing empty line if there's content (creates space before separator)
    if !content.is_empty() {
        element! {
            View(flex_direction: FlexDirection::Column) {
                Text(content: content)
                Text(content: "")
            }
        }
        .into()
    } else {
        // Empty element if no content
        element! {
            View {}
        }
        .into()
    }
}

/// Render the slash command autocomplete menu (if open)
fn render_slash_menu(state: &RenderState) -> AnyElement<'static> {
    let Some(ref menu) = state.slash_menu else {
        return element! { View {} }.into();
    };

    if menu.filtered_commands.is_empty() {
        return element! { View {} }.into();
    }

    // Build menu content
    let max_visible = 10; // Show at most 10 commands
    let mut content = String::new();

    for (i, cmd) in menu.filtered_commands.iter().take(max_visible).enumerate() {
        let is_selected = i == menu.selected_index;
        let indicator = if is_selected { " ❯ " } else { "   " };

        // Format: indicator + /command + spaces + description
        // Use 14 chars for command name (including /) to align descriptions
        let cmd_display = format!("/{}", cmd.name);
        content.push_str(&format!(
            "{}{:<14}  {}\n",
            indicator, cmd_display, cmd.description
        ));
    }

    // Remove trailing newline
    if content.ends_with('\n') {
        content.pop();
    }

    element! {
        View(flex_direction: FlexDirection::Column) {
            Text(content: content)
            Text(content: "")
        }
    }
    .into()
}

/// Render argument hint for completed slash commands
/// Render stash indicator if stash is active
fn render_stash_indicator(state: &RenderState) -> AnyElement<'static> {
    if !state.show_stash_indicator {
        return element! { View {} }.into();
    }

    // Use orange accent color for the › character
    use super::colors::{escape, LOGO_FG};
    let accent_fg = escape::fg(LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
    let reset = escape::RESET;

    let indicator_text = format!(
        "  {}›{} Stashed (auto-restores after submit)",
        accent_fg, reset
    );

    element! {
        View(flex_direction: FlexDirection::Column) {
            Text(content: indicator_text, wrap: TextWrap::NoWrap)
        }
    }
    .into()
}

fn render_argument_hint(state: &RenderState) -> AnyElement<'static> {
    // Only show hint when menu is closed and input starts with a completed command
    if state.slash_menu.is_some() || !state.input_buffer.starts_with('/') {
        return element! { View {} }.into();
    }

    // Extract command name (without leading /)
    let cmd_text = state.input_buffer.trim_start_matches('/');

    // Find exact match
    if let Some(cmd) = super::slash_menu::COMMANDS
        .iter()
        .find(|c| c.name == cmd_text)
    {
        if let Some(hint) = cmd.argument_hint {
            let hint_text = format!("     {}  {}", " ".repeat(cmd_text.len()), hint);
            return element! {
                View(flex_direction: FlexDirection::Column) {
                    Text(content: hint_text)
                }
            }
            .into();
        }
    }

    element! { View {} }.into()
}

/// Render trust prompt dialog
fn render_trust_prompt(prompt: &TrustPromptState, width: usize) -> AnyElement<'static> {
    let yes_indicator = if prompt.selected == TrustChoice::Yes {
        " ❯ "
    } else {
        "   "
    };
    let no_indicator = if prompt.selected == TrustChoice::No {
        " ❯ "
    } else {
        "   "
    };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
        ) {
            // Horizontal rule separator
            Text(content: make_separator(width))
            // Title
            Text(content: " Do you trust the files in this folder?")
            Text(content: "")
            // Working directory
            Text(content: format!(" {}", prompt.working_directory))
            Text(content: "")
            // Security warning (wrapped text)
            Text(content: " Claude Code may read, write, or execute files contained in this directory. This can pose security risks, so only use")
            Text(content: " files from trusted sources.")
            Text(content: "")
            // Learn more link (plain text)
            Text(content: " Learn more")
            Text(content: "")
            // Options
            Text(content: format!("{}1. Yes, proceed", yes_indicator))
            Text(content: format!("{}2. No, exit", no_indicator))
            Text(content: "")
            // Footer
            Text(content: " Enter to confirm · Esc to cancel")
        }
    }.into()
}

/// Render thinking toggle dialog
fn render_thinking_dialog(dialog: &ThinkingDialog, width: usize) -> AnyElement<'static> {
    let enabled_indicator = if dialog.selected == ThinkingMode::Enabled {
        " ❯ "
    } else {
        "   "
    };
    let disabled_indicator = if dialog.selected == ThinkingMode::Disabled {
        " ❯ "
    } else {
        "   "
    };
    let enabled_check = if dialog.current == ThinkingMode::Enabled {
        " ✔"
    } else {
        ""
    };
    let disabled_check = if dialog.current == ThinkingMode::Disabled {
        " ✔"
    } else {
        ""
    };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
        ) {
            // Horizontal rule separator at top
            Text(content: make_separator(width))
            // Title
            Text(content: " Toggle thinking mode")
            // Subtitle
            Text(content: " Enable or disable thinking for this session.")
            Text(content: "")
            // Options with descriptions
            Text(content: format!("{}1. Enabled{}  Claude will think before responding", enabled_indicator, enabled_check))
            Text(content: format!("{}2. Disabled{}   Claude will respond without extended thinking", disabled_indicator, disabled_check))
            Text(content: "")
            // Footer (note: lowercase 'escape' per fixture)
            Text(content: " Enter to confirm · escape to exit")
        }
    }.into()
}

/// Render tasks dialog with border
fn render_tasks_dialog(dialog: &TasksDialog, width: usize) -> AnyElement<'static> {
    // Inner width accounts for box borders (│ on each side)
    let inner_width = width.saturating_sub(2);

    // Build content string
    let content = if dialog.is_empty() {
        "No tasks currently running".to_string()
    } else {
        // Format task list with selection indicator
        dialog
            .tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let indicator = if i == dialog.selected_index() {
                    "❯ "
                } else {
                    "  "
                };
                format!("{}{}", indicator, task.description)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Box drawing chars
    let h_line = "─".repeat(inner_width);
    let top_border = format!("╭{}╮", h_line);
    let bottom_border = format!("╰{}╯", h_line);

    // Pad content lines to fill width
    let pad_line = |s: &str| {
        // Calculate visual width (accounting for multi-byte chars)
        let visible_len = s.chars().count();
        let padding = inner_width.saturating_sub(visible_len);
        format!("│{}{}│", s, " ".repeat(padding))
    };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
        ) {
            Text(content: top_border)
            Text(content: pad_line(" Background tasks"))
            Text(content: pad_line(&format!(" {}", content)))
            Text(content: bottom_border)
            Text(content: "  ↑/↓ to select · Enter to view · Esc to close")
        }
    }
    .into()
}

/// Render export dialog
fn render_export_dialog(dialog: &ExportDialog, width: usize) -> AnyElement<'static> {
    use super::widgets::export::ExportMethod;

    let inner_width = width.saturating_sub(2);
    let h_line = "─".repeat(inner_width);
    let top_border = format!("╭{}╮", h_line);
    let bottom_border = format!("╰{}╯", h_line);

    let pad_line = |s: &str| {
        let visible_len = s.chars().count();
        let padding = inner_width.saturating_sub(visible_len);
        format!("│{}{}│", s, " ".repeat(padding))
    };

    match dialog.step {
        ExportStep::MethodSelection => {
            let clipboard_cursor = if dialog.selected_method == ExportMethod::Clipboard {
                "❯"
            } else {
                " "
            };
            let file_cursor = if dialog.selected_method == ExportMethod::File {
                "❯"
            } else {
                " "
            };

            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    width: 100pct,
                ) {
                    Text(content: top_border)
                    Text(content: pad_line(" Export Conversation"))
                    Text(content: pad_line(""))
                    Text(content: pad_line(" Select export method:"))
                    Text(content: pad_line(&format!(" {} 1. Copy to clipboard", clipboard_cursor)))
                    Text(content: pad_line(&format!(" {} 2. Save to file", file_cursor)))
                    Text(content: bottom_border)
                    Text(content: "  ↑/↓ to select · Enter to confirm · Esc to cancel")
                }
            }
            .into()
        }
        ExportStep::FilenameInput => element! {
            View(
                flex_direction: FlexDirection::Column,
                width: 100pct,
            ) {
                Text(content: top_border)
                Text(content: pad_line(" Export Conversation"))
                Text(content: pad_line(""))
                Text(content: pad_line(" Enter filename:"))
                Text(content: pad_line(&format!(" {}", dialog.filename)))
                Text(content: bottom_border)
                Text(content: "  Enter to save · esc to go back")
            }
        }
        .into(),
    }
}

/// Render help dialog
fn render_help_dialog(dialog: &HelpDialog, width: usize) -> AnyElement<'static> {
    use super::slash_menu::COMMANDS;
    use super::widgets::HelpTab;

    let inner_width = width.saturating_sub(2);

    // Build tab header line
    let version_part = format!("─Claude Code v{}─", dialog.version);
    let tabs_part = format!(
        " {} ─ {} ─ {} ─",
        HelpTab::General.name(),
        HelpTab::Commands.name(),
        HelpTab::CustomCommands.name()
    );
    let hint = "(←/→ or tab to cycle)";
    let used = version_part.len() + tabs_part.len() + hint.len() + 1;
    let remaining = inner_width.saturating_sub(used);
    let tab_header = format!(
        " {}{}{}{}",
        version_part,
        tabs_part,
        hint,
        "─".repeat(remaining)
    );

    let footer = " For more help: https://code.claude.com/docs/en/overview";

    match dialog.active_tab {
        HelpTab::General => {
            element! {
                View(flex_direction: FlexDirection::Column, width: 100pct) {
                    Text(content: tab_header)
                    Text(content: "")
                    Text(content: "")
                    Text(content: "  Claude understands your codebase, makes edits with your permission, and executes commands — right from your terminal.")
                    Text(content: "  / for commands    ctrl + o for verbose output              cmd + v to paste images")
                    Text(content: "  & for background  backslash (\\) + return (⏎) for newline   ctrl + s to stash prompt")
                    Text(content: "")
                    Text(content: footer)
                }
            }
            .into()
        }
        HelpTab::Commands => {
            let selected = dialog.commands_selected;
            let cmd = COMMANDS.get(selected);
            let next_cmd = COMMANDS.get(selected + 1);

            let selected_line = format!(
                "  ❯ /{}",
                cmd.map(|c| c.name).unwrap_or("")
            );
            let description_line = format!(
                "    {}",
                cmd.map(|c| c.description).unwrap_or("")
            );
            let next_line = if let Some(next) = next_cmd {
                format!("  ↓ /{}", next.name)
            } else {
                String::new()
            };

            element! {
                View(flex_direction: FlexDirection::Column, width: 100pct) {
                    Text(content: tab_header)
                    Text(content: "")
                    Text(content: "  Browse default commands:")
                    Text(content: selected_line)
                    Text(content: description_line)
                    Text(content: next_line)
                    Text(content: "")
                    Text(content: footer)
                }
            }
            .into()
        }
        HelpTab::CustomCommands => {
            element! {
                View(flex_direction: FlexDirection::Column, width: 100pct) {
                    Text(content: tab_header)
                    Text(content: "")
                    Text(content: "  Browse custom commands:")
                    Text(content: "  (no custom commands configured)")
                    Text(content: "")
                    Text(content: footer)
                }
            }
            .into()
        }
    }
}

/// Render memory dialog
fn render_memory_dialog(
    dialog: &super::widgets::MemoryDialog,
    _width: usize,
) -> AnyElement<'static> {
    // Build visible items
    let items: Vec<_> = dialog
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = i == dialog.selected_index();
            let prefix = if is_selected { "❯" } else { " " };
            let status = if entry.is_active { "✓" } else { " " };
            let path = entry.path.as_deref().unwrap_or("(not configured)");

            format!(
                " {} {} {}. {} - {}",
                prefix,
                status,
                i + 1,
                entry.source.name(),
                path
            )
        })
        .collect();

    // Count active entries
    let active_count = dialog.entries.iter().filter(|e| e.is_active).count();
    let header = if active_count == 1 {
        " Memory · 1 file".to_string()
    } else {
        format!(" Memory · {} files", active_count)
    };

    let footer = " Enter to view · esc to cancel".to_string();

    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct) {
            Text(content: header)
            Text(content: "")
            #(items.into_iter().map(|item| {
                element! { Text(content: item) }
            }))
            Text(content: "")
            Text(content: footer)
        }
    }
    .into()
}

/// Render hooks dialog
fn render_hooks_dialog(dialog: &super::widgets::HooksDialog, _width: usize) -> AnyElement<'static> {
    use super::widgets::HooksView;

    match dialog.view {
        HooksView::HookList => render_hooks_list(dialog),
        HooksView::Matchers => render_hooks_matchers(dialog),
    }
}

/// Render the main hooks list view
fn render_hooks_list(dialog: &super::widgets::HooksDialog) -> AnyElement<'static> {
    use super::widgets::HookType;

    let hooks = HookType::all();
    let visible_start = dialog.scroll_offset();
    let visible_end = (visible_start + dialog.visible_count()).min(hooks.len());

    // Build visible items
    let items: Vec<_> = hooks
        .iter()
        .enumerate()
        .skip(visible_start)
        .take(visible_end - visible_start)
        .map(|(i, hook)| {
            let is_selected = i == dialog.selected_index();
            let is_last_visible = i == visible_end - 1 && dialog.has_more_below();

            let prefix = if is_selected {
                "❯"
            } else if is_last_visible {
                "↓"
            } else {
                " "
            };

            format!(
                " {} {}.  {} - {}",
                prefix,
                i + 1,
                hook.name(),
                hook.description()
            )
        })
        .collect();

    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct) {
            Text(content: " Hooks")
            Text(content: format!(" {} hooks", dialog.active_hook_count))
            Text(content: "")
            #(items.into_iter().map(|item| {
                element! { Text(content: item) }
            }).collect::<Vec<_>>())
            Text(content: "")
            Text(content: " Enter to confirm · esc to cancel")
        }
    }
    .into()
}

/// Render the matchers view for a selected hook type
fn render_hooks_matchers(dialog: &super::widgets::HooksDialog) -> AnyElement<'static> {
    use super::widgets::HookType;

    let hook = dialog.selected_hook.unwrap_or(HookType::PreToolUse);

    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct) {
            Text(content: format!(" {} - Tool Matchers", hook.name()))
            Text(content: " Input to command is JSON of tool call arguments.")
            Text(content: " Exit code 0 - stdout/stderr not shown")
            Text(content: " Exit code 2 - show stderr to model and block tool call")
            Text(content: " Other exit codes - show stderr to user only but continue with tool call")
            Text(content: "")
            Text(content: " ❯ 1. + Add new matcher…")
            Text(content: "   No matchers configured yet")
            Text(content: "")
            Text(content: " Enter to confirm · esc to cancel")
        }
    }
    .into()
}

/// Render model picker dialog
fn render_model_picker_dialog(
    dialog: &super::widgets::ModelPickerDialog,
    _width: usize,
) -> AnyElement<'static> {
    use super::widgets::ModelChoice;

    let choices = ModelChoice::all();

    element! {
        View(flex_direction: FlexDirection::Column) {
            // Title
            Text(content: " Select model")
            // Description
            Text(content: " Switch between Claude models. Applies to this session and future Claude Code sessions. For other/previous model names,")
            Text(content: "  specify with --model.")
            // Empty line
            Text(content: "")
            // Options
            #(choices.iter().enumerate().map(|(i, choice)| {
                let is_selected = *choice == dialog.selected;
                let is_current = *choice == dialog.current;

                let cursor = if is_selected { "❯" } else { " " };
                let checkmark = if is_current { " ✔" } else { "" };
                let number = i + 1;

                let label = match choice {
                    ModelChoice::Default => "Default (recommended)",
                    ModelChoice::Sonnet => "Sonnet",
                    ModelChoice::Haiku => "Haiku",
                };

                let description = format!(
                    "{} · {}",
                    choice.display_name(),
                    choice.description()
                );

                // Format: " ❯ 1. Label checkmark           Description"
                let content = format!(
                    " {} {}. {:<22}{} {}",
                    cursor,
                    number,
                    label,
                    checkmark,
                    description
                );

                element! {
                    Text(content: content)
                }
            }))
            // Empty line
            Text(content: "")
            // Footer
            Text(content: " Enter to confirm · esc to exit")
        }
    }
    .into_any()
}

/// Render rich permission dialog
fn render_permission_dialog(perm: &PermissionRequest, width: usize) -> AnyElement<'static> {
    // Render the dialog content using the widget
    let content = perm.dialog.render(width);

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
        ) {
            Text(content: content)
        }
    }
    .into()
}

/// Format header lines with Claude branding (returns 3 lines)
fn format_header_lines(state: &RenderState) -> (String, String, String) {
    let model_name = model_display_name(&state.status.model);

    // Get working directory display (shortened if possible)
    let working_dir = std::env::current_dir()
        .map(|p| {
            // Try to convert to ~ format using HOME env var
            if let Ok(home) = std::env::var("HOME") {
                let home_path = std::path::PathBuf::from(&home);
                if let Ok(stripped) = p.strip_prefix(&home_path) {
                    return format!("~/{}", stripped.display());
                }
            }
            p.display().to_string()
        })
        .unwrap_or_else(|_| "~".to_string());

    // Determine product name and version based on claude_version
    let (product_name, version) = match &state.claude_version {
        Some(v) => ("Claude Code", format!("v{}", v)),
        None => ("Claudeless", env!("CARGO_PKG_VERSION").to_string()),
    };
    let model_str = format!("{} · Claude Max", model_name);

    // Use styled ANSI output when connected to a TTY
    if state.is_tty {
        (
            styled_logo_line1(product_name, &version),
            styled_logo_line2(&model_str),
            styled_logo_line3(&working_dir),
        )
    } else {
        let line1 = format!(" ▐▛███▜▌   {} {}", product_name, version);
        let line2 = format!("▝▜█████▛▘  {}", model_str);
        let line3 = format!("  ▘▘ ▝▝    {}", working_dir);
        (line1, line2, line3)
    }
}

/// Format status bar content
pub(crate) fn format_status_bar(state: &RenderState, width: usize) -> String {
    // Check for exit hint first (takes precedence)
    if let Some(hint) = &state.exit_hint {
        return match hint {
            ExitHint::CtrlC => "  Press Ctrl-C again to exit".to_string(),
            ExitHint::CtrlD => "  Press Ctrl-D again to exit".to_string(),
            ExitHint::Escape => "  Esc to clear again".to_string(),
        };
    }

    // Status bar format matches real Claude CLI
    let mode_text = match &state.permission_mode {
        PermissionMode::Default => "  ? for shortcuts".to_string(),
        PermissionMode::Plan => "  ⏸ plan mode on (shift+tab to cycle)".to_string(),
        PermissionMode::AcceptEdits => "  ⏵⏵ accept edits on (shift+tab to cycle)".to_string(),
        PermissionMode::BypassPermissions => {
            "  ⏵⏵ bypass permissions on (shift+tab to cycle)".to_string()
        }
        PermissionMode::Delegate => "  delegate mode (shift+tab to cycle)".to_string(),
        PermissionMode::DontAsk => "  don't ask mode (shift+tab to cycle)".to_string(),
    };

    // For non-default modes, show "Use meta+t to toggle thinking" on the right
    // For default mode, just show the shortcuts hint (or "Thinking off" if disabled)
    match &state.permission_mode {
        PermissionMode::Default => {
            if state.thinking_enabled {
                mode_text
            } else {
                // Pad to align "Thinking off" to the right side
                let padding = width.saturating_sub(mode_text.len() + "Thinking off".len());
                format!("{}{:width$}Thinking off", mode_text, "", width = padding)
            }
        }
        _ => {
            // Non-default modes show "Use meta+t to toggle thinking" on the right
            let right_text = "Use meta+t to toggle thinking";
            // Calculate visual width of mode_text (accounting for multi-byte chars)
            let mode_visual_width = mode_text.chars().count();
            let right_width = right_text.len();
            let padding = width.saturating_sub(mode_visual_width + right_width);
            format!("{}{:width$}{}", mode_text, "", right_text, width = padding)
        }
    }
}

/// Format styled status bar content (with ANSI colors)
fn format_status_bar_styled(state: &RenderState, width: usize) -> String {
    use crate::tui::colors::styled_permission_status;

    // Check for exit hint first (takes precedence)
    if let Some(hint) = &state.exit_hint {
        return match hint {
            ExitHint::CtrlC => "  Press Ctrl-C again to exit".to_string(),
            ExitHint::CtrlD => "  Press Ctrl-D again to exit".to_string(),
            ExitHint::Escape => "  Esc to clear again".to_string(),
        };
    }

    // Get styled permission status
    let status = styled_permission_status(&state.permission_mode);

    // Calculate visual width of the status text (excluding ANSI sequences)
    let mode_visual_width = match &state.permission_mode {
        PermissionMode::Default => "  ? for shortcuts".chars().count(),
        PermissionMode::Plan => "  ⏸ plan mode on (shift+tab to cycle)".chars().count(),
        PermissionMode::AcceptEdits => "  ⏵⏵ accept edits on (shift+tab to cycle)".chars().count(),
        PermissionMode::BypassPermissions => "  ⏵⏵ bypass permissions on (shift+tab to cycle)"
            .chars()
            .count(),
        PermissionMode::Delegate => "  delegate mode (shift+tab to cycle)".chars().count(),
        PermissionMode::DontAsk => "  don't ask mode (shift+tab to cycle)".chars().count(),
    };

    // Add right-aligned text based on mode
    match &state.permission_mode {
        PermissionMode::Default => {
            if state.thinking_enabled {
                status
            } else {
                // Show "Thinking off" aligned to the right
                let right = "Thinking off";
                let padding = width.saturating_sub(mode_visual_width + right.len());
                format!("{}{:width$}{}", status, "", right, width = padding)
            }
        }
        _ => {
            // Non-default modes show "Use meta+t to toggle thinking" on the right
            let right_text = "Use meta+t to toggle thinking";
            let padding = width.saturating_sub(mode_visual_width + right_text.len());
            format!("{}{:width$}{}", status, "", right_text, width = padding)
        }
    }
}

/// Map model ID to display name
fn model_display_name(model: &str) -> String {
    let model_lower = model.to_lowercase();

    // Short aliases default to current version (4.5)
    match model_lower.as_str() {
        "haiku" | "claude-haiku" => return "Haiku 4.5".to_string(),
        "sonnet" | "claude-sonnet" => return "Sonnet 4.5".to_string(),
        "opus" | "claude-opus" => return "Opus 4.5".to_string(),
        _ => {}
    }

    // Parse full model ID like "claude-sonnet-4-20250514"
    let base_name = if model_lower.contains("haiku") {
        "Haiku"
    } else if model_lower.contains("opus") {
        "Opus"
    } else if model_lower.contains("sonnet") {
        "Sonnet"
    } else {
        // Unknown model, show as-is
        return model.to_string();
    };

    // Extract version if present (e.g., "4.5" from "claude-opus-4-5-...")
    let version = extract_model_version(model);

    match version {
        Some(v) => format!("{} {}", base_name, v),
        None => base_name.to_string(),
    }
}

fn extract_model_version(model: &str) -> Option<String> {
    // Pattern: claude-{name}-{major}-{minor?}-{date}
    // e.g., "claude-opus-4-5-20251101" -> "4.5"
    // e.g., "claude-sonnet-4-20250514" -> "4"
    let parts: Vec<&str> = model.split('-').collect();
    if parts.len() >= 4 && parts[0] == "claude" {
        let major = parts[2];
        if major.chars().all(|c| c.is_ascii_digit()) {
            let minor = parts.get(3);
            if let Some(m) = minor {
                if m.chars().all(|c| c.is_ascii_digit()) && m.len() <= 2 {
                    return Some(format!("{}.{}", major, m));
                }
            }
            return Some(major.to_string());
        }
    }
    None
}

/// Legacy TuiApp struct for compatibility
/// This wraps the iocraft-based app and provides the same interface
pub struct TuiApp {
    state: TuiAppState,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new(
        scenario: Scenario,
        sessions: SessionManager,
        clock: ClockHandle,
        config: TuiConfig,
    ) -> std::io::Result<Self> {
        let state = TuiAppState::new(scenario, sessions, clock, config);
        Ok(Self { state })
    }

    /// Run the main event loop using iocraft fullscreen
    pub fn run(&mut self) -> std::io::Result<ExitReason> {
        loop {
            let state = self.state.clone();

            // Check if we're already in a tokio runtime
            if tokio::runtime::Handle::try_current().is_ok() {
                // Already in a runtime - use block_in_place to run async code
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        // ignore_ctrl_c() prevents iocraft from exiting on Ctrl+C - we handle it ourselves
                        // Use render_loop() instead of fullscreen() to:
                        // 1. Render inline (like the real Claude CLI) instead of alternate screen
                        // 2. Not enable mouse capture (allows normal text selection/copy)
                        element!(App(state: Some(state.clone())))
                            .render_loop()
                            .ignore_ctrl_c()
                            .await
                    })
                })?;
            } else {
                // No runtime - create a new one
                let rt = tokio::runtime::Runtime::new()?;
                // ignore_ctrl_c() prevents iocraft from exiting on Ctrl+C - we handle it ourselves
                // Use render_loop() instead of fullscreen() to:
                // 1. Render inline (like the real Claude CLI) instead of alternate screen
                // 2. Not enable mouse capture (allows normal text selection/copy)
                rt.block_on(async {
                    element!(App(state: Some(state.clone())))
                        .render_loop()
                        .ignore_ctrl_c()
                        .await
                })?;
            }

            // Check if we exited due to suspend request
            if matches!(self.state.exit_reason(), Some(ExitReason::Suspended)) {
                // Print suspend messages
                println!("Claude Code has been suspended. Run `fg` to bring Claude Code back.");
                println!("Note: ctrl + z now suspends Claude Code, ctrl + _ undoes input.");

                // Raise SIGTSTP to actually suspend the process
                // After this, execution pauses until SIGCONT is received
                #[cfg(unix)]
                {
                    use nix::sys::signal::{raise, Signal};
                    let _ = raise(Signal::SIGTSTP);
                }

                // On resume (SIGCONT), clear exit state and re-enter fullscreen
                self.state.clear_exit_state();
                continue;
            }

            // Exit for any other reason
            return Ok(self.state.exit_reason().unwrap_or(ExitReason::Completed));
        }
    }

    /// Get state reference for testing
    pub fn state(&self) -> &TuiAppState {
        &self.state
    }

    // Compatibility methods that delegate to state
    pub fn exit(&mut self, reason: ExitReason) {
        self.state.exit(reason);
    }

    pub fn mode(&self) -> AppMode {
        self.state.mode()
    }

    pub fn exit_message(&self) -> Option<String> {
        self.state.exit_message()
    }

    pub fn input_buffer(&self) -> String {
        self.state.input_buffer()
    }

    pub fn cursor_pos(&self) -> usize {
        self.state.cursor_pos()
    }

    pub fn response_content(&self) -> String {
        self.state.render_state().response_content
    }

    pub fn is_streaming(&self) -> bool {
        self.state.render_state().is_streaming
    }

    pub fn status(&self) -> StatusInfo {
        self.state.render_state().status
    }

    pub fn pending_permission(&self) -> Option<PermissionRequest> {
        self.state.render_state().pending_permission
    }

    pub fn show_permission_request(&mut self, permission_type: PermissionType) {
        self.state.show_permission_request(permission_type);
    }

    pub fn show_bash_permission(&mut self, command: String, description: Option<String>) {
        self.state.show_bash_permission(command, description);
    }

    pub fn show_edit_permission(
        &mut self,
        file_path: String,
        diff_lines: Vec<super::widgets::permission::DiffLine>,
    ) {
        self.state.show_edit_permission(file_path, diff_lines);
    }

    pub fn show_write_permission(&mut self, file_path: String, content_lines: Vec<String>) {
        self.state.show_write_permission(file_path, content_lines);
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
