// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! CLI argument parsing matching Claude's interface.

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::permission::PermissionMode;
use crate::state::SettingSource;

/// Claude CLI Simulator
#[derive(Parser, Debug, Clone)]
#[command(
    name = "claude",
    about = "Claude CLI Simulator",
    disable_help_flag = true,
    disable_version_flag = true
)]
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

    /// System prompt
    #[arg(long)]
    pub system_prompt: Option<String>,

    /// Allowed tools (can be specified multiple times)
    #[arg(long = "allowedTools", alias = "allowed-tools")]
    pub allowed_tools: Vec<String>,

    /// Disallowed tools
    #[arg(long = "disallowedTools", alias = "disallowed-tools")]
    pub disallowed_tools: Vec<String>,

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

    /// Fallback model on overload
    #[arg(long)]
    pub fallback_model: Option<String>,

    /// Maximum budget in USD
    #[arg(long)]
    pub max_budget_usd: Option<f64>,

    /// Load settings from a JSON file or inline JSON string (can be specified multiple times)
    #[arg(long, value_name = "FILE_OR_JSON")]
    pub settings: Vec<String>,

    // === Help and version (manual, not clap-managed) ===
    /// Display help for command
    #[arg(short = 'h', long)]
    pub help: bool,

    /// Output the version number
    #[arg(short = 'v', long)]
    pub version: bool,

    // === Compatibility flags (accepted but ignored) ===
    /// Additional directories to allow tool access to
    #[arg(long)]
    pub add_dir: Vec<String>,

    /// Agent for the current session
    #[arg(long)]
    pub agent: Option<String>,

    /// JSON object defining custom agents
    #[arg(long)]
    pub agents: Option<String>,

    /// Append a system prompt to the default system prompt
    #[arg(long)]
    pub append_system_prompt: Option<String>,

    /// Beta headers to include in API requests
    #[arg(long)]
    pub betas: Vec<String>,

    /// Enable Claude in Chrome integration
    #[arg(long)]
    pub chrome: bool,

    /// Disable Claude in Chrome integration
    #[arg(long)]
    pub no_chrome: bool,

    /// Write debug logs to a specific file path
    #[arg(long)]
    pub debug_file: Option<String>,

    /// Disable all skills
    #[arg(long)]
    pub disable_slash_commands: bool,

    /// File resources to download at startup
    #[arg(long)]
    pub file: Vec<String>,

    /// When resuming, create a new session ID
    #[arg(long)]
    pub fork_session: bool,

    /// Resume a session linked to a PR
    #[arg(long)]
    pub from_pr: Option<Option<String>>,

    /// Automatically connect to IDE on startup
    #[arg(long)]
    pub ide: bool,

    /// JSON Schema for structured output validation
    #[arg(long)]
    pub json_schema: Option<String>,

    /// Load plugins from directories for this session only
    #[arg(long)]
    pub plugin_dir: Vec<String>,

    /// Re-emit user messages from stdin back on stdout
    #[arg(long)]
    pub replay_user_messages: bool,

    /// Specify the list of available tools from the built-in set
    #[arg(long)]
    pub tools: Vec<String>,

    #[command(flatten)]
    pub output: OutputOptions,

    #[command(flatten)]
    pub session: SessionOptions,

    #[command(flatten)]
    pub permissions: PermissionOptions,

    #[command(flatten)]
    pub mcp: McpOptions,

    #[command(flatten)]
    pub simulator: SimulatorOptions,

    /// Subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Output formatting options.
#[derive(Args, Debug, Clone, Default)]
pub struct OutputOptions {
    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    pub output_format: OutputFormat,

    /// Verbose output mode
    #[arg(long)]
    pub verbose: bool,

    /// Debug mode with optional filter
    #[arg(short = 'd', long)]
    pub debug: Option<Option<String>>,

    /// Include partial message chunks (with stream-json output)
    #[arg(long)]
    pub include_partial_messages: bool,
}

/// Session management options.
#[derive(Args, Debug, Clone, Default)]
pub struct SessionOptions {
    /// Continue previous conversation
    #[arg(long = "continue", short = 'c')]
    pub continue_conversation: bool,

    /// Resume a specific conversation by ID
    #[arg(long, short = 'r')]
    pub resume: Option<String>,

    /// Use a specific session ID
    #[arg(long)]
    pub session_id: Option<String>,

    /// Disable session persistence - sessions will not be saved to disk and
    /// cannot be resumed (only works with --print)
    #[arg(long)]
    pub no_session_persistence: bool,
}

