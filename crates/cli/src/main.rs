// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Claude CLI Simulator binary entry point.

use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use claudeless::config::ToolExecutionMode;
use claudeless::session::SessionContext;
use claudeless::state::session::SessionManager;
use claudeless::state::StateWriter;
use claudeless::time::ClockHandle;
use claudeless::tools::builtin::BuiltinExecutor;
use claudeless::tools::{create_executor, ExecutionContext, ToolExecutor};
use claudeless::tui::{ExitReason, TuiApp, TuiConfig};
use claudeless::{
    load_mcp_config, CaptureLog, CapturedArgs, CapturedOutcome, Cli, FailureExecutor, McpConfig,
    McpManager, OutputWriter, PermissionBypass, ResponseSpec, Scenario,
};
use parking_lot::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Validate permission bypass configuration
    let bypass = PermissionBypass::new(
        cli.allow_dangerously_skip_permissions,
        cli.dangerously_skip_permissions,
    );
    if bypass.is_not_allowed() {
        eprintln!("{}", PermissionBypass::error_message());
        std::process::exit(1);
    }

    // Load MCP configuration if specified
    let _mcp_manager = load_mcp_configs(&cli)?;

    // Check for TUI mode first
    if cli.should_use_tui() {
        return run_tui_mode(&cli, bypass.is_active());
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

        FailureExecutor::execute(&spec, &mut stderr).await?;
        return Ok(());
    }

    // Apply delay if configured
    if let Some(delay_ms) = cli.delay_ms {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    // Load scenario if specified
    let mut scenario = if let Some(ref path) = cli.scenario {
        Some(Scenario::load(Path::new(path))?)
    } else {
        None
    };

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

                FailureExecutor::execute(failure_spec, &mut stderr).await?;
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
    let response_delay = match &response {
        Some(ResponseSpec::Detailed { delay_ms, .. }) => *delay_ms,
        _ => None,
    };

    if let Some(delay) = response_delay {
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }

    // Record outcome
    if let Some(ref log) = capture {
        let outcome = match &response {
            Some(spec) => CapturedOutcome::Response {
                text: match spec {
                    ResponseSpec::Simple(s) => s.clone(),
                    ResponseSpec::Detailed { text, .. } => text.clone(),
                },
                matched_rule,
                delay_ms: response_delay.unwrap_or(0),
            },
            None => CapturedOutcome::NoMatch {
                used_default: false,
            },
        };
        log.record(captured_args, outcome);
    }

    // Build session context for state directory (needed for session_id in output)
    let session_ctx = SessionContext::build(scenario.as_ref().map(|s| s.config()), &cli);

    // Write output
    let mut stdout = io::stdout();
    let response = response.unwrap_or(ResponseSpec::Simple(String::new()));
    let tool_calls = match &response {
        ResponseSpec::Detailed { tool_calls, .. } => tool_calls.clone(),
        _ => vec![],
    };

    // Use real Claude format (result wrapper for JSON, system init + result for stream-JSON)
    let mut writer = OutputWriter::new(&mut stdout, cli.output_format.clone(), cli.model.clone());
    let tools: Vec<String> = cli.allowed_tools.clone();
    writer.write_real_response(&response, &session_ctx.session_id.to_string(), tools)?;

    // Create state writer for recording turns and handling stateful tools
    let state_writer = StateWriter::new(
        session_ctx.session_id.to_string(),
        &session_ctx.project_path,
        session_ctx.launch_timestamp,
        &session_ctx.model,
        &session_ctx.working_directory,
    )?;
    let state_writer = Arc::new(RwLock::new(state_writer));

    // Write queue-operation for print mode (-p)
    if cli.print {
        state_writer.read().write_queue_operation()?;
    }

    // Record the turn to state directory
    let response_text = match &response {
        ResponseSpec::Simple(s) => s.clone(),
        ResponseSpec::Detailed { text, .. } => text.clone(),
    };

    if tool_calls.is_empty() {
        // Simple turn without tool calls
        state_writer.write().record_turn(&prompt, &response_text)?;
    } else {
        // Turn with tool calls - use granular recording
        // 1. Record user message
        let user_uuid = state_writer.write().record_user_message(&prompt)?;

        // 2. Record initial assistant text (if any)
        if !response_text.is_empty() {
            state_writer
                .write()
                .record_assistant_response(&user_uuid, &response_text)?;
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

            // Create executor with state writer for stateful tools
            let executor: Box<dyn ToolExecutor> = match execution_mode {
                ToolExecutionMode::Live => {
                    Box::new(BuiltinExecutor::new().with_state_writer(Arc::clone(&state_writer)))
                }
                _ => create_executor(execution_mode),
            };

            // Execute each tool call and write results
            for (i, call) in tool_calls.iter().enumerate() {
                let tool_use_id = format!("toolu_{:08x}", i);

                // 3. Record assistant message with tool_use block
                let tool_use_block = claudeless::state::ContentBlock::ToolUse {
                    id: tool_use_id.clone(),
                    name: call.tool.clone(),
                    input: call.input.clone(),
                };
                let assistant_uuid = state_writer
                    .write()
                    .record_assistant_tool_use(&user_uuid, vec![tool_use_block])?;

                // 4. Execute tool
                let result = executor.execute(call, &tool_use_id, &ctx);
                writer.write_tool_result(&result)?;

                // 5. Record tool result to JSONL
                let result_content = result.text().unwrap_or("");
                let tool_use_result = result.tool_use_result().unwrap_or(serde_json::json!({}));
                state_writer.write().record_tool_result(
                    &tool_use_id,
                    result_content,
                    &assistant_uuid,
                    tool_use_result,
                )?;
            }

            // 6. Record final assistant response after tool execution
            // Real Claude writes a final message summarizing the tool results
            let final_response = "Done! The requested operation has been completed successfully.";
            state_writer
                .write()
                .record_assistant_response(&user_uuid, final_response)?;
        }
    }

    stdout.flush()?;

    Ok(())
}

