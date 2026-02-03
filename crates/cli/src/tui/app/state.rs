// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI application state management.

#[path = "display_state.rs"]
mod display_state;
#[path = "input_state.rs"]
mod input_state;

pub use super::dialogs::DialogState;
pub use display_state::DisplayState;
pub use input_state::InputState;

use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::Arc;

use crate::permission::PermissionMode;
use crate::runtime::Runtime;
use crate::state::session::SessionManager;
use crate::state::todos::{TodoState, TodoStatus};
use crate::time::{Clock, ClockHandle};
use crate::tui::widgets::context::ContextUsage;
use crate::tui::widgets::permission::{RichPermissionDialog, SessionPermissionKey};

use crate::tui::widgets::setup::SetupState;

use super::types::{
    AppMode, BypassChoice, BypassConfirmState, ExitReason, RenderState, StatusInfo,
    TrustPromptState, TuiConfig,
};

/// Shared state for the TUI app that can be accessed from outside the component
#[derive(Clone)]
pub struct TuiAppState {
    pub(super) inner: Arc<Mutex<TuiAppStateInner>>,
}

pub(super) struct TuiAppStateInner {
    // Focused state groups
    /// Input editing state
    pub input: InputState,
    /// Active dialog state
    pub dialog: DialogState,
    /// Display/rendering state
    pub display: DisplayState,

    // Core dependencies
    /// Runtime for shared execution (required for TUI mode)
    pub runtime: Option<Runtime>,
    /// Session manager for conversation state
    pub sessions: SessionManager,
    /// Clock for timing
    pub clock: ClockHandle,
    /// Configuration
    pub config: TuiConfig,

    // Session state
    /// Current application mode
    pub mode: AppMode,
    /// Current status message
    pub status: StatusInfo,
    /// Current permission mode
    pub permission_mode: PermissionMode,
    /// Session-level permission grants
    pub session_grants: HashSet<SessionPermissionKey>,
    /// Whether trust has been granted (for untrusted dirs)
    pub trust_granted: bool,
    /// Whether extended thinking mode is enabled
    pub thinking_enabled: bool,
    /// Whether bypass permissions is allowed (requires --dangerously-skip-permissions)
    pub allow_bypass_permissions: bool,

    // Exit state
    /// Whether app should exit
    pub should_exit: bool,
    /// Exit reason (for testing)
    pub exit_reason: Option<ExitReason>,
    /// Message to display after TUI exits (e.g., farewell from /exit)
    pub exit_message: Option<String>,

    // Compacting state
    /// Whether compacting is in progress
    pub is_compacting: bool,
    /// When compacting started (for async completion)
    pub compacting_started: Option<std::time::Instant>,

    // Data
    /// Todo list state
    pub todos: TodoState,

    // Stop hook state
    /// Whether Claude Code is continuing as a result of a stop hook
    pub stop_hook_active: bool,
    /// Pending message from stop hook to inject as next prompt
    pub pending_hook_message: Option<String>,

    // Session start hook state
    /// Whether the session start hook has been fired
    pub session_start_hook_fired: bool,

    // Initial prompt state
    /// Pending initial prompt from CLI positional arg (processed once on startup)
    pub pending_initial_prompt: Option<String>,
}

impl TuiAppStateInner {
    /// Mark the app for exit with the given reason.
    pub fn exit(&mut self, reason: ExitReason) {
        self.should_exit = true;
        self.exit_reason = Some(reason);
    }

    /// Dismiss any active dialog and return to input mode.
    pub fn dismiss_dialog(&mut self, name: &str) {
        self.mode = AppMode::Input;
        self.dialog.dismiss();
        self.display.response_content = format!("{} dismissed", name);
        self.display.is_command_output = true;
    }
}

impl TuiAppState {
    /// Create a new TUI app state for testing (no runtime).
    #[doc(hidden)]
    pub fn for_test(sessions: SessionManager, clock: ClockHandle, config: TuiConfig) -> Self {
        Self::new(sessions, clock, config, None)
    }

