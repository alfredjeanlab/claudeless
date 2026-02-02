// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI application types and configuration.

use std::path::PathBuf;

use crate::config::{ResolvedTimeouts, ScenarioConfig, DEFAULT_MODEL, DEFAULT_USER_NAME};
use crate::permission::PermissionMode;
use crate::runtime::Runtime;
use crate::tui::widgets::permission::RichPermissionDialog;
use crate::tui::widgets::trust::TrustChoice;

use super::state::{DialogState, DisplayState, InputState};

/// Configuration from scenario for TUI behavior
#[derive(Clone, Debug)]
pub struct TuiConfig {
    pub trusted: bool,
    pub logged_in: bool,
    pub user_name: String,
    pub model: String,
    pub working_directory: PathBuf,
    pub permission_mode: PermissionMode,
    /// Whether bypass permissions mode is allowed (requires --dangerously-skip-permissions)
    pub allow_bypass_permissions: bool,
    /// Whether bypass confirmation dialog should be shown on startup
    /// (--dangerously-skip-permissions without --allow-dangerously-skip-permissions)
    pub bypass_confirmation_needed: bool,
    /// Resolved timeout configuration
    pub timeouts: ResolvedTimeouts,
    /// Explicit Claude version, or None for Claudeless-native mode
    pub claude_version: Option<String>,
    /// Whether output is connected to a TTY
    pub is_tty: bool,
    /// Initial prompt from CLI positional argument
    pub initial_prompt: Option<String>,
    /// Placeholder text for the input prompt
    pub placeholder: Option<String>,
    /// Provider name shown in header (default: "Claude Max")
    pub provider: Option<String>,
    /// Show "Welcome back!" splash instead of normal header
    pub show_welcome_back: bool,
    /// Right panel rows for the welcome back box (None = default Tips/Recent activity)
    pub welcome_back_right_panel: Option<Vec<String>>,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            trusted: true,
            logged_in: true,
            user_name: DEFAULT_USER_NAME.to_string(),
            model: DEFAULT_MODEL.to_string(),
            working_directory: std::env::current_dir().unwrap_or_default(),
            permission_mode: PermissionMode::Default,
            allow_bypass_permissions: false,
            bypass_confirmation_needed: false,
            timeouts: ResolvedTimeouts::default(),
            claude_version: None,
            is_tty: false,
            initial_prompt: None,
            placeholder: None,
            provider: None,
            show_welcome_back: false,
            welcome_back_right_panel: None,
        }
    }
}

