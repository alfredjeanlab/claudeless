// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Claude CLI Simulator binary entry point.

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use parking_lot::RwLock;

use claudeless::capture::{CaptureLog, CapturedArgs, CapturedOutcome};
use claudeless::cli::Cli;
use claudeless::config::{ResolvedTimeouts, ResponseSpec, ToolExecutionMode};
use claudeless::failure::FailureExecutor;
use claudeless::mcp::{load_mcp_config, McpConfig, McpManager, McpServerStatus};
use claudeless::output::{
    print_error, print_mcp, print_mcp_error, print_mcp_warning, print_warning, McpServerInfo,
    OutputWriter,
};
use claudeless::permission::PermissionBypass;
use claudeless::scenario::Scenario;
use claudeless::session::SessionContext;
use claudeless::state::session::SessionManager;
use claudeless::state::{
    ClaudeSettings, SettingsLoader, SettingsPaths, StateDirectory, StateWriter,
};
use claudeless::time::ClockHandle;
use claudeless::tools::{create_executor_with_mcp, ExecutionContext};
use claudeless::tui::{ExitReason, TuiApp, TuiConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Validate --no-session-persistence usage
    if let Err(msg) = cli.validate_no_session_persistence() {
        print_error(msg);
        std::process::exit(1);
    }

    // Validate --session-id is a valid UUID
    if let Err(msg) = cli.validate_session_id() {
        print_error(msg);
        std::process::exit(1);
    }

    // Validate permission bypass configuration
    let bypass = PermissionBypass::new(
        cli.allow_dangerously_skip_permissions,
        cli.dangerously_skip_permissions,
    );
    if bypass.is_not_allowed() {
        eprintln!("{}", PermissionBypass::error_message());
        std::process::exit(1);
    }

    // Load settings from files and CLI overrides
    let _settings = load_settings(&cli);

    // Load and initialize MCP servers
    let mcp_manager = load_mcp_configs(&cli).await?;

    // Check for TUI mode first
    if cli.should_use_tui() {
        return run_tui_mode(&cli, bypass.is_active(), mcp_manager).await;
    }

    // In non-TUI mode (print mode), require a prompt
    if cli.prompt.is_none() {
        print_error("Input must be provided either through stdin or as a prompt argument when using --print");
        std::process::exit(1);
    }

    // Initialize capture log if requested
    let capture = if let Some(ref path) = cli.capture {
        Some(CaptureLog::with_file(Path::new(path))?)
    } else {
        None
    };

    // Get the prompt (from positional arg or would need to read stdin in real impl)
    let prompt = cli.prompt.clone().unwrap_or_default();

    // Record captured args
    let captured_args = CapturedArgs {
        prompt: cli.prompt.clone(),
        model: cli.model.clone(),
        output_format: format!("{:?}", cli.output_format).to_lowercase(),
        print_mode: cli.print,
        continue_conversation: cli.continue_conversation,
        resume: cli.resume.clone(),
        allowed_tools: cli.allowed_tools.clone(),
        cwd: cli.cwd.clone(),
    };

    // Load scenario if specified (needed for session context)
    let mut scenario = if let Some(ref path) = cli.scenario {
        Some(Scenario::load(Path::new(path))?)
    } else {
        None
    };

    // Build session context early (needed for state_writer in failure handling)
    let session_ctx = SessionContext::build(scenario.as_ref().map(|s| s.config()), &cli);

    // Create state writer early so failures can record to session JSONL
    // Skip if --no-session-persistence is enabled
    let state_writer = if !cli.no_session_persistence {
        Some(Arc::new(RwLock::new(StateWriter::new(
            session_ctx.session_id.to_string(),
            &session_ctx.project_path,
            session_ctx.launch_timestamp,
            &session_ctx.model,
            &session_ctx.working_directory,
        )?)))
    } else {
        None
    };

    // Handle failure injection from CLI flag
    if let Some(ref mode) = cli.failure {
        let spec = FailureExecutor::from_mode(mode);
        let mut stderr = io::stderr();

        if let Some(ref log) = capture {
            log.record(
                captured_args,
                CapturedOutcome::Failure {
                    failure_type: format!("{:?}", mode),
                    message: "Injected failure".to_string(),
                },
            );
        }

        // Write queue-operation before recording error
        if cli.print {
            if let Some(ref writer) = state_writer {
                writer.read().write_queue_operation()?;
            }
        }

        // Record error to session JSONL before exiting
        FailureExecutor::execute_with_session(&spec, &mut stderr, state_writer.as_ref()).await?;
        return Ok(());
    }

    // Apply response delay if configured (scenario already loaded above)
    let timeouts = ResolvedTimeouts::resolve(
        scenario
            .as_ref()
            .and_then(|s| s.config().timing.timeouts.as_ref()),
    );
    if timeouts.response_delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(timeouts.response_delay_ms)).await;
    }

    // Match prompt to get response
    let (response, matched_rule) = if let Some(ref mut s) = scenario {
        if let Some(result) = s.match_prompt(&prompt) {
            // Check for failure in rule
            if let Some(failure_spec) = s.get_failure(&result) {
                let mut stderr = io::stderr();

                if let Some(ref log) = capture {
                    log.record(
                        captured_args,
                        CapturedOutcome::Failure {
                            failure_type: format!("{:?}", failure_spec),
                            message: "Scenario failure".to_string(),
                        },
                    );
                }

                // Write queue-operation before recording error
                if cli.print {
                    if let Some(ref writer) = state_writer {
                        writer.read().write_queue_operation()?;
                    }
                }

                // Record error to session JSONL before exiting
                FailureExecutor::execute_with_session(
                    failure_spec,
                    &mut stderr,
                    state_writer.as_ref(),
                )
                .await?;
                return Ok(());
            }
            (
                s.get_response(&result).cloned(),
                Some("matched".to_string()),
            )
        } else if let Some(default) = s.default_response() {
            (Some(default.clone()), Some("default".to_string()))
        } else {
            (None, None)
        }
    } else {
        // No scenario - use a default response
        (
            Some(ResponseSpec::Simple("Hello! I'm Claudeless!".to_string())),
            None,
        )
    };

    // Get response delay from spec if detailed
    let response_delay = response.as_ref().and_then(|r| r.delay_ms());

    if let Some(delay) = response_delay {
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }

    // Record outcome
    if let Some(ref log) = capture {
        let outcome = match &response {
            Some(spec) => CapturedOutcome::Response {
                text: spec.text().to_string(),
                matched_rule,
                delay_ms: response_delay.unwrap_or(0),
            },
            None => CapturedOutcome::NoMatch {
                used_default: false,
            },
        };
        log.record(captured_args, outcome);
    }

    // Write output (session_ctx already built above)
    let mut stdout = io::stdout();
    let response = response.unwrap_or(ResponseSpec::Simple(String::new()));
    let tool_calls = response.tool_calls().to_vec();

    // Use real Claude format (result wrapper for JSON, system init + result for stream-JSON)
    let mut writer = OutputWriter::new(&mut stdout, cli.output_format.clone(), cli.model.clone());

    // Combine builtin tools with MCP tools (MCP tools use mcp__<server>__<tool> naming)
    let mut tools: Vec<String> = cli.allowed_tools.clone();
    let mcp_tools = get_mcp_tool_names(&mcp_manager);
    tools.extend(mcp_tools);

    // Get MCP server info for init event
    let mcp_servers = get_mcp_server_info(&mcp_manager);

    writer.write_real_response_with_mcp(
        &response,
        &session_ctx.session_id.to_string(),
        tools,
        mcp_servers,
    )?;

    // Write queue-operation for print mode (-p) unless persistence is disabled
    // (state_writer already created above)
    if cli.print && !cli.no_session_persistence {
        if let Some(ref writer) = state_writer {
            writer.read().write_queue_operation()?;
        }
    }

    // Record the turn to state directory
    let response_text = response.text().to_string();

    if tool_calls.is_empty() {
        // Simple turn without tool calls
        if let Some(ref writer) = state_writer {
            writer.write().record_turn(&prompt, &response_text)?;
        }
    } else {
        // Turn with tool calls - use granular recording if persistence enabled
        // 1. Record user message (if persistence enabled)
        let user_uuid = if let Some(ref writer) = state_writer {
            Some(writer.write().record_user_message(&prompt)?)
        } else {
            None
        };

        // 2. Record initial assistant text (if any and persistence enabled)
        if !response_text.is_empty() {
            if let (Some(ref writer), Some(ref uuid)) = (&state_writer, &user_uuid) {
                writer
                    .write()
                    .record_assistant_response(uuid, &response_text)?;
            }
        }

        // Determine execution mode (CLI flag overrides scenario config)
        let execution_mode = cli
            .tool_mode
            .clone()
            .map(ToolExecutionMode::from)
            .or_else(|| {
                scenario
                    .as_ref()
                    .and_then(|s| s.config().tool_execution.as_ref())
                    .map(|te| te.mode.clone())
            })
            .unwrap_or_default();

        if execution_mode != ToolExecutionMode::Disabled {
            // Create execution context
            let mut ctx = ExecutionContext::default();
            if let Some(ref cwd) = cli.cwd {
                ctx = ctx.with_cwd(cwd);
            }

            // Create executor with state writer for stateful tools and MCP support
            let executor = create_executor_with_mcp(
                execution_mode,
                mcp_manager.as_ref().map(Arc::clone),
                state_writer.as_ref().map(Arc::clone),
            );

            // Execute each tool call and write results
            for (i, call) in tool_calls.iter().enumerate() {
                let tool_use_id = format!("toolu_{:08x}", i);

                // 3. Record assistant message with tool_use block (if persistence enabled)
                let assistant_uuid =
                    if let (Some(ref writer), Some(ref uuid)) = (&state_writer, &user_uuid) {
                        let tool_use_block = claudeless::state::ContentBlock::ToolUse {
                            id: tool_use_id.clone(),
                            name: call.tool.clone(),
                            input: call.input.clone(),
                        };
                        Some(
                            writer
                                .write()
                                .record_assistant_tool_use(uuid, vec![tool_use_block])?,
                        )
                    } else {
                        None
                    };

                // 4. Execute tool
                let result = executor.execute(call, &tool_use_id, &ctx);
                writer.write_tool_result(&result)?;

                // 5. Record tool result to JSONL (if persistence enabled)
                if let (Some(ref writer), Some(ref asst_uuid)) = (&state_writer, &assistant_uuid) {
                    let result_content = result.text().unwrap_or("");
                    let tool_use_result = result.tool_use_result().unwrap_or(serde_json::json!({}));
                    writer.write().record_tool_result(
                        &tool_use_id,
                        result_content,
                        asst_uuid,
                        tool_use_result,
                    )?;
                }
            }

            // 6. Record final assistant response after tool execution (if persistence enabled)
            // Real Claude writes a final message summarizing the tool results
            if let (Some(ref writer), Some(ref uuid)) = (&state_writer, &user_uuid) {
                let final_response =
                    "Done! The requested operation has been completed successfully.";
                writer
                    .write()
                    .record_assistant_response(uuid, final_response)?;
            }
        }
    }

    stdout.flush()?;

    // Shutdown MCP servers gracefully
    if let Some(mgr) = mcp_manager {
        shutdown_mcp_manager(mgr).await;
    }

    Ok(())
}