    /// Create a new TUI app state with a Runtime for shared execution
    pub fn new(
        sessions: SessionManager,
        clock: ClockHandle,
        config: TuiConfig,
        runtime: Option<Runtime>,
    ) -> Self {
        // Determine initial mode and dialog based on state
        let (initial_mode, dialog) = if config.bypass_confirmation_needed {
            // Bypass confirmation takes priority
            (
                AppMode::BypassConfirm,
                DialogState::BypassConfirm(BypassConfirmState {
                    selected: BypassChoice::No,
                }),
            )
        } else if !config.trusted {
            (
                AppMode::Trust,
                DialogState::Trust(TrustPromptState::new(
                    config.working_directory.to_string_lossy().to_string(),
                )),
            )
        } else if !config.logged_in {
            let version = config
                .clone()
                .claude_version
                .unwrap_or_else(|| crate::config::DEFAULT_CLAUDE_VERSION.to_string());
            (AppMode::Setup, DialogState::Setup(SetupState::new(version)))
        } else {
            (AppMode::Input, DialogState::None)
        };

        Self {
            inner: Arc::new(Mutex::new(TuiAppStateInner {
                // Focused state groups
                input: InputState::default(),
                dialog,
                display: DisplayState::new(),

                // Core dependencies
                runtime,
                sessions,
                clock,

                // Session state
                mode: initial_mode,
                status: StatusInfo {
                    model: config.model.clone(),
                    ..Default::default()
                },
                permission_mode: config.permission_mode.clone(),
                session_grants: HashSet::new(),
                trust_granted: config.trusted,
                thinking_enabled: true, // Default to enabled
                allow_bypass_permissions: config.allow_bypass_permissions,

                // Exit state
                should_exit: false,
                exit_reason: None,
                exit_message: None,

                // Compacting state
                is_compacting: false,
                compacting_started: None,

                // Data
                todos: TodoState::new(),

                // Stop hook state
                stop_hook_active: false,
                pending_hook_message: None,

                // Session start hook state
                session_start_hook_fired: false,

                // Initial prompt state
                pending_initial_prompt: config.initial_prompt.clone(),

                // Config (must be last as it's moved)
                config,
            })),
        }
    }

    /// Get the render state snapshot
    pub fn render_state(&self) -> RenderState {
        let inner = self.inner.lock();
        RenderState {
            mode: inner.mode.clone(),
            input: inner.input.clone(),
            dialog: inner.dialog.clone(),
            display: inner.display.clone(),
            status: inner.status.clone(),
            permission_mode: inner.permission_mode.clone(),
            thinking_enabled: inner.thinking_enabled,
            user_name: inner.config.user_name.clone(),
            claude_version: inner.config.claude_version.clone(),
            is_tty: inner.config.is_tty,
            is_compacting: inner.is_compacting,
            spinner_frame: inner.display.spinner_frame,
            spinner_verb: inner.display.spinner_verb.clone(),
            placeholder: inner.config.placeholder.clone(),
            provider: inner.config.provider.clone(),
            show_welcome_back: inner.config.show_welcome_back,
            welcome_back_right_panel: inner.config.welcome_back_right_panel.clone(),
        }
    }

    /// Advance the spinner animation frame
    pub fn advance_spinner(&self) {
        use crate::tui::spinner;
        let mut inner = self.inner.lock();
        let cycle_len = spinner::spinner_cycle().len();
        inner.display.spinner_frame = (inner.display.spinner_frame + 1) % cycle_len;
    }

    /// Get terminal width
    pub fn terminal_width(&self) -> u16 {
        self.inner.lock().display.terminal_width
    }