impl TuiConfig {
    /// Create TuiConfig from a Runtime.
    ///
    /// This extracts all needed configuration from the Runtime, avoiding
    /// duplicate loading of scenario, settings, and hooks.
    pub fn from_runtime(
        runtime: &Runtime,
        allow_bypass_permissions: bool,
        bypass_confirmation_needed: bool,
        is_tty: bool,
    ) -> Self {
        let cli = runtime.cli();
        let config = runtime.scenario_config();

        // CLI permission mode overrides scenario (unless CLI is default)
        let permission_mode = if cli.permissions.permission_mode != PermissionMode::Default {
            cli.permissions.permission_mode.clone()
        } else {
            config
                .environment
                .permission_mode
                .as_deref()
                .and_then(|s| clap::ValueEnum::from_str(s, true).ok())
                .unwrap_or_default()
        };

        // CLI claude_version overrides scenario
        let claude_version = cli
            .simulator
            .claude_version
            .clone()
            .or_else(|| config.identity.claude_version.clone());

        Self {
            trusted: config.environment.trusted,
            logged_in: config.environment.logged_in,
            user_name: config
                .identity
                .user_name
                .clone()
                .unwrap_or_else(|| DEFAULT_USER_NAME.to_string()),
            model: config
                .identity
                .default_model
                .clone()
                .unwrap_or_else(|| cli.model.clone()),
            working_directory: config
                .environment
                .working_directory
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default()),
            permission_mode,
            allow_bypass_permissions,
            bypass_confirmation_needed,
            timeouts: runtime.timeouts().clone(),
            claude_version,
            is_tty,
            initial_prompt: cli.prompt.clone(),
            placeholder: config.identity.placeholder.clone(),
            provider: config.identity.provider.clone(),
            show_welcome_back: config.identity.show_welcome_back.unwrap_or(false),
            welcome_back_right_panel: config.identity.welcome_back_right_panel.clone(),
        }
    }

    // TODO(refactor): Group bypass/permission params into a struct
    #[allow(clippy::too_many_arguments)]
    pub fn from_scenario(
        config: &ScenarioConfig,
        cli_model: Option<&str>,
        cli_permission_mode: &PermissionMode,
        allow_bypass_permissions: bool,
        bypass_confirmation_needed: bool,
        cli_claude_version: Option<&str>,
        is_tty: bool,
        initial_prompt: Option<String>,
    ) -> Self {
        // CLI permission mode overrides scenario (unless CLI is default)
        let permission_mode = if *cli_permission_mode != PermissionMode::Default {
            cli_permission_mode.clone()
        } else {
            config
                .environment
                .permission_mode
                .as_deref()
                .and_then(|s| clap::ValueEnum::from_str(s, true).ok())
                .unwrap_or_default()
        };

        // CLI claude_version overrides scenario
        let claude_version = cli_claude_version
            .map(|s| s.to_string())
            .or_else(|| config.identity.claude_version.clone());

        Self {
            trusted: config.environment.trusted,
            logged_in: config.environment.logged_in,
            user_name: config
                .identity
                .user_name
                .clone()
                .unwrap_or_else(|| DEFAULT_USER_NAME.to_string()),
            model: cli_model
                .map(|s| s.to_string())
                .or_else(|| config.identity.default_model.clone())
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            working_directory: config
                .environment
                .working_directory
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default()),
            permission_mode,
            allow_bypass_permissions,
            bypass_confirmation_needed,
            timeouts: ResolvedTimeouts::resolve(config.timing.timeouts.as_ref()),
            claude_version,
            is_tty,
            initial_prompt,
            placeholder: config.identity.placeholder.clone(),
            provider: config.identity.provider.clone(),
            show_welcome_back: config.identity.show_welcome_back.unwrap_or(false),
            welcome_back_right_panel: config.identity.welcome_back_right_panel.clone(),
        }
    }
}

/// Application mode
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppMode {
    /// Showing setup wizard (first run)
    Setup,
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
    /// Showing bypass permissions confirmation dialog
    BypassConfirm,
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
    /// Whether compacting is in progress
    pub is_compacting: bool,
    /// Current spinner frame index
    pub spinner_frame: usize,
    /// Current spinner verb
    pub spinner_verb: String,
    /// Placeholder text for the input prompt
    pub placeholder: Option<String>,
    /// Provider name shown in header
    pub provider: Option<String>,
    /// Show "Welcome back!" splash instead of normal header
    pub show_welcome_back: bool,
    /// Right panel rows for the welcome back box
    pub welcome_back_right_panel: Option<Vec<String>>,
}

/// Permission request state using the rich permission dialog
#[derive(Clone, Debug)]
pub struct PermissionRequest {
    pub dialog: RichPermissionDialog,
    /// Tool use ID for JSONL recording
    pub tool_use_id: Option<String>,
    /// Display content to show after permission is granted (completed tools + response text).
    /// If None, falls back to the simple "[Permission granted]" message.
    pub post_grant_display: Option<String>,
}

impl PermissionRequest {
    pub fn new(dialog: RichPermissionDialog) -> Self {
        Self {
            dialog,
            tool_use_id: None,
            post_grant_display: None,
        }
    }

    pub fn with_tool_use_id(dialog: RichPermissionDialog, tool_use_id: String) -> Self {
        Self {
            dialog,
            tool_use_id: Some(tool_use_id),
            post_grant_display: None,
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

/// Default terminal width when not detected
pub const DEFAULT_TERMINAL_WIDTH: u16 = 120;

/// Trust prompt state (simplified for iocraft)
#[derive(Clone, Debug)]
pub struct TrustPromptState {
    pub working_directory: String,
    pub selected: TrustChoice,
}

/// Bypass confirmation dialog choice
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BypassChoice {
    /// No, exit (default)
    No,
    /// Yes, I accept
    Yes,
}

/// Bypass confirmation dialog state
#[derive(Clone, Debug)]
pub struct BypassConfirmState {
    pub selected: BypassChoice,
}

impl TrustPromptState {
    pub fn new(working_directory: String) -> Self {
        Self {
            working_directory,
            selected: TrustChoice::Yes,
        }
    }
}