impl SessionOptions {
    /// Validate that --no-session-persistence is only used with --print
    pub fn validate_no_session_persistence(&self, print_mode: bool) -> Result<(), &'static str> {
        if self.no_session_persistence && !print_mode {
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

/// Permission control options.
#[derive(Args, Debug, Clone)]
pub struct PermissionOptions {
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
}

impl Default for PermissionOptions {
    fn default() -> Self {
        Self {
            permission_mode: PermissionMode::Default,
            allow_dangerously_skip_permissions: false,
            dangerously_skip_permissions: false,
        }
    }
}

/// MCP (Model Context Protocol) server options.
#[derive(Args, Debug, Clone, Default)]
pub struct McpOptions {
    /// Load MCP servers from JSON files or inline JSON strings (can be specified multiple times)
    #[arg(long, value_name = "CONFIG")]
    pub mcp_config: Vec<String>,

    /// Only use MCP servers from --mcp-config, ignoring all other MCP configurations
    #[arg(long)]
    pub strict_mcp_config: bool,

    /// Enable MCP debug mode (shows MCP server errors)
    #[arg(long)]
    pub mcp_debug: bool,
}

/// Simulator-specific options (not in real Claude).
#[derive(Args, Debug, Clone, Default)]
pub struct SimulatorOptions {
    /// Scenario file or directory for scripted responses
    #[arg(long, env = "CLAUDELESS_SCENARIO")]
    pub scenario: Option<String>,

    /// Failure mode to inject
    #[arg(long, env = "CLAUDELESS_FAILURE")]
    pub failure: Option<FailureMode>,

    /// Claude version to simulate (e.g., "2.1.12")
    /// When not set, displays "Claudeless" branding instead of "Claude Code"
    #[arg(long, env = "CLAUDELESS_CLAUDE_VERSION")]
    pub claude_version: Option<String>,
}

impl Cli {
    /// Determine if TUI mode should be used
    pub fn should_use_tui(&self) -> bool {
        use std::io::IsTerminal;
        !self.print && std::io::stdin().is_terminal()
    }

    /// Validate all CLI arguments.
    ///
    /// Calls validation methods on all option structs and returns the first error.
    /// Note: Permission bypass validation is handled separately by PermissionBypass
    /// in main.rs since it needs to output a specific error message.
    pub fn validate(&self) -> Result<(), &'static str> {
        self.session.validate_no_session_persistence(self.print)?;
        self.session.validate_session_id()?;
        Ok(())
    }
}

/// Subcommands matching real Claude Code CLI.
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Check the health of your Claude Code auto-updater
    #[command(disable_help_flag = true)]
    Doctor {
        #[arg(short = 'h', long)]
        help: bool,
    },

    /// Install Claude Code native build
    #[command(disable_help_flag = true)]
    Install {
        /// Target version (stable, latest, or specific version)
        target: Option<String>,

        /// Force installation even if already installed
        #[arg(long)]
        force: bool,

        #[arg(short = 'h', long)]
        help: bool,
    },

    /// Configure and manage MCP servers
    #[command(disable_help_flag = true)]
    Mcp {
        #[command(subcommand)]
        command: Option<McpCommands>,

        #[arg(short = 'h', long)]
        help: bool,
    },

    /// Manage Claude Code plugins
    #[command(disable_help_flag = true)]
    Plugin {
        #[command(subcommand)]
        command: Option<PluginCommands>,

        #[arg(short = 'h', long)]
        help: bool,
    },

    /// Set up a long-lived authentication token
    #[command(name = "setup-token", disable_help_flag = true)]
    SetupToken {
        #[arg(short = 'h', long)]
        help: bool,
    },

    /// Check for updates and install if available
    #[command(disable_help_flag = true)]
    Update {
        #[arg(short = 'h', long)]
        help: bool,
    },
}

impl Commands {
    /// Check if the subcommand wants to display help.
    pub fn wants_help(&self) -> bool {
        match self {
            Commands::Doctor { help } => *help,
            Commands::Install { help, .. } => *help,
            Commands::Mcp { help, command } => {
                *help
                    || match command {
                        Some(McpCommands::Add { help, .. }) => *help,
                        Some(McpCommands::Serve { help, .. }) => *help,
                        _ => false,
                    }
            }
            Commands::Plugin { help, command } => {
                *help
                    || match command {
                        Some(PluginCommands::Marketplace { help, command }) => {
                            *help || command.is_none()
                        }
                        _ => false,
                    }
            }
            Commands::SetupToken { help } => *help,
            Commands::Update { help } => *help,
        }
    }
}

/// MCP subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum McpCommands {
    /// Add an MCP server
    #[command(disable_help_flag = true)]
    Add {
        /// Server name
        name: Option<String>,

        /// Command or URL
        command_or_url: Option<String>,

        /// Additional arguments
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        /// Environment variables
        #[arg(short = 'e', long)]
        env: Vec<String>,

        /// WebSocket headers
        #[arg(short = 'H', long)]
        header: Vec<String>,

        /// Configuration scope
        #[arg(short = 's', long)]
        scope: Option<String>,

        /// Transport type
        #[arg(short = 't', long)]
        transport: Option<String>,

        #[arg(short = 'h', long)]
        help: bool,
    },

    /// Start the Claude Code MCP server
    #[command(disable_help_flag = true)]
    Serve {
        /// Enable debug mode
        #[arg(short = 'd', long)]
        debug: bool,

        /// Override verbose mode
        #[arg(long)]
        verbose: bool,

        #[arg(short = 'h', long)]
        help: bool,
    },
}

/// Plugin subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum PluginCommands {
    /// Manage Claude Code marketplaces
    #[command(disable_help_flag = true)]
    Marketplace {
        #[command(subcommand)]
        command: Option<MarketplaceCommands>,

        #[arg(short = 'h', long)]
        help: bool,
    },
}

/// Marketplace subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum MarketplaceCommands {
    /// Add a marketplace
    Add { source: String },
    /// List marketplaces
    List,
    /// Remove a marketplace
    Remove { name: String },
    /// Update marketplaces
    Update { name: Option<String> },
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
