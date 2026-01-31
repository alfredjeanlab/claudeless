// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Claude CLI Simulator binary entry point.

use std::io::IsTerminal;
use std::sync::Arc;

use clap::Parser;

use claudeless::cli::Cli;
use claudeless::hooks::load_hooks;
use claudeless::output::{print_error, print_warning};
use claudeless::permission::PermissionBypass;
use claudeless::runtime::{RuntimeBuildError, RuntimeBuilder};
use claudeless::scenario::Scenario;
use claudeless::state::session::SessionManager;
use claudeless::time::ClockHandle;
use claudeless::tui::{ExitReason, TuiApp, TuiConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

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

/// Run in TUI mode.
async fn run_tui_mode(
    runtime: claudeless::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;

    // Ignore SIGINT so Ctrl+C is captured as a key event rather than killing the process.
    #[cfg(unix)]
    {
        use std::sync::atomic::AtomicBool;
        let flag = Arc::new(AtomicBool::new(false));
        if let Err(e) = signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&flag))
        {
            print_warning(format_args!("Failed to ignore SIGINT: {}", e));
        }
        // Leak the flag so it stays registered for the lifetime of the process
        std::mem::forget(flag);
    }

    let cli = runtime.cli();

    // Check permission bypass
    let bypass = PermissionBypass::new(
        cli.permissions.allow_dangerously_skip_permissions,
        cli.permissions.dangerously_skip_permissions,
    );

    // Load scenario for TUI (separate from Runtime's scenario for fallback)
    let scenario = if let Some(ref path) = cli.simulator.scenario {
        Scenario::load(Path::new(path))?
    } else {
        let config = claudeless::config::ScenarioConfig::default();
        Scenario::from_config(config)?
    };

    // Load settings for hooks
    let settings = load_settings(cli);

    // Create TUI config from scenario
    let is_tty = std::io::stdout().is_terminal();
    let mut tui_config = TuiConfig::from_scenario(
        scenario.config(),
        Some(&cli.model),
        &cli.permissions.permission_mode,
        bypass.is_active(),
        cli.simulator.claude_version.as_deref(),
        is_tty,
    );

    // Use state writer and hook executor from runtime
    tui_config.state_writer = runtime.state_writer();
    tui_config.hook_executor = load_hooks(&settings).ok();

    let sessions = SessionManager::new();
    let clock = ClockHandle::system();

    // Create TUI app with runtime for shared execution
    let mut app = TuiApp::new_with_runtime(scenario, sessions, clock, tui_config, runtime)?;
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

/// Load settings from all sources with correct precedence.
fn load_settings(cli: &Cli) -> claudeless::state::ClaudeSettings {
    use claudeless::state::{SettingsLoader, SettingsPaths, StateDirectory};
    use std::path::PathBuf;

    let working_dir = cli
        .cwd
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Resolve state directory
    let state_dir = StateDirectory::resolve().ok();

    if let Some(ref dir) = state_dir {
        let paths = SettingsPaths::resolve(dir.root(), &working_dir);
        let loader = SettingsLoader::new(paths);
        loader.load_with_overrides(&cli.settings)
    } else {
        // No state directory available, just load from CLI settings
        let paths = SettingsPaths::project_only(&working_dir);
        let loader = SettingsLoader::new(paths);
        loader.load_with_overrides(&cli.settings)
    }
}