/// Shutdown MCP manager gracefully.
///
/// Attempts to take exclusive ownership for clean shutdown. Falls back to
/// holding the lock across await if other references exist (safe at exit time).
#[allow(clippy::await_holding_lock)]
async fn shutdown_mcp_manager(mgr: Arc<RwLock<McpManager>>) {
    match Arc::try_unwrap(mgr) {
        Ok(rwlock) => {
            rwlock.into_inner().shutdown().await;
        }
        Err(arc) => {
            // Other references exist; holding lock across await is safe at exit
            arc.write().shutdown().await;
        }
    }
}

/// Extract qualified tool names from MCP manager.
///
/// Returns MCP tools in the `mcp__<server>__<tool>` format used by real Claude CLI.
fn get_mcp_tool_names(mcp_manager: &Option<Arc<RwLock<McpManager>>>) -> Vec<String> {
    match mcp_manager {
        Some(manager) => {
            let guard = manager.read();
            guard
                .tools()
                .iter()
                .map(|tool| tool.qualified_name())
                .collect()
        }
        None => vec![],
    }
}

/// Extract MCP server info for init event output.
///
/// Maps server status to the format expected by real Claude CLI:
/// - Running -> "connected"
/// - Failed -> "failed"
/// - Disconnected -> "disconnected"
/// - Uninitialized servers are excluded
fn get_mcp_server_info(mcp_manager: &Option<Arc<RwLock<McpManager>>>) -> Vec<McpServerInfo> {
    match mcp_manager {
        Some(manager) => {
            let guard = manager.read();
            guard
                .servers()
                .iter()
                .filter_map(|server| {
                    match &server.status {
                        McpServerStatus::Running => Some(McpServerInfo::connected(&server.name)),
                        McpServerStatus::Failed(_) => Some(McpServerInfo::failed(&server.name)),
                        McpServerStatus::Disconnected => {
                            Some(McpServerInfo::disconnected(&server.name))
                        }
                        McpServerStatus::Uninitialized => None, // Not included in output
                    }
                })
                .collect()
        }
        None => vec![],
    }
}

