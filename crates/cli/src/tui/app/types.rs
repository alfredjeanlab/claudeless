// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI application types and configuration.

use std::path::PathBuf;

use crate::config::{ScenarioConfig, DEFAULT_MODEL, DEFAULT_USER_NAME};
use crate::permission::PermissionMode;
use crate::tui::widgets::permission::{PermissionSelection, RichPermissionDialog};
use crate::tui::widgets::trust::TrustChoice;

use super::state::{DialogState, DisplayState, InputState};

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
    /// Whether output is connected to a TTY
    pub is_tty: bool,
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
            is_tty: false,
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
        is_tty: bool,
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
            is_tty,
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
    /// Showing tasks dialog
    TasksDialog,
    /// Showing model picker dialog
    ModelPicker,
    /// Showing export dialog
    ExportDialog,
    /// Showing help dialog
    HelpDialog,
    /// Showing hooks management dialog
    HooksDialog,
    /// Showing memory management dialog
    MemoryDialog,
}

/// Status bar information
#[derive(Clone, Debug, Default)]
pub struct StatusInfo {
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub session_id: Option<String>,
}

/// Snapshot of app state for rendering - uses focused state structs
#[derive(Clone, Debug)]
pub struct RenderState {
    pub mode: AppMode,
    /// Input state (buffer, cursor, history, etc.)
    pub input: InputState,
    /// Dialog state (active dialog if any)
    pub dialog: DialogState,
    /// Display state (response, conversation, terminal, etc.)
    pub display: DisplayState,
    pub status: StatusInfo,
    pub permission_mode: PermissionMode,
    pub thinking_enabled: bool,
    pub user_name: String,
    /// Explicit Claude version, or None for Claudeless-native mode
    pub claude_version: Option<String>,
    /// Whether output is connected to a TTY
    pub is_tty: bool,
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
    Suspended,     // Ctrl+Z suspend (will resume)
}

/// Type of exit hint being shown
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExitHint {
    /// "Press Ctrl-C again to exit"
    CtrlC,
    /// "Press Ctrl-D again to exit"
    CtrlD,
    /// "Esc to clear again" (after closing slash menu)
    Escape,
}

/// Exit hint timeout in milliseconds (2 seconds)
pub const EXIT_HINT_TIMEOUT_MS: u64 = 2000;

/// Default terminal width when not detected
pub const DEFAULT_TERMINAL_WIDTH: u16 = 120;

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
