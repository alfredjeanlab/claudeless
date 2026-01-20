// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI application state and main iocraft component.

use iocraft::prelude::*;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::{ScenarioConfig, DEFAULT_MODEL, DEFAULT_USER_NAME};
use crate::permission::PermissionMode;
use crate::scenario::Scenario;
use crate::state::session::SessionManager;
use crate::time::{Clock, ClockHandle};

use super::streaming::{StreamingConfig, StreamingResponse};
use super::widgets::permission::{PermissionSelection, PermissionType, RichPermissionDialog};
use super::widgets::thinking::{ThinkingDialog, ThinkingMode};
use super::widgets::trust::TrustChoice;

/// Configuration from scenario for TUI behavior
#[derive(Clone, Debug)]
pub struct TuiConfig {
    pub trusted: bool,
    pub user_name: String,
    pub model: String,
    pub working_directory: PathBuf,
    pub permission_mode: PermissionMode,
    /// Whether bypass permissions mode is allowed (requires --dangerously-skip-permissions)
    pub allow_bypass_permissions: bool,
    /// Delay in milliseconds before compact completes (default: 500)
    pub compact_delay_ms: Option<u64>,
    /// Explicit Claude version, or None for Claudeless-native mode
    pub claude_version: Option<String>,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            trusted: true,
            user_name: DEFAULT_USER_NAME.to_string(),
            model: DEFAULT_MODEL.to_string(),
            working_directory: std::env::current_dir().unwrap_or_default(),
            permission_mode: PermissionMode::Default,
            allow_bypass_permissions: false,
            compact_delay_ms: None,
            claude_version: None,
        }
    }
}

impl TuiConfig {
    pub fn from_scenario(
        config: &ScenarioConfig,
        cli_model: Option<&str>,
        cli_permission_mode: &PermissionMode,
        allow_bypass_permissions: bool,
        cli_claude_version: Option<&str>,
    ) -> Self {
        // CLI permission mode overrides scenario (unless CLI is default)
        let permission_mode = if *cli_permission_mode != PermissionMode::Default {
            cli_permission_mode.clone()
        } else {
            config
                .permission_mode
                .as_deref()
                .and_then(|s| clap::ValueEnum::from_str(s, true).ok())
                .unwrap_or_default()
        };

        // CLI claude_version overrides scenario
        let claude_version = cli_claude_version
            .map(|s| s.to_string())
            .or_else(|| config.claude_version.clone());

        Self {
            trusted: config.trusted,
            user_name: config
                .user_name
                .clone()
                .unwrap_or_else(|| DEFAULT_USER_NAME.to_string()),
            model: cli_model
                .map(|s| s.to_string())
                .or_else(|| config.default_model.clone())
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            working_directory: config
                .working_directory
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default()),
            permission_mode,
            allow_bypass_permissions,
            compact_delay_ms: config.compact_delay_ms,
            claude_version,
        }
    }
}

/// Application mode
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppMode {
    /// Waiting for user input
    Input,
    /// Processing/streaming response
    Responding,
    /// Showing permission prompt
    Permission,
    /// Thinking indicator
    Thinking,
    /// Showing trust prompt dialog
    Trust,
    /// Showing thinking toggle dialog
    ThinkingToggle,
}

/// Status bar information
#[derive(Clone, Debug, Default)]
pub struct StatusInfo {
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub session_id: Option<String>,
}

/// Snapshot of app state for rendering (avoids borrow issues)
#[derive(Clone, Debug)]
pub struct RenderState {
    pub mode: AppMode,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub response_content: String,
    pub is_streaming: bool,
    pub status: StatusInfo,
    pub pending_permission: Option<PermissionRequest>,
    pub user_name: String,
    pub trust_prompt: Option<TrustPromptState>,
    pub thinking_dialog: Option<ThinkingDialog>,
    pub thinking_enabled: bool,
    pub permission_mode: PermissionMode,
    pub is_command_output: bool,
    pub conversation_display: String,
    pub is_compacted: bool,
    pub exit_hint: Option<ExitHint>,
    /// Explicit Claude version, or None for Claudeless-native mode
    pub claude_version: Option<String>,
}

/// Permission request state using the rich permission dialog
#[derive(Clone, Debug)]
pub struct PermissionRequest {
    pub dialog: RichPermissionDialog,
}

/// Legacy permission choice (for compatibility)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionChoice {
    Allow,
    AllowSession,
    Deny,
}

impl From<PermissionSelection> for PermissionChoice {
    fn from(selection: PermissionSelection) -> Self {
        match selection {
            PermissionSelection::Yes => PermissionChoice::Allow,
            PermissionSelection::YesSession => PermissionChoice::AllowSession,
            PermissionSelection::No => PermissionChoice::Deny,
        }
    }
}