    /// Update terminal width (called on resize)
    pub fn set_terminal_width(&self, width: u16) {
        self.inner.lock().display.terminal_width = width;
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

    /// Get a clone of the clock handle (useful for tests to advance fake time)
    pub fn clock(&self) -> ClockHandle {
        self.inner.lock().clock.clone()
    }

    /// Get current mode
    pub fn mode(&self) -> AppMode {
        self.inner.lock().mode.clone()
    }

    /// Get input buffer
    pub fn input_buffer(&self) -> String {
        self.inner.lock().input.buffer.clone()
    }

    /// Get cursor position
    pub fn cursor_pos(&self) -> usize {
        self.inner.lock().input.cursor_pos
    }

    /// Get history
    pub fn history(&self) -> Vec<String> {
        self.inner.lock().input.history.clone()
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
        if let (Some(_hint), Some(shown_at)) =
            (&inner.display.exit_hint, inner.display.exit_hint_shown_at)
        {
            let now = inner.clock.now_millis();
            let exit_hint_timeout = inner.config.timeouts.exit_hint_ms;
            if now.saturating_sub(shown_at) >= exit_hint_timeout {
                inner.display.clear_exit_hint();
            }
        }
    }

    /// Check for async compacting completion
    pub fn check_compacting(&self) {
        let mut inner = self.inner.lock();
        if inner.is_compacting {
            if let Some(started) = inner.compacting_started {
                let delay_ms = inner.config.timeouts.compact_delay_ms;
                if started.elapsed() >= std::time::Duration::from_millis(delay_ms) {
                    // Keep is_compacting true for status bar hint;
                    // content renderer uses is_compacted to distinguish phases
                    inner.compacting_started = None;
                    inner.mode = AppMode::Input;
                    inner.display.is_compacted = true;

                    // Build tool summary from session turns
                    let tool_summary = build_tool_summary(&inner.sessions);

                    // Build command output text
                    let cmd_output = format!(
                        "Compacted (ctrl+o to see full summary){}",
                        if tool_summary.is_empty() {
                            String::new()
                        } else {
                            format!("\n{}", tool_summary)
                        }
                    );

                    // Accumulate command output into conversation display
                    inner.display.conversation_display =
                        format!("❯ /compact\n  \u{23BF}  {}", cmd_output);

                    inner.display.response_content.clear();
                    inner.display.is_command_output = true;
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

        // First line: just the title (content renderer adds ⎿ prefix)
        lines.push("Context Usage".to_string());

        // Build grid rows (10 cells per row, 10 rows)
        for row in 0..10 {
            let start = row * 10;
            let end = start + 10;
            let row_cells: String = cells[start..end]
                .iter()
                .map(|c| format!("{} ", c))
                .collect::<String>()
                .trim_end()
                .to_string();

            // Labels on the right side of certain rows
            let label = match row {
                0 => format!(
                    "{} \u{00b7} {}/{} tokens ({:.0}%)",
                    usage.model_name,
                    ContextUsage::format_tokens_short(usage.total_used()),
                    ContextUsage::format_tokens_short(usage.total_tokens),
                    usage.used_percentage()
                ),
                2 => "Estimated usage by category".to_string(),
                3 => format!(
                    "\u{26C1} System prompt: {} tokens ({:.1}%)",
                    ContextUsage::format_tokens(usage.system_prompt_tokens),
                    usage.percentage(usage.system_prompt_tokens)
                ),
                4 => format!(
                    "\u{26C1} System tools: {} tokens ({:.1}%)",
                    ContextUsage::format_tokens(usage.system_tools_tokens),
                    usage.percentage(usage.system_tools_tokens)
                ),
                5 => format!(
                    "\u{26C1} Messages: {} tokens ({:.1}%)",
                    ContextUsage::format_tokens(usage.messages_tokens),
                    usage.percentage(usage.messages_tokens)
                ),
                6 => format!(
                    "\u{26F6} Free space: {} ({:.1}%)",
                    ContextUsage::format_tokens(usage.free_space_tokens),
                    usage.percentage(usage.free_space_tokens)
                ),
                7 => format!(
                    "\u{26DD} Autocompact buffer: {} tokens ({:.1}%)",
                    ContextUsage::format_tokens(usage.autocompact_buffer_tokens),
                    usage.percentage(usage.autocompact_buffer_tokens)
                ),
                _ => String::new(),
            };

            if label.is_empty() {
                lines.push(format!("     {}", row_cells));
            } else {
                lines.push(format!("     {}   {}", row_cells, label));
            }
        }

        lines.join("\n")
    }

    /// Check if the session start hook needs to fire.
    ///
    /// Fires once when the app enters Input mode for the first time.
    pub fn check_session_start_hook(&self) {
        let should_fire = {
            let inner = self.inner.lock();
            if inner.session_start_hook_fired {
                return;
            }
            if inner.mode != AppMode::Input {
                return;
            }
            inner.runtime.is_some()
        };

        if !should_fire {
            return;
        }

        // Take runtime, fire hook, put it back
        let runtime = {
            let mut inner = self.inner.lock();
            inner.session_start_hook_fired = true;
            inner.runtime.take()
        };

        if let Some(rt) = runtime {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(rt.fire_session_start_hook());
            });
            let mut inner = self.inner.lock();
            inner.runtime = Some(rt);
        }
    }

    /// Check for pending stop hook message and process it.
    ///
    /// Returns true if a hook message was processed.
    pub fn check_pending_hook_message(&self) -> bool {
        let pending = {
            let mut inner = self.inner.lock();
            inner.pending_hook_message.take()
        };

        if let Some(hook_msg) = pending {
            // Process the hook message as a new prompt
            self.process_prompt(hook_msg);
            true
        } else {
            false
        }
    }

    /// Check for and process pending initial prompt.
    ///
    /// Returns true if an initial prompt was processed.
    /// Only processes if in Input mode (not during trust dialog).
    pub fn check_initial_prompt(&self) -> bool {
        let pending = {
            let mut inner = self.inner.lock();
            // Only process if in Input mode (not during trust dialog)
            if inner.mode != AppMode::Input {
                return false;
            }
            inner.pending_initial_prompt.take()
        };

        if let Some(prompt) = pending {
            self.process_prompt(prompt);
            true
        } else {
            false
        }
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

    /// Take ownership of the runtime (for shutdown).
    ///
    /// This removes the runtime from the state, allowing the caller to
    /// call shutdown methods on it.
    pub fn take_runtime(&self) -> Option<Runtime> {
        let mut inner = self.inner.lock();
        inner.runtime.take()
    }
}

/// Build tool summary from session turns for /compact output
pub(super) fn build_tool_summary(sessions: &SessionManager) -> String {
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
