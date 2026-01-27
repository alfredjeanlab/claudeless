// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI application state management.

use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::Arc;

use crate::permission::PermissionMode;
use crate::scenario::Scenario;
use crate::state::session::SessionManager;
use crate::state::todos::{TodoState, TodoStatus};
use crate::time::{Clock, ClockHandle};
use crate::tui::slash_menu::SlashMenuState;
use crate::tui::widgets::context::ContextUsage;
use crate::tui::widgets::export::ExportDialog;
use crate::tui::widgets::help::HelpDialog;
use crate::tui::widgets::permission::{RichPermissionDialog, SessionPermissionKey};
use crate::tui::widgets::tasks::TasksDialog;
use crate::tui::widgets::thinking::ThinkingDialog;
use crate::tui::widgets::{HooksDialog, MemoryDialog, ModelPickerDialog};

use super::types::{
    AppMode, ExitHint, ExitReason, PermissionRequest, RenderState, StatusInfo, TrustPromptState,
    TuiConfig, DEFAULT_TERMINAL_WIDTH, EXIT_HINT_TIMEOUT_MS,
};

/// Shared state for the TUI app that can be accessed from outside the component
#[derive(Clone)]
pub struct TuiAppState {
    pub(super) inner: Arc<Mutex<TuiAppStateInner>>,
}

pub(super) struct TuiAppStateInner {
    /// Current application mode
    pub mode: AppMode,

    /// Input buffer for user typing
    pub input_buffer: String,

    /// Cursor position in input
    pub cursor_pos: usize,

    /// Response content being displayed
    pub response_content: String,

    /// Whether response is currently streaming
    pub is_streaming: bool,

    /// Current status message
    pub status: StatusInfo,

    /// Scenario for response matching
    pub scenario: Arc<Mutex<Scenario>>,

    /// Session manager for conversation state
    pub sessions: Arc<Mutex<SessionManager>>,

    /// Clock for timing
    pub clock: ClockHandle,

    /// Command history
    pub history: Vec<String>,

    /// Current history index
    pub history_index: Option<usize>,

    /// Pending permission request
    pub pending_permission: Option<PermissionRequest>,

    /// Whether app should exit
    pub should_exit: bool,

    /// Exit reason (for testing)
    pub exit_reason: Option<ExitReason>,

    /// Message to display after TUI exits (e.g., farewell from /exit)
    pub exit_message: Option<String>,

    /// Configuration from scenario
    pub config: TuiConfig,

    /// Whether trust has been granted (for untrusted dirs)
    pub trust_granted: bool,

    /// Trust prompt dialog state
    pub trust_prompt: Option<TrustPromptState>,

    /// Whether extended thinking mode is enabled
    pub thinking_enabled: bool,

    /// Thinking toggle dialog state
    pub thinking_dialog: Option<ThinkingDialog>,

    /// Tasks dialog state
    pub tasks_dialog: Option<TasksDialog>,

    /// Model picker dialog state
    pub model_picker_dialog: Option<ModelPickerDialog>,

    /// Export dialog state
    pub export_dialog: Option<ExportDialog>,

    /// Help dialog state
    pub help_dialog: Option<HelpDialog>,

    /// Hooks dialog state
    pub hooks_dialog: Option<HooksDialog>,

    /// Memory dialog state
    pub memory_dialog: Option<MemoryDialog>,

    /// Current permission mode
    pub permission_mode: PermissionMode,

    /// Whether bypass permissions is allowed (requires --dangerously-skip-permissions)
    pub allow_bypass_permissions: bool,

    /// Whether compacting is in progress
    pub is_compacting: bool,

    /// Whether current response is command output (not a Claude response)
    pub is_command_output: bool,

    /// When compacting started (for async completion)
    pub compacting_started: Option<std::time::Instant>,

    /// Visible conversation history (accumulates turns, cleared on /compact)
    pub conversation_display: String,

    /// Whether conversation has been compacted (for showing separator)
    pub is_compacted: bool,

    /// Active exit hint (if any)
    pub exit_hint: Option<ExitHint>,

    /// When exit hint was shown (milliseconds from clock)
    pub exit_hint_shown_at: Option<u64>,

    /// Current terminal width
    pub terminal_width: u16,

    /// Session-level permission grants
    /// Permissions granted with "Yes, allow for session" are stored here
    pub session_grants: HashSet<SessionPermissionKey>,