/// Load and initialize MCP servers from CLI flags.
async fn load_mcp_configs(
    cli: &Cli,
) -> Result<Option<Arc<RwLock<McpManager>>>, Box<dyn std::error::Error>> {
    if cli.mcp_config.is_empty() {
        return Ok(None);
    }

    // Load config files
    let mut configs = Vec::new();
    for config_input in &cli.mcp_config {
        match load_mcp_config(config_input) {
            Ok(config) => configs.push(config),
            Err(e) => {
                print_error(format_args!("loading MCP config: {}", e));
                std::process::exit(1);
            }
        }
    }

    let merged = McpConfig::merge(configs);
    let mut manager = McpManager::from_config(&merged);

    if cli.mcp_debug {
        print_mcp(format_args!(
            "Loading {} server(s): {:?}",
            manager.server_count(),
            manager.server_names()
        ));
    }

    // Initialize servers (spawn processes, discover tools)
    let results = manager.initialize(cli.mcp_debug).await;

    // Handle initialization results
    for (name, result) in &results {
        match result {
            Ok(()) => {
                if cli.mcp_debug {
                    if let Some(server) = manager.get_server(name) {
                        print_mcp(format_args!(
                            "Server '{}' started with {} tool(s): {:?}",
                            name,
                            server.tools.len(),
                            server.tool_names()
                        ));
                    }
                }
            }
            Err(e) => {
                if cli.strict_mcp_config {
                    print_mcp_error(format_args!("Server '{}' failed to start: {}", name, e));
                    std::process::exit(1);
                } else if cli.mcp_debug {
                    print_mcp_warning(format_args!("Server '{}' failed to start: {}", name, e));
                }
            }
        }
    }

    // Check if any servers are running
    if manager.running_server_count() == 0 && cli.mcp_debug {
        print_mcp("No servers running");
    }

    Ok(Some(Arc::new(RwLock::new(manager))))
}

