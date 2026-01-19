// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! CLI argument parsing matching Claude's interface.

use clap::{Parser, ValueEnum};

use crate::config::ToolExecutionMode;
use crate::permission::PermissionMode;

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
    #[arg(long, default_value = "claude-sonnet-4-20250514")]
    pub model: String,

    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    pub output_format: OutputFormat,

    /// Maximum tokens in response
    #[arg(long)]
    pub max_tokens: Option<u32>,

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

    /// Response delay in milliseconds
    #[arg(long, env = "CLAUDELESS_DELAY_MS")]
    pub delay_ms: Option<u64>,

    /// Enable TUI mode (interactive terminal interface)
    #[arg(long, env = "CLAUDELESS_TUI")]
    pub tui: bool,

    /// Force non-TUI mode even if stdin is a TTY
    #[arg(long)]
    pub no_tui: bool,

    /// Tool execution mode (overrides scenario config)
    #[arg(long, value_enum, env = "CLAUDELESS_TOOL_EXECUTION_MODE")]
    pub tool_execution_mode: Option<CliToolExecutionMode>,

    /// Root directory for sandboxed tool execution
    #[arg(long, env = "CLAUDELESS_SANDBOX_ROOT")]
    pub sandbox_root: Option<String>,

    /// Allow real Bash execution in simulated mode
    #[arg(long, env = "CLAUDELESS_ALLOW_REAL_BASH")]
    pub allow_real_bash: bool,
}

/// CLI-friendly tool execution mode enum
#[derive(Clone, Debug, ValueEnum)]
pub enum CliToolExecutionMode {
    /// No tool execution (default)
    Disabled,
    /// Return pre-configured results from scenario
    Mock,
    /// Execute built-in tools in sandbox
    Simulated,
    /// Spawn real MCP servers
    RealMcp,
}

impl From<CliToolExecutionMode> for ToolExecutionMode {
    fn from(mode: CliToolExecutionMode) -> Self {
        match mode {
            CliToolExecutionMode::Disabled => ToolExecutionMode::Disabled,
            CliToolExecutionMode::Mock => ToolExecutionMode::Mock,
            CliToolExecutionMode::Simulated => ToolExecutionMode::Simulated,
            CliToolExecutionMode::RealMcp => ToolExecutionMode::RealMcp,
        }
    }
}