    /// Whether the shortcuts panel is currently visible
    pub show_shortcuts_panel: bool,

    /// Slash command autocomplete menu state (None if menu is closed)
    pub slash_menu: Option<SlashMenuState>,

    /// Whether shell mode is currently active (user typed '!' at empty input)
    pub shell_mode: bool,

    /// Todo list state
    pub todos: TodoState,

    /// Stack of previous input states for undo (Ctrl+_)
    /// Each entry is a snapshot of input_buffer before a change
    pub undo_stack: Vec<String>,

    /// Stashed input text (Ctrl+S to stash/restore)
    pub stash_buffer: Option<String>,

    /// Whether to show the stash indicator message
    pub show_stash_indicator: bool,
}

impl TuiAppState {
    /// Create a new TUI app state
    pub fn new(
        scenario: Scenario,
        sessions: SessionManager,
        clock: ClockHandle,
        config: TuiConfig,
    ) -> Self {
        // Determine initial mode based on trust state
        let initial_mode = if config.trusted {
            AppMode::Input
        } else {
            AppMode::Trust
        };

        // Create trust prompt if not trusted
        let trust_prompt = if !config.trusted {
            Some(TrustPromptState::new(
                config.working_directory.to_string_lossy().to_string(),
            ))
        } else {
            None
        };

        Self {
            inner: Arc::new(Mutex::new(TuiAppStateInner {
                mode: initial_mode,
                input_buffer: String::new(),
                cursor_pos: 0,
                response_content: String::new(),
                is_streaming: false,
                status: StatusInfo {
                    model: config.model.clone(),
                    ..Default::default()
                },
                scenario: Arc::new(Mutex::new(scenario)),
                sessions: Arc::new(Mutex::new(sessions)),
                clock,
                history: Vec::new(),
                history_index: None,
                pending_permission: None,
                should_exit: false,
                exit_reason: None,
                exit_message: None,
                trust_granted: config.trusted,
                trust_prompt,
                thinking_enabled: true, // Default to enabled
                thinking_dialog: None,
                tasks_dialog: None,
                model_picker_dialog: None,
                export_dialog: None,
                help_dialog: None,
                hooks_dialog: None,
                memory_dialog: None,
                permission_mode: config.permission_mode.clone(),
                allow_bypass_permissions: config.allow_bypass_permissions,
                is_compacting: false,
                is_command_output: false,
                compacting_started: None,
                conversation_display: String::new(),
                is_compacted: false,
                exit_hint: None,
                exit_hint_shown_at: None,
                terminal_width: crossterm::terminal::size()
                    .map(|(w, _)| w)
                    .unwrap_or(DEFAULT_TERMINAL_WIDTH),
                config,
                session_grants: HashSet::new(),
                show_shortcuts_panel: false,
                slash_menu: None,
                shell_mode: false,
                todos: TodoState::new(),
                undo_stack: Vec::new(),
                stash_buffer: None,
                show_stash_indicator: false,
            })),
        }
    }

    /// Get the render state snapshot
    pub fn render_state(&self) -> RenderState {
        let inner = self.inner.lock();
        RenderState {
            mode: inner.mode.clone(),
            input_buffer: inner.input_buffer.clone(),
            cursor_pos: inner.cursor_pos,
            response_content: inner.response_content.clone(),
            is_streaming: inner.is_streaming,
            status: inner.status.clone(),
            pending_permission: inner.pending_permission.clone(),
            user_name: inner.config.user_name.clone(),
            trust_prompt: inner.trust_prompt.clone(),
            thinking_dialog: inner.thinking_dialog.clone(),
            tasks_dialog: inner.tasks_dialog.clone(),
            model_picker_dialog: inner.model_picker_dialog.clone(),
            thinking_enabled: inner.thinking_enabled,
            permission_mode: inner.permission_mode.clone(),
            is_command_output: inner.is_command_output,
            conversation_display: inner.conversation_display.clone(),
            is_compacted: inner.is_compacted,
            exit_hint: inner.exit_hint.clone(),
            claude_version: inner.config.claude_version.clone(),
            terminal_width: inner.terminal_width,
            show_shortcuts_panel: inner.show_shortcuts_panel,
            slash_menu: inner.slash_menu.clone(),
            shell_mode: inner.shell_mode,
            is_tty: inner.config.is_tty,
            export_dialog: inner.export_dialog.clone(),
            help_dialog: inner.help_dialog.clone(),
            hooks_dialog: inner.hooks_dialog.clone(),
            memory_dialog: inner.memory_dialog.clone(),
            stash_buffer: inner.stash_buffer.clone(),
            show_stash_indicator: inner.show_stash_indicator,
        }
    }