/// Load MCP configurations from CLI flags
fn load_mcp_configs(cli: &Cli) -> Result<McpManager, Box<dyn std::error::Error>> {
    if cli.mcp_config.is_empty() {
        return Ok(McpManager::new());
    }

    let mut configs = Vec::new();
    for config_input in &cli.mcp_config {
        match load_mcp_config(config_input) {
            Ok(config) => configs.push(config),
            Err(e) => {
                eprintln!("Error loading MCP config: {}", e);
                std::process::exit(1);
            }
        }
    }

    let merged = McpConfig::merge(configs);
    let manager = McpManager::from_config(&merged);

    if cli.mcp_debug {
        eprintln!(
            "MCP: Loaded {} server(s): {:?}",
            manager.server_count(),
            manager.server_names()
        );
    }

    Ok(manager)
}

/// Run in TUI mode
fn run_tui_mode(
    cli: &Cli,
    allow_bypass_permissions: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ignore SIGINT so Ctrl+C is captured as a key event rather than killing the process.
    // This is required because raw mode alone doesn't prevent SIGINT generation on macOS/tmux.
    // SAFETY: SIG_IGN is a well-defined, constant signal handler provided by the OS that
    // simply ignores the signal. This is necessary to allow raw mode terminal input to
    // capture Ctrl+C as a key event.
    #[cfg(unix)]
    unsafe {
        use nix::sys::signal::{signal, SigHandler, Signal};
        if let Err(e) = signal(Signal::SIGINT, SigHandler::SigIgn) {
            eprintln!("Warning: Failed to ignore SIGINT: {}", e);
        }
    }

    // Load scenario if specified
    let scenario = if let Some(ref path) = cli.scenario {
        Scenario::load(Path::new(path))?
    } else {
        // Default scenario
        let config = claudeless::config::ScenarioConfig::default();
        Scenario::from_config(config)?
    };

    // Create TUI config from scenario
    let is_tty = std::io::stdout().is_terminal();
    let tui_config = TuiConfig::from_scenario(
        scenario.config(),
        Some(&cli.model),
        &cli.permission_mode,
        allow_bypass_permissions,
        cli.claude_version.as_deref(),
        is_tty,
    );

    let sessions = SessionManager::new();
    let clock = ClockHandle::system();

    let mut app = TuiApp::new(scenario, sessions, clock, tui_config)?;
    let exit_reason = app.run()?;

    // Print exit message if any (e.g., farewell from /exit)
    if let Some(msg) = app.exit_message() {
        println!("{}", msg);
    }

    match exit_reason {
        ExitReason::Interrupted => std::process::exit(130),
        ExitReason::Error(msg) => {
            eprintln!("Error: {}", msg);
            std::process::exit(1);
        }
        ExitReason::UserQuit | ExitReason::Completed => Ok(()),
    }
}