impl Cli {
    /// Determine if TUI mode should be used
    pub fn should_use_tui(&self) -> bool {
        if self.no_tui {
            return false;
        }
        if self.tui {
            return true;
        }
        // Auto-detect: use TUI if stdin is a TTY and not in print mode
        use std::io::IsTerminal;
        !self.print && std::io::stdin().is_terminal()
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
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_prompt() {
        let cli = Cli::try_parse_from(["claude", "hello world"]).unwrap();
        assert_eq!(cli.prompt, Some("hello world".to_string()));
        assert!(!cli.print);
    }

    #[test]
    fn test_parse_print_mode() {
        let cli = Cli::try_parse_from(["claude", "-p", "test prompt"]).unwrap();
        assert!(cli.print);
        assert_eq!(cli.prompt, Some("test prompt".to_string()));
    }

    #[test]
    fn test_parse_output_format_json() {
        let cli = Cli::try_parse_from(["claude", "--output-format", "json", "-p", "test"]).unwrap();
        assert!(matches!(cli.output_format, OutputFormat::Json));
    }

    #[test]
    fn test_parse_output_format_stream_json() {
        let cli = Cli::try_parse_from(["claude", "--output-format", "stream-json", "-p", "test"])
            .unwrap();
        assert!(matches!(cli.output_format, OutputFormat::StreamJson));
    }

    #[test]
    fn test_parse_model() {
        let cli =
            Cli::try_parse_from(["claude", "--model", "claude-opus-4-20250514", "-p", "test"])
                .unwrap();
        assert_eq!(cli.model, "claude-opus-4-20250514");
    }

    #[test]
    fn test_parse_allowed_tools() {
        let cli = Cli::try_parse_from([
            "claude",
            "--allowedTools",
            "Bash",
            "--allowedTools",
            "Read",
            "-p",
            "test",
        ])
        .unwrap();
        assert_eq!(cli.allowed_tools, vec!["Bash", "Read"]);
    }

    #[test]
    fn test_parse_simulator_flags() {
        let cli = Cli::try_parse_from([
            "claude",
            "--scenario",
            "/path/to/scenario.toml",
            "--capture",
            "/tmp/capture.jsonl",
            "--failure",
            "rate-limit",
            "--delay-ms",
            "100",
            "-p",
            "test",
        ])
        .unwrap();
        assert_eq!(cli.scenario, Some("/path/to/scenario.toml".to_string()));
        assert_eq!(cli.capture, Some("/tmp/capture.jsonl".to_string()));
        assert!(matches!(cli.failure, Some(FailureMode::RateLimit)));
        assert_eq!(cli.delay_ms, Some(100));
    }

    #[test]
    fn test_parse_continue_conversation() {
        let cli = Cli::try_parse_from(["claude", "-c", "-p", "continue"]).unwrap();
        assert!(cli.continue_conversation);
    }

    #[test]
    fn test_parse_resume() {
        let cli = Cli::try_parse_from(["claude", "-r", "session-123", "-p", "resume"]).unwrap();
        assert_eq!(cli.resume, Some("session-123".to_string()));
    }

    #[test]
    fn test_default_model() {
        let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
        assert_eq!(cli.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_parse_max_tokens() {
        let cli = Cli::try_parse_from(["claude", "--max-tokens", "4096", "-p", "test"]).unwrap();
        assert_eq!(cli.max_tokens, Some(4096));
    }

    #[test]
    fn test_parse_system_prompt() {
        let cli = Cli::try_parse_from([
            "claude",
            "--system-prompt",
            "You are a helpful assistant",
            "-p",
            "test",
        ])
        .unwrap();
        assert_eq!(
            cli.system_prompt,
            Some("You are a helpful assistant".to_string())
        );
    }

    #[test]
    fn test_parse_cwd() {
        let cli =
            Cli::try_parse_from(["claude", "--cwd", "/home/user/project", "-p", "test"]).unwrap();
        assert_eq!(cli.cwd, Some("/home/user/project".to_string()));
    }

    #[test]
    fn test_parse_input_format() {
        let cli =
            Cli::try_parse_from(["claude", "--input-format", "stream-json", "-p", "test"]).unwrap();
        assert_eq!(cli.input_format, "stream-json");
    }

    #[test]
    fn test_default_input_format() {
        let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
        assert_eq!(cli.input_format, "text");
    }

    #[test]
    fn test_parse_session_id() {
        let cli = Cli::try_parse_from([
            "claude",
            "-p",
            "--session-id",
            "01234567-89ab-cdef-0123-456789abcdef",
            "test",
        ])
        .unwrap();
        assert_eq!(
            cli.session_id,
            Some("01234567-89ab-cdef-0123-456789abcdef".to_string())
        );
    }

    #[test]
    fn test_parse_verbose() {
        let cli = Cli::try_parse_from(["claude", "--verbose", "-p", "test"]).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_parse_debug() {
        // Debug flag without value
        let cli = Cli::try_parse_from(["claude", "-d", "-p", "test"]).unwrap();
        assert!(cli.debug.is_some());
    }

    #[test]
    fn test_parse_include_partial_messages() {
        let cli =
            Cli::try_parse_from(["claude", "--include-partial-messages", "-p", "test"]).unwrap();
        assert!(cli.include_partial_messages);
    }

    #[test]
    fn test_parse_fallback_model() {
        let cli = Cli::try_parse_from(["claude", "--fallback-model", "claude-haiku", "-p", "test"])
            .unwrap();
        assert_eq!(cli.fallback_model, Some("claude-haiku".to_string()));
    }

    #[test]
    fn test_parse_max_budget_usd() {
        let cli =
            Cli::try_parse_from(["claude", "--max-budget-usd", "10.50", "-p", "test"]).unwrap();
        assert_eq!(cli.max_budget_usd, Some(10.50));
    }
}