    /// Get terminal width
    pub fn terminal_width(&self) -> u16 {
        self.inner.lock().terminal_width
    }

    /// Update terminal width (called on resize)
    pub fn set_terminal_width(&self, width: u16) {
        self.inner.lock().terminal_width = width;
    }

    /// Check if app should exit
    pub fn should_exit(&self) -> bool {
        self.inner.lock().should_exit
    }

    /// Get exit reason
    pub fn exit_reason(&self) -> Option<ExitReason> {
        self.inner.lock().exit_reason.clone()
    }

    /// Get exit message (e.g., farewell from /exit)
    pub fn exit_message(&self) -> Option<String> {
        self.inner.lock().exit_message.clone()
    }

    /// Get current mode
    pub fn mode(&self) -> AppMode {
        self.inner.lock().mode.clone()
    }

    /// Get input buffer
    pub fn input_buffer(&self) -> String {
        self.inner.lock().input_buffer.clone()
    }

    /// Get cursor position
    pub fn cursor_pos(&self) -> usize {
        self.inner.lock().cursor_pos
    }

    /// Get history
    pub fn history(&self) -> Vec<String> {
        self.inner.lock().history.clone()
    }

    /// Request app exit
    pub fn exit(&self, reason: ExitReason) {
        let mut inner = self.inner.lock();
        inner.should_exit = true;
        inner.exit_reason = Some(reason);
    }

    /// Clear exit state to allow re-entry (used after suspend/resume)
    pub fn clear_exit_state(&self) {
        let mut inner = self.inner.lock();
        inner.should_exit = false;
        inner.exit_reason = None;
    }

    /// Check if exit hint has timed out and clear it
    pub fn check_exit_hint_timeout(&self) {
        let mut inner = self.inner.lock();
        if let (Some(_hint), Some(shown_at)) = (&inner.exit_hint, inner.exit_hint_shown_at) {
            let now = inner.clock.now_millis();
            if now.saturating_sub(shown_at) >= EXIT_HINT_TIMEOUT_MS {
                inner.exit_hint = None;
                inner.exit_hint_shown_at = None;
            }
        }
    }

    /// Check for async compacting completion
    pub fn check_compacting(&self) {
        let mut inner = self.inner.lock();
        if inner.is_compacting {
            if let Some(started) = inner.compacting_started {
                let delay_ms = inner.config.compact_delay_ms.unwrap_or(500);
                if started.elapsed() >= std::time::Duration::from_millis(delay_ms) {
                    inner.is_compacting = false;
                    inner.compacting_started = None;
                    inner.mode = AppMode::Input;
                    inner.is_compacted = true;

                    // Build tool summary from session turns
                    let tool_summary = build_tool_summary(&inner.sessions);

                    // Set response with elbow connector format
                    inner.response_content = format!(
                        "Compacted (ctrl+o to see full summary){}",
                        if tool_summary.is_empty() {
                            String::new()
                        } else {
                            format!("\n{}", tool_summary)
                        }
                    );
                    inner.is_command_output = true;

                    // Set conversation display to show the /compact command
                    inner.conversation_display = "❯ /compact".to_string();
                }
            }
        }
    }

