// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! RuntimeBuilder for constructing Runtime with fluent API.

use std::path::Path;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::cli::Cli;
use crate::config::ResolvedTimeouts;
use crate::hooks::load_hooks;
use crate::mcp::{load_mcp_config, McpConfig, McpManager};
use crate::output::{print_mcp, print_mcp_warning};
use crate::permission::PermissionBypass;
use crate::scenario::Scenario;
use crate::state::io::JsonLoad;
use crate::state::{
    ClaudeSettings, SessionsIndex, SettingsLoader, SettingsPaths, StateDirectory, StateWriter,
};
use crate::tools::create_executor_with_mcp_and_permissions;

use super::core::Runtime;
use super::RuntimeContext;

/// Builder for constructing Runtime instances.
///
/// Provides a fluent API for initializing runtime components:
/// ```ignore
/// let runtime = RuntimeBuilder::new(cli)
///     .with_scenario(path)?
///     .build()
///     .await?;
/// ```
pub struct RuntimeBuilder {
    cli: Cli,
    scenario: Option<Scenario>,
    mcp_manager: Option<Arc<RwLock<McpManager>>>,
    settings: Option<ClaudeSettings>,
}

impl RuntimeBuilder {
    /// Create a new builder from CLI args.
    ///
    /// Validates CLI arguments during construction.
    pub fn new(cli: Cli) -> Result<Self, RuntimeBuildError> {
        // Validate all CLI arguments
        if let Err(msg) = cli.validate() {
            return Err(RuntimeBuildError::Validation(msg.to_string()));
        }

        // Note: --dangerously-skip-permissions without --allow-dangerously-skip-permissions
        // is handled by the TUI with a confirmation dialog (not a build error).

        Ok(Self {
            cli,
            scenario: None,
            mcp_manager: None,
            settings: None,
        })
    }

    /// Load scenario from file path.
    pub fn with_scenario(mut self, path: &Path) -> Result<Self, RuntimeBuildError> {
        self.scenario = Some(Scenario::load(path)?);
        Ok(self)
    }

    /// Load scenario from CLI args if specified.
    pub fn with_scenario_from_cli(mut self) -> Result<Self, RuntimeBuildError> {
        if let Some(ref path) = self.cli.simulator.scenario {
            self.scenario = Some(Scenario::load(Path::new(path))?);
        }
        Ok(self)
    }

