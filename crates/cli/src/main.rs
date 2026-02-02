// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Claude CLI Simulator binary entry point.

use std::io::IsTerminal;

use clap::Parser;

use claudeless::cli::{Cli, Commands, McpCommands, PluginCommands};
use claudeless::help;
use claudeless::output::print_error;
use claudeless::permission::PermissionBypass;
use claudeless::runtime::{Runtime, RuntimeBuildError, RuntimeBuilder};
use claudeless::state::session::SessionManager;
use claudeless::time::ClockHandle;
use claudeless::tui::{ExitReason, TuiApp, TuiConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Handle help for subcommands
    if let Some(ref cmd) = cli.command {
        if cmd.wants_help() {
            let text = render_subcommand_help(cmd);
            print!("{}", text);
            return Ok(());
        }
    }

    // Handle top-level help
    if cli.help {
        let mut text = help::render_main_help();
        text.push('\n');
        text.push_str(&help::claudeless_options_section());
        print!("{}", text);
        return Ok(());
    }

    // Handle version
    if cli.version {
        if let Some(ref v) = cli.simulator.claude_version {
            println!("{v} (Claude Code)");
        } else {
            println!("claudeless {}", env!("CARGO_PKG_VERSION"));
        }
        return Ok(());
    }

    // Build runtime using RuntimeBuilder
    let runtime = match RuntimeBuilder::new(cli.clone()) {
        Ok(builder) => match builder.build_from_cli().await {
            Ok(runtime) => runtime,
            Err(e) => {
                print_error(e.to_string());
                std::process::exit(1);
            }
        },
        Err(RuntimeBuildError::Validation(msg)) => {
            print_error(msg);
            std::process::exit(1);
        }
        Err(RuntimeBuildError::PermissionBypass) => {
            eprintln!("{}", PermissionBypass::error_message());
            std::process::exit(1);
        }
        Err(e) => {
            print_error(e.to_string());
            std::process::exit(1);
        }
    };

    // Branch based on mode
    if runtime.should_use_tui() {
        run_tui_mode(runtime).await
    } else {
        // Move runtime to allow mutation
        let mut runtime = runtime;
        runtime.execute_print_mode().await
    }
}

/// Render help text for a subcommand.
fn render_subcommand_help(cmd: &Commands) -> String {
    match cmd {
        Commands::Doctor { .. } => help::render_doctor_help(),
        Commands::Install { .. } => help::render_install_help(),
        Commands::Mcp { command, .. } => match command {
            Some(McpCommands::Add { .. }) => help::render_mcp_add_help(),
            Some(McpCommands::Serve { .. }) => help::render_mcp_serve_help(),
            _ => help::render_mcp_help(),
        },
        Commands::Plugin { command, .. } => match command {
            Some(PluginCommands::Marketplace { .. }) => help::render_plugin_marketplace_help(),
            _ => help::render_plugin_help(),
        },
        Commands::SetupToken { .. } => help::render_setup_token_help(),
        Commands::Update { .. } => help::render_update_help(),
    }
}

/// Run in TUI mode.
async fn run_tui_mode(runtime: Runtime) -> Result<(), Box<dyn std::error::Error>> {
    // Check permission bypass
    let bypass = PermissionBypass::new(
        runtime.cli().permissions.allow_dangerously_skip_permissions,
        runtime.cli().permissions.dangerously_skip_permissions,
    );

    // Create TUI config from runtime's scenario config
    let is_tty = std::io::stdout().is_terminal();
    let tui_config = TuiConfig::from_runtime(
        &runtime,
        bypass.is_active() || bypass.is_not_allowed(),
        bypass.is_not_allowed(),
        is_tty,
    );

    let mut sessions = SessionManager::new();

    // Check for resume flag and load existing session
    if let Some(ref resume_id) = runtime.cli().session.resume {
        // Resume existing session (already validated in builder)
        sessions.resume(resume_id);
    }

    let clock = ClockHandle::system();

    // Create TUI app with runtime for shared execution
    let mut app = TuiApp::new(sessions, clock, tui_config, runtime)?;
    let exit_reason = app.run()?;

    // Print exit message if any (e.g., farewell from /exit)
    if let Some(msg) = app.exit_message() {
        println!("{}", msg);
    }

    // Shutdown MCP servers before exiting
    if let Some(runtime) = app.take_runtime() {
        runtime.shutdown_mcp().await;
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
