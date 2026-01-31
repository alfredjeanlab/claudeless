// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! CLI argument parsing matching Claude's interface.

use clap::{Parser, ValueEnum};

use crate::config::ToolExecutionMode;
use crate::permission::PermissionMode;
use crate::state::SettingSource;

/// Claude CLI Simulator
#[derive(Parser, Debug)]
#[command(name = "claude", version, about = "Claude CLI Simulator")]
pub struct Cli {
    /// The prompt to send (positional or via --print)
    #[arg(value_name = "PROMPT")]
    pub prompt: Option<String>,

    /// Print mode - non-interactive single response
    #[arg(short = 'p', long)]
    pub print: bool,

    /// Model to use (ignored by simulator, accepted for compatibility)
    #[arg(long, default_value = "claude-opus-4-5-20251101")]
    pub model: String,

    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    pub output_format: OutputFormat,

    /// System prompt
    #[arg(long)]
    pub system_prompt: Option<String>,

    /// Continue previous conversation
    #[arg(long, short = 'c')]
    pub continue_conversation: bool,

    /// Resume a specific conversation by ID
    #[arg(long, short = 'r')]
    pub resume: Option<String>,

    /// Allowed tools (can be specified multiple times)
    #[arg(long = "allowedTools")]
    pub allowed_tools: Vec<String>,

    /// Disallowed tools
    #[arg(long = "disallowedTools")]
    pub disallowed_tools: Vec<String>,

    /// Permission mode for tool execution
    #[arg(long = "permission-mode", value_enum, default_value = "default")]
    pub permission_mode: PermissionMode,

    /// Enable bypassing all permission checks as an option.
    /// Recommended only for sandboxes with no internet access.
    #[arg(long, env = "CLAUDE_ALLOW_DANGEROUSLY_SKIP_PERMISSIONS")]
    pub allow_dangerously_skip_permissions: bool,

    /// Bypass all permission checks.
    /// Recommended only for sandboxes with no internet access.
    /// Requires --allow-dangerously-skip-permissions to be set.
    #[arg(long, env = "CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS")]
    pub dangerously_skip_permissions: bool,

    /// Input file to read prompt from
    #[arg(long)]
    pub input_file: Option<String>,

    /// Working directory
    #[arg(long = "cwd")]
    pub cwd: Option<String>,

    /// Comma-separated list of setting sources to load (user, project, local).
    /// When not specified, all sources are loaded.
    #[arg(long = "setting-sources", value_delimiter = ',')]
    pub setting_sources: Option<Vec<SettingSource>>,

    /// Input format (text or stream-json)
    #[arg(long, value_parser = ["text", "stream-json"], default_value = "text")]
    pub input_format: String,

    /// Use a specific session ID
    #[arg(long)]
    pub session_id: Option<String>,

    /// Verbose output mode
    #[arg(long)]
    pub verbose: bool,

    /// Debug mode with optional filter
    #[arg(short = 'd', long)]
    pub debug: Option<Option<String>>,

    /// Include partial message chunks (with stream-json output)
    #[arg(long)]
    pub include_partial_messages: bool,

    /// Fallback model on overload
    #[arg(long)]
    pub fallback_model: Option<String>,

    /// Maximum budget in USD
    #[arg(long)]
    pub max_budget_usd: Option<f64>,

    /// Load MCP servers from JSON files or inline JSON strings (can be specified multiple times)
    #[arg(long, value_name = "CONFIG")]
    pub mcp_config: Vec<String>,

    /// Only use MCP servers from --mcp-config, ignoring all other MCP configurations
    #[arg(long)]
    pub strict_mcp_config: bool,

    /// Enable MCP debug mode (shows MCP server errors)
    #[arg(long)]
    pub mcp_debug: bool,

    /// Disable session persistence - sessions will not be saved to disk and
    /// cannot be resumed (only works with --print)
    #[arg(long)]
    pub no_session_persistence: bool,

    /// Load settings from a JSON file or inline JSON string (can be specified multiple times)
    #[arg(long, value_name = "FILE_OR_JSON")]
    pub settings: Vec<String>,

    // Simulator-specific flags (not in real Claude)
    /// Scenario file or directory for scripted responses
    #[arg(long, env = "CLAUDELESS_SCENARIO")]
    pub scenario: Option<String>,

    /// Capture file for recording interactions
    #[arg(long, env = "CLAUDELESS_CAPTURE")]
    pub capture: Option<String>,

    /// Failure mode to inject
    #[arg(long, env = "CLAUDELESS_FAILURE")]
    pub failure: Option<FailureMode>,

    /// Tool execution mode (overrides scenario config)
    #[arg(long, value_enum, env = "CLAUDELESS_TOOL_MODE")]
    pub tool_mode: Option<CliToolExecutionMode>,

    /// Claude version to simulate (e.g., "2.1.12")
    /// When not set, displays "Claudeless" branding instead of "Claude Code"
    #[arg(long, env = "CLAUDELESS_CLAUDE_VERSION")]
    pub claude_version: Option<String>,
}

/// CLI-friendly tool execution mode enum
#[derive(Clone, Debug, ValueEnum)]
pub enum CliToolExecutionMode {
    /// No tool execution (default)
    Disabled,
    /// Return pre-configured results from scenario
    Mock,
    /// Execute built-in tools directly
    Live,
}

impl From<CliToolExecutionMode> for ToolExecutionMode {
    fn from(mode: CliToolExecutionMode) -> Self {
        match mode {
            CliToolExecutionMode::Disabled => ToolExecutionMode::Disabled,
            CliToolExecutionMode::Mock => ToolExecutionMode::Mock,
            CliToolExecutionMode::Live => ToolExecutionMode::Live,
        }
    }
}

impl Cli {
    /// Determine if TUI mode should be used
    pub fn should_use_tui(&self) -> bool {
        use std::io::IsTerminal;
        !self.print && std::io::stdin().is_terminal()
    }

    /// Validate that --no-session-persistence is only used with --print
    pub fn validate_no_session_persistence(&self) -> Result<(), &'static str> {
        if self.no_session_persistence && !self.print {
            return Err("--no-session-persistence can only be used with --print mode");
        }
        Ok(())
    }

    /// Validate that --session-id is a valid UUID if provided
    pub fn validate_session_id(&self) -> Result<(), &'static str> {
        if let Some(ref id) = self.session_id {
            if uuid::Uuid::parse_str(id).is_err() {
                return Err("Invalid session ID. Must be a valid UUID.");
            }
        }
        Ok(())
    }
}

/// Output format for responses
#[derive(Clone, Debug, ValueEnum, Default)]
pub enum OutputFormat {
    /// Plain text output
    #[default]
    Text,
    /// JSON response object
    Json,
    /// Line-delimited streaming JSON events
    #[value(name = "stream-json")]
    StreamJson,
}

/// Failure modes that can be injected
#[derive(Clone, Debug, ValueEnum)]
pub enum FailureMode {
    /// Simulate network unreachable error
    NetworkUnreachable,
    /// Simulate connection timeout
    ConnectionTimeout,
    /// Simulate authentication error
    AuthError,
    /// Simulate rate limiting (429)
    RateLimit,
    /// Simulate out of credits / billing error
    OutOfCredits,
    /// Simulate partial/interrupted response
    PartialResponse,
    /// Return malformed JSON
    MalformedJson,
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