/// Load settings from all sources with correct precedence.
///
/// Loads settings files and CLI-provided settings:
/// 1. Global (~/.claude/settings.json) - lowest priority
/// 2. Project (.claude/settings.json)
/// 3. Local (.claude/settings.local.json)
/// 4. CLI --settings flags (in order specified) - highest priority
fn load_settings(cli: &Cli) -> ClaudeSettings {
    let working_dir = cli
        .cwd
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Resolve state directory (CLAUDELESS_STATE_DIR or CLAUDE_LOCAL_STATE_DIR or temp)
    let state_dir = StateDirectory::resolve().ok();

    let settings = if let Some(ref dir) = state_dir {
        let paths = SettingsPaths::resolve(dir.root(), &working_dir);
        let loader = SettingsLoader::new(paths);
        loader.load_with_overrides(&cli.settings)
    } else {
        // No state directory available, just load from CLI settings
        let paths = SettingsPaths::project_only(&working_dir);
        let loader = SettingsLoader::new(paths);
        loader.load_with_overrides(&cli.settings)
    };

    settings
}

/// Run in TUI mode with MCP support
async fn run_tui_mode(
    cli: &Cli,
    allow_bypass_permissions: bool,
    mcp_manager: Option<Arc<RwLock<McpManager>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ignore SIGINT so Ctrl+C is captured as a key event rather than killing the process.
    // This is required because raw mode alone doesn't prevent SIGINT generation on macOS/tmux.
    // We register a flag that gets set on SIGINT but we never check it - effectively ignoring the signal.
    #[cfg(unix)]
    {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;
        let flag = Arc::new(AtomicBool::new(false));
        if let Err(e) = signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&flag))
        {
            print_warning(format_args!("Failed to ignore SIGINT: {}", e));
        }
        // Leak the flag so it stays registered for the lifetime of the process
        std::mem::forget(flag);
    }

    // Load scenario if specified
    let scenario = if let Some(ref path) = cli.scenario {
        Scenario::load(Path::new(path))?
    } else {
        // Default scenario
        let config = claudeless::config::ScenarioConfig::default();
        Scenario::from_config(config)?
    };

    // Build session context for state directory
    let session_ctx = SessionContext::build(Some(scenario.config()), cli);

    // Create state writer for JSONL persistence (unless --no-session-persistence)
    let state_writer = if !cli.no_session_persistence {
        StateWriter::new(
            session_ctx.session_id.to_string(),
            &session_ctx.project_path,
            session_ctx.launch_timestamp,
            &session_ctx.model,
            &session_ctx.working_directory,
        )
        .ok()
        .map(|w| Arc::new(RwLock::new(w)))
    } else {
        None
    };

    // Create TUI config from scenario
    let is_tty = std::io::stdout().is_terminal();
    let mut tui_config = TuiConfig::from_scenario(
        scenario.config(),
        Some(&cli.model),
        &cli.permission_mode,
        allow_bypass_permissions,
        cli.claude_version.as_deref(),
        is_tty,
    );
    tui_config.state_writer = state_writer;

    let sessions = SessionManager::new();
    let clock = ClockHandle::system();

    let mut app = TuiApp::new(scenario, sessions, clock, tui_config)?;
    let exit_reason = app.run()?;

    // Print exit message if any (e.g., farewell from /exit)
    if let Some(msg) = app.exit_message() {
        println!("{}", msg);
    }

    // Shutdown MCP servers before exiting
    if let Some(mgr) = mcp_manager {
        shutdown_mcp_manager(mgr).await;
    }

    match exit_reason {
        ExitReason::Interrupted => std::process::exit(130),
        ExitReason::Error(msg) => {
            print_error(&msg);
            std::process::exit(1);
        }
        ExitReason::UserQuit | ExitReason::Completed | ExitReason::Suspended => Ok(()),
    }
}