/// Reason for app exit
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExitReason {
    UserQuit,      // Ctrl+D or explicit quit
    Interrupted,   // Ctrl+C
    Completed,     // Normal completion
    Error(String), // Error occurred
}

/// Type of exit hint being shown
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExitHint {
    /// "Press Ctrl-C again to exit"
    CtrlC,
    /// "Press Ctrl-D again to exit"
    CtrlD,
}

/// Exit hint timeout in milliseconds (2 seconds)
const EXIT_HINT_TIMEOUT_MS: u64 = 2000;

/// Trust prompt state (simplified for iocraft)
#[derive(Clone, Debug)]
pub struct TrustPromptState {
    pub working_directory: String,
    pub selected: TrustChoice,
}

impl TrustPromptState {
    pub fn new(working_directory: String) -> Self {
        Self {
            working_directory,
            selected: TrustChoice::Yes,
        }
    }
}

/// Shared state for the TUI app that can be accessed from outside the component
#[derive(Clone)]
pub struct TuiAppState {
    inner: Arc<Mutex<TuiAppStateInner>>,
}

struct TuiAppStateInner {
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
                trust_granted: config.trusted,
                trust_prompt,
                thinking_enabled: true, // Default to enabled
                thinking_dialog: None,
                permission_mode: config.permission_mode.clone(),
                allow_bypass_permissions: config.allow_bypass_permissions,
                is_compacting: false,
                is_command_output: false,
                compacting_started: None,
                conversation_display: String::new(),
                is_compacted: false,
                exit_hint: None,
                exit_hint_shown_at: None,
                config,
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
            thinking_enabled: inner.thinking_enabled,
            permission_mode: inner.permission_mode.clone(),
            is_command_output: inner.is_command_output,
            conversation_display: inner.conversation_display.clone(),
            is_compacted: inner.is_compacted,
            exit_hint: inner.exit_hint.clone(),
            claude_version: inner.config.claude_version.clone(),
        }
    }

    /// Check if app should exit
    pub fn should_exit(&self) -> bool {
        self.inner.lock().should_exit
    }

    /// Get exit reason
    pub fn exit_reason(&self) -> Option<ExitReason> {
        self.inner.lock().exit_reason.clone()
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

    /// Handle key event based on current mode
    pub fn handle_key_event(&self, key: KeyEvent) {
        let mode = self.mode();
        match mode {
            AppMode::Trust => self.handle_trust_key(key),
            AppMode::Input => self.handle_input_key(key),
            AppMode::Permission => self.handle_permission_key(key),
            AppMode::Responding | AppMode::Thinking => self.handle_responding_key(key),
            AppMode::ThinkingToggle => self.handle_thinking_key(key),
        }
    }

    /// Handle key events in input mode
    fn handle_input_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();
        match (key.modifiers, key.code) {
            // Ctrl+C - Interrupt
            (m, KeyCode::Char('c')) if m.contains(KeyModifiers::CONTROL) => {
                drop(inner);
                self.handle_interrupt();
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
            (m, KeyCode::Char('t')) if m.contains(KeyModifiers::ALT) => {
                inner.thinking_dialog = Some(ThinkingDialog::new(inner.thinking_enabled));
                inner.mode = AppMode::ThinkingToggle;
            }

            // Shift+Tab - Cycle permission mode
            (m, KeyCode::BackTab) if m.contains(KeyModifiers::SHIFT) => {
                inner.permission_mode = inner
                    .permission_mode
                    .cycle_next(inner.allow_bypass_permissions);
            }

            // Enter - Submit input
            (_, KeyCode::Enter) => {
                // Clear exit hint on Enter
                inner.exit_hint = None;
                inner.exit_hint_shown_at = None;
                if !inner.input_buffer.is_empty() {
                    drop(inner);
                    self.submit_input();
                }
            }

            // Escape - Cancel current input
            (_, KeyCode::Esc) => {
                inner.input_buffer.clear();
                inner.cursor_pos = 0;
            }

            // Backspace - Delete character before cursor
            (_, KeyCode::Backspace) => {
                if inner.cursor_pos > 0 {
                    let pos = inner.cursor_pos - 1;
                    inner.cursor_pos = pos;
                    inner.input_buffer.remove(pos);
                }
            }

            // Delete - Delete character at cursor
            (_, KeyCode::Delete) => {
                let pos = inner.cursor_pos;
                if pos < inner.input_buffer.len() {
                    inner.input_buffer.remove(pos);
                }
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

            // Up arrow - Previous history
            (_, KeyCode::Up) => {
                Self::navigate_history_inner(&mut inner, -1);
            }

            // Down arrow - Next history
            (_, KeyCode::Down) => {
                Self::navigate_history_inner(&mut inner, 1);
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
            }

            // Ctrl+K - Clear line after cursor
            (m, KeyCode::Char('k')) if m.contains(KeyModifiers::CONTROL) => {
                let pos = inner.cursor_pos;
                inner.input_buffer.truncate(pos);
            }

            // Ctrl+W - Delete word before cursor
            (m, KeyCode::Char('w')) if m.contains(KeyModifiers::CONTROL) => {
                Self::delete_word_before_cursor_inner(&mut inner);
            }

            // Regular character input
            (m, KeyCode::Char(c)) if m.is_empty() || m == KeyModifiers::SHIFT => {
                let pos = inner.cursor_pos;
                inner.input_buffer.insert(pos, c);
                inner.cursor_pos = pos + 1;
                // Reset history browsing on new input
                inner.history_index = None;
                // Clear exit hint on typing
                inner.exit_hint = None;
                inner.exit_hint_shown_at = None;
            }

            _ => {}
        }
    }

    /// Handle key events in responding/thinking mode
    fn handle_responding_key(&self, key: KeyEvent) {
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

    /// Handle Ctrl+C interrupt
    fn handle_interrupt(&self) {
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
        }
    }

    /// Navigate through command history
    fn navigate_history_inner(inner: &mut TuiAppStateInner, direction: i32) {
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
                return;
            }
            Some(i) => Some(i),
        };

        if let Some(idx) = new_index {
            inner.history_index = Some(idx);
            inner.input_buffer = inner.history[idx].clone();
            inner.cursor_pos = inner.input_buffer.len();
        }
    }

    /// Delete word before cursor (Ctrl+W behavior)
    fn delete_word_before_cursor_inner(inner: &mut TuiAppStateInner) {
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

    /// Submit the current input
    fn submit_input(&self) {
        let mut inner = self.inner.lock();
        let input = std::mem::take(&mut inner.input_buffer);
        inner.cursor_pos = 0;

        // Add to history
        if !input.is_empty() {
            inner.history.push(input.clone());
        }
        inner.history_index = None;

        // Check for slash commands
        if input.starts_with('/') {
            Self::handle_command_inner(&mut inner, &input);
        } else {
            // Process the input as a prompt
            drop(inner);
            self.process_prompt(input);
        }
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

                // Set response content (will be rendered with elbow connector)
                inner.response_content = "(no content)".to_string();
            }
            "/compact" => {
                // Show compacting in progress message
                inner.mode = AppMode::Responding;
                inner.is_compacting = true;
                inner.compacting_started = Some(std::time::Instant::now());
                // Use correct symbol (✻) and ellipsis (…)
                inner.response_content =
                    "✻ Compacting conversation… (ctrl+c to interrupt)".to_string();
            }
            "/help" | "/?" => {
                inner.response_content = "Available commands:\n\
                    /clear   - Clear conversation history\n\
                    /compact - Compact conversation history\n\
                    /help    - Show this help\n"
                    .to_string();
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

    /// Show a permission request with rich dialog
    pub fn show_permission_request(&self, permission_type: PermissionType) {
        let mut inner = self.inner.lock();
        inner.pending_permission = Some(PermissionRequest {
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

    // Handle terminal events (keyboard input)
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
    // If in trust mode, render trust prompt
    if state.mode == AppMode::Trust {
        if let Some(ref prompt) = state.trust_prompt {
            return render_trust_prompt(prompt);
        }
    }

    // If in thinking toggle mode, render just the thinking dialog
    if state.mode == AppMode::ThinkingToggle {
        if let Some(ref dialog) = state.thinking_dialog {
            return render_thinking_dialog(dialog);
        }
    }

    // If in permission mode, render just the permission dialog (full-screen)
    if state.mode == AppMode::Permission {
        if let Some(ref perm) = state.pending_permission {
            return render_permission_dialog(perm);
        }
    }

    // Format header lines
    let (header_line1, header_line2, header_line3) = format_header_lines(state);

    // Format input line
    // Show placeholder only when there's no conversation history (initial state)
    let input_display = if state.input_buffer.is_empty() {
        if state.conversation_display.is_empty() && state.response_content.is_empty() {
            // Show placeholder only on initial state
            "❯ Try \"refactor mod.rs\"".to_string()
        } else {
            // After conversation started, show just the cursor
            "❯".to_string()
        }
    } else {
        format!("❯ {}", state.input_buffer)
    };

    // Main layout matching real Claude CLI
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
        ) {
            // Header with Claude branding (3 lines)
            Text(content: header_line1)
            Text(content: header_line2)
            Text(content: header_line3)

            // Empty line after header
            Text(content: "")

            // Conversation history area (if any)
            #(render_conversation_area(state))

            // Input area with separators
            Text(content: SEPARATOR)
            Text(content: input_display)
            Text(content: SEPARATOR)

            // Status bar
            Text(content: format_status_bar(state))
        }
    }
    .into()
}

/// Render conversation history area
fn render_conversation_area(state: &RenderState) -> AnyElement<'static> {
    let mut content = String::new();

    // Add compact separator if conversation has been compacted
    if state.is_compacted {
        content.push_str(COMPACT_SEPARATOR);
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

/// Full-width horizontal separator (120 chars)
const SEPARATOR: &str = "────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────";

/// Compact separator with centered text (matches real Claude CLI)
const COMPACT_SEPARATOR: &str = "══════════════════════════════════════ Conversation compacted · ctrl+o for history ═════════════════════════════════════";

/// Build tool summary from session turns for /compact output
fn build_tool_summary(sessions: &Arc<Mutex<SessionManager>>) -> String {
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
fn format_tool_summary(tool: &crate::state::session::TurnToolCall) -> Option<String> {
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

/// Render trust prompt dialog
fn render_trust_prompt(prompt: &TrustPromptState) -> AnyElement<'static> {
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
            Text(content: SEPARATOR)
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
fn render_thinking_dialog(dialog: &ThinkingDialog) -> AnyElement<'static> {
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
            Text(content: SEPARATOR)
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

/// Render rich permission dialog
fn render_permission_dialog(perm: &PermissionRequest) -> AnyElement<'static> {
    // Render the dialog content using the widget
    let content = perm.dialog.render(120);

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

    // Header line 1: Conditional branding based on claude_version
    let line1 = match &state.claude_version {
        Some(version) => format!(" ▐▛███▜▌   Claude Code v{}", version),
        None => format!(" ▐▛███▜▌   Claudeless {}", env!("CARGO_PKG_VERSION")),
    };
    let line2 = format!("▝▜█████▛▘  {} · Claude Max", model_name);
    let line3 = format!("  ▘▘ ▝▝    {}", working_dir);

    (line1, line2, line3)
}

/// Format status bar content
fn format_status_bar(state: &RenderState) -> String {
    // Check for exit hint first (takes precedence)
    if let Some(hint) = &state.exit_hint {
        return match hint {
            ExitHint::CtrlC => "  Press Ctrl-C again to exit".to_string(),
            ExitHint::CtrlD => "  Press Ctrl-D again to exit".to_string(),
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
                let padding = 120 - mode_text.len() - "Thinking off".len();
                format!("{}{:width$}Thinking off", mode_text, "", width = padding)
            }
        }
        _ => {
            // Non-default modes show "Use meta+t to toggle thinking" on the right
            let right_text = "Use meta+t to toggle thinking";
            // Use 120 char width to match the default SEPARATOR width
            let total_width: usize = 120;
            // Calculate visual width of mode_text (accounting for multi-byte chars)
            let mode_visual_width = mode_text.chars().count();
            let right_width = right_text.len();
            let padding = total_width.saturating_sub(mode_visual_width + right_width);
            format!("{}{:width$}{}", mode_text, "", right_text, width = padding)
        }
    }
}

/// Map model ID to display name
fn model_display_name(model: &str) -> String {
    // Handle short aliases (from --model flag)
    let model_lower = model.to_lowercase();
    let base_name = match model_lower.as_str() {
        "haiku" | "claude-haiku" => "Haiku",
        "sonnet" | "claude-sonnet" => "Sonnet",
        "opus" | "claude-opus" => "Opus",
        _ => {
            // Parse full model ID like "claude-sonnet-4-20250514"
            if model_lower.contains("haiku") {
                "Haiku"
            } else if model_lower.contains("opus") {
                "Opus"
            } else if model_lower.contains("sonnet") {
                "Sonnet"
            } else {
                // Unknown model, show as-is
                return model.to_string();
            }
        }
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
        let state = self.state.clone();

        // Check if we're already in a tokio runtime
        if tokio::runtime::Handle::try_current().is_ok() {
            // Already in a runtime - use block_in_place to run async code
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    // ignore_ctrl_c() prevents iocraft from exiting on Ctrl+C - we handle it ourselves
                    element!(App(state: Some(state.clone())))
                        .fullscreen()
                        .ignore_ctrl_c()
                        .await
                })
            })?;
        } else {
            // No runtime - create a new one
            let rt = tokio::runtime::Runtime::new()?;
            // ignore_ctrl_c() prevents iocraft from exiting on Ctrl+C - we handle it ourselves
            rt.block_on(async {
                element!(App(state: Some(state.clone())))
                    .fullscreen()
                    .ignore_ctrl_c()
                    .await
            })?;
        }
        Ok(self.state.exit_reason().unwrap_or(ExitReason::Completed))
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