    /// Initialize MCP servers from config files.
    pub async fn with_mcp(mut self, configs: Vec<McpConfig>) -> Result<Self, RuntimeBuildError> {
        if configs.is_empty() {
            return Ok(self);
        }

        let merged = McpConfig::merge(configs);
        let mut manager = McpManager::from_config(&merged);

        if self.cli.mcp.mcp_debug {
            print_mcp(format_args!(
                "Loading {} server(s): {:?}",
                manager.server_count(),
                manager.server_names()
            ));
        }

        // Initialize servers
        let results = manager.initialize(self.cli.mcp.mcp_debug).await;

        // Handle initialization results
        for (name, result) in &results {
            match result {
                Ok(()) => {
                    if self.cli.mcp.mcp_debug {
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
                    if self.cli.mcp.strict_mcp_config {
                        return Err(RuntimeBuildError::McpServer {
                            name: name.clone(),
                            error: e.to_string(),
                        });
                    } else if self.cli.mcp.mcp_debug {
                        print_mcp_warning(format_args!("Server '{}' failed to start: {}", name, e));
                    }
                }
            }
        }

        if manager.running_server_count() == 0 && self.cli.mcp.mcp_debug {
            print_mcp("No servers running");
        }

        self.mcp_manager = Some(Arc::new(RwLock::new(manager)));
        Ok(self)
    }

    /// Initialize MCP servers from CLI args if specified.
    pub async fn with_mcp_from_cli(self) -> Result<Self, RuntimeBuildError> {
        if self.cli.mcp.mcp_config.is_empty() {
            return Ok(self);
        }

        // Load config files
        let mut configs = Vec::new();
        for config_input in &self.cli.mcp.mcp_config {
            match load_mcp_config(config_input) {
                Ok(config) => configs.push(config),
                Err(e) => {
                    return Err(RuntimeBuildError::McpConfig(e.to_string()));
                }
            }
        }

        self.with_mcp(configs).await
    }

    /// Load settings from files.
    pub fn with_settings(mut self) -> Self {
        self.settings = Some(load_settings(&self.cli));
        self
    }

    /// Build the Runtime.
    pub async fn build(self) -> Result<Runtime, RuntimeBuildError> {
        // Load settings if not already loaded
        let settings = self.settings.unwrap_or_else(|| load_settings(&self.cli));

        // Validate resume session exists
        if let Some(ref resume_id) = self.cli.session.resume {
            let state_dir = StateDirectory::resolve()
                .map_err(|e| RuntimeBuildError::Validation(e.to_string()))?;

            let working_dir = self
                .cli
                .cwd
                .as_ref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            let index_path = state_dir
                .project_dir(&working_dir)
                .join("sessions-index.json");

            if index_path.exists() {
                let index = SessionsIndex::load(&index_path)
                    .map_err(|e| RuntimeBuildError::Validation(e.to_string()))?;

                if index.get(resume_id).is_none() {
                    return Err(RuntimeBuildError::SessionNotFound(resume_id.clone()));
                }
            } else {
                return Err(RuntimeBuildError::SessionNotFound(resume_id.clone()));
            }
        }

        // Build runtime context
        let runtime_ctx =
            RuntimeContext::build(self.scenario.as_ref().map(|s| s.config()), &self.cli);

        // Create state writer (unless --no-session-persistence)
        let state_writer = if !self.cli.session.no_session_persistence {
            let session_id = runtime_ctx.session_id.to_string();

            // Load existing message count if resuming
            let initial_message_count = if self.cli.session.resume.is_some() {
                StateWriter::load_message_count_from_index(&runtime_ctx.project_path, &session_id)
                    .unwrap_or(0)
            } else {
                0
            };

            if initial_message_count > 0 {
                // Resuming with existing messages
                StateWriter::new_with_count(
                    session_id,
                    &runtime_ctx.project_path,
                    runtime_ctx.launch_timestamp,
                    &runtime_ctx.model,
                    &runtime_ctx.working_directory,
                    initial_message_count,
                )
                .ok()
                .map(|w| Arc::new(RwLock::new(w)))
            } else {
                // New session or no messages yet
                StateWriter::new(
                    session_id,
                    &runtime_ctx.project_path,
                    runtime_ctx.launch_timestamp,
                    &runtime_ctx.model,
                    &runtime_ctx.working_directory,
                )
                .ok()
                .map(|w| {
                    // Write initial session state (JSONL, sessions-index, empty todo)
                    let _ = w.initialize_session();
                    Arc::new(RwLock::new(w))
                })
            }
        } else {
            None
        };

        // Load hook executor from settings, injecting common context fields
        let hook_executor = load_hooks(&settings).ok().map(|executor| {
            let cwd = Some(runtime_ctx.working_directory.to_string_lossy().into_owned());
            let transcript_path = state_writer.as_ref().map(|sw| {
                sw.read()
                    .session_jsonl_path()
                    .to_string_lossy()
                    .into_owned()
            });
            let permission_mode = serde_json::to_value(&runtime_ctx.permission_mode)
                .ok()
                .and_then(|v| v.as_str().map(String::from));
            executor.with_context(cwd, transcript_path, permission_mode)
        });

        // Get execution mode from scenario (defaults to Live)
        let execution_mode = self
            .scenario
            .as_ref()
            .and_then(|s| s.config().tool_execution.as_ref())
            .map(|te| te.mode.clone())
            .unwrap_or_default();

        // Build permission checker from runtime context
        let bypass = PermissionBypass::new(
            self.cli.permissions.allow_dangerously_skip_permissions,
            self.cli.permissions.dangerously_skip_permissions,
        );
        let scenario_tools = self
            .scenario
            .as_ref()
            .and_then(|s| s.config().tool_execution.as_ref())
            .map(|te| te.tools.clone())
            .unwrap_or_default();
        let checker = runtime_ctx.permission_checker_with_overrides(bypass, scenario_tools);

        // Create executor with MCP support and permission checking
        let executor = create_executor_with_mcp_and_permissions(
            execution_mode,
            self.mcp_manager.as_ref().map(Arc::clone),
            state_writer.as_ref().map(Arc::clone),
            checker,
        );

        // Resolve timeouts
        let timeouts = ResolvedTimeouts::resolve(
            self.scenario
                .as_ref()
                .and_then(|s| s.config().timing.timeouts.as_ref()),
        );

        Ok(Runtime::new(
            runtime_ctx,
            self.scenario,
            executor,
            state_writer,
            hook_executor,
            self.mcp_manager,
            self.cli,
            timeouts,
        ))
    }

    /// Build with default initialization from CLI args.
    ///
    /// Convenience method that loads scenario and MCP from CLI.
    pub async fn build_from_cli(self) -> Result<Runtime, RuntimeBuildError> {
        self.with_scenario_from_cli()?
            .with_mcp_from_cli()
            .await?
            .with_settings()
            .build()
            .await
    }
}

/// Load settings from all sources with correct precedence.
fn load_settings(cli: &Cli) -> ClaudeSettings {
    let working_dir = cli
        .cwd
        .as_ref()
        .map(std::path::PathBuf::from)
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

/// Errors that can occur when building a Runtime.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeBuildError {
    #[error("CLI validation failed: {0}")]
    Validation(String),

    #[error("Permission bypass not allowed without --allow-dangerously-skip-permissions")]
    PermissionBypass,

    #[error("Failed to load scenario: {0}")]
    Scenario(#[from] crate::scenario::ScenarioError),

    #[error("Failed to load MCP config: {0}")]
    McpConfig(String),

    #[error("MCP server '{name}' failed to start: {error}")]
    McpServer { name: String, error: String },

    #[error("Session not found: {0}")]
    SessionNotFound(String),
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