    /// Format todo items for display.
    pub(super) fn format_todos(todos: &TodoState) -> String {
        if todos.is_empty() {
            "No todos currently tracked".to_string()
        } else {
            todos
                .items
                .iter()
                .map(|item| {
                    let status = match item.status {
                        TodoStatus::Pending => "[ ]",
                        TodoStatus::InProgress => "[*]",
                        TodoStatus::Completed => "[x]",
                    };
                    format!("{} {}", status, item.content)
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    /// Generate a random farewell message for /exit command
    pub(super) fn random_farewell() -> &'static str {
        const FAREWELLS: &[&str] = &["Goodbye!", "Bye!", "See ya!", "Catch you later!"];
        let idx = fastrand::usize(..FAREWELLS.len());
        FAREWELLS[idx]
    }

    /// Format context usage as a grid display
    pub(super) fn format_context_usage(usage: &ContextUsage) -> String {
        let cells = usage.grid_cells();
        let mut lines = Vec::new();

        // Build grid rows (10 cells per row, 9 rows)
        for row in 0..9 {
            let start = row * 10;
            let end = start + 10;
            let row_cells: String = cells[start..end]
                .iter()
                .map(|c| format!("{} ", c))
                .collect::<String>()
                .trim_end()
                .to_string();

            // Rows have category labels on the right
            let label = match row {
                1 => "  Estimated usage by category".to_string(),
                2 => format!(
                    "  ⛁ System prompt: {} tokens ({:.1}%)",
                    ContextUsage::format_tokens(usage.system_prompt_tokens),
                    usage.percentage(usage.system_prompt_tokens)
                ),
                3 => format!(
                    "  ⛁ System tools: {} tokens ({:.1}%)",
                    ContextUsage::format_tokens(usage.system_tools_tokens),
                    usage.percentage(usage.system_tools_tokens)
                ),
                4 => format!(
                    "  ⛁ Memory files: {} tokens ({:.1}%)",
                    ContextUsage::format_tokens(usage.memory_files_tokens),
                    usage.percentage(usage.memory_files_tokens)
                ),
                5 => format!(
                    "  ⛁ Messages: {} tokens ({:.1}%)",
                    ContextUsage::format_tokens(usage.messages_tokens),
                    usage.percentage(usage.messages_tokens)
                ),
                6 => format!(
                    "  ⛶ Free space: {} ({:.1}%)",
                    ContextUsage::format_tokens(usage.free_space_tokens),
                    usage.percentage(usage.free_space_tokens)
                ),
                7 => format!(
                    "  ⛝ Autocompact buffer: {} tokens ({:.1}%)",
                    ContextUsage::format_tokens(usage.autocompact_buffer_tokens),
                    usage.percentage(usage.autocompact_buffer_tokens)
                ),
                _ => String::new(),
            };

            lines.push(format!("     {}   {}", row_cells, label));
        }

        // Add memory files section
        lines.push(String::new());
        lines.push("     Memory files · /memory".to_string());
        for file in &usage.memory_files {
            lines.push(format!("     └ {}: {} tokens", file.path, file.tokens));
        }

        lines.join("\n")
    }

    /// Check if a permission is already granted for this session
    pub(super) fn is_session_granted(
        &self,
        permission_type: &crate::tui::widgets::permission::PermissionType,
    ) -> bool {
        let inner = self.inner.lock();
        let dialog = RichPermissionDialog::new(permission_type.clone());
        let key = dialog.session_key();
        inner.session_grants.contains(&key)
    }
}

/// Build tool summary from session turns for /compact output
pub(super) fn build_tool_summary(sessions: &Arc<Mutex<SessionManager>>) -> String {
    let sessions = sessions.lock();
    let Some(session) = sessions.get_current() else {
        return String::new();
    };

    let mut summaries = Vec::new();
    for turn in &session.turns {
        for tool_call in &turn.tool_calls {
            if let Some(summary) = format_tool_summary(tool_call) {
                summaries.push(summary);
            }
        }
    }
    summaries.join("\n")
}

/// Format a single tool call for the compact summary
pub(super) fn format_tool_summary(tool: &crate::state::session::TurnToolCall) -> Option<String> {
    match tool.tool.as_str() {
        "Read" => {
            let path = tool.input.get("file_path")?.as_str()?;
            // Extract just the filename from the path
            let filename = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            let lines = tool.output.as_ref().map(|o| o.lines().count()).unwrap_or(0);
            Some(format!("Read {} ({} lines)", filename, lines))
        }
        "Write" => {
            let path = tool.input.get("file_path")?.as_str()?;
            let filename = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            Some(format!("Wrote {}", filename))
        }
        "Edit" => {
            let path = tool.input.get("file_path")?.as_str()?;
            let filename = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            Some(format!("Edited {}", filename))
        }
        "Bash" => {
            let cmd = tool.input.get("command")?.as_str()?;
            let short_cmd = if cmd.len() > 30 {
                format!("{}...", &cmd[..27])
            } else {
                cmd.to_string()
            };
            Some(format!("Ran `{}`", short_cmd))
        }
        _ => None,
    }
}
