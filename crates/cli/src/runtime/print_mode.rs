// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Print mode execution for non-interactive CLI use.

use std::io::{self, Write};

use crate::capture::{CapturedArgs, CapturedOutcome};
use crate::failure::FailureExecutor;
use crate::output::{McpServerInfo, OutputWriter};
use crate::runtime::TurnResult;

use super::Runtime;

impl Runtime {
    /// Execute print mode (non-interactive, single prompt).
    ///
    /// Processes the prompt from CLI args and writes to stdout.
    pub async fn execute_print_mode(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Require a prompt in print mode
        let prompt = match &self.cli.prompt {
            Some(p) => p.clone(),
            None => {
                return Err("Input must be provided either through stdin or as a prompt argument when using --print".into());
            }
        };

        // Build captured args for logging
        let captured_args = CapturedArgs {
            prompt: self.cli.prompt.clone(),
            model: self.cli.model.clone(),
            output_format: format!("{:?}", self.cli.output.output_format).to_lowercase(),
            print_mode: self.cli.print,
            continue_conversation: self.cli.session.continue_conversation,
            resume: self.cli.session.resume.clone(),
            allowed_tools: self.cli.allowed_tools.clone(),
            cwd: self.cli.cwd.clone(),
        };

        // Handle failure injection from CLI flag
        if let Some(ref mode) = self.cli.simulator.failure {
            return self.handle_failure_injection(mode, &captured_args).await;
        }

        // Write queue-operation at session start (before any state recording)
        self.write_queue_operation()?;

        // Execute the response loop
        self.execute_response_loop(&prompt, captured_args).await?;

        // Shutdown MCP servers gracefully
        self.shutdown_mcp().await;

        Ok(())
    }

    /// Handle failure injection from CLI flag.
    async fn handle_failure_injection(
        &self,
        mode: &crate::cli::FailureMode,
        captured_args: &CapturedArgs,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let spec = FailureExecutor::from_mode(mode);
        let mut stderr = io::stderr();

        if let Some(ref log) = self.capture {
            log.record(
                captured_args.clone(),
                CapturedOutcome::Failure {
                    failure_type: format!("{:?}", mode),
                    message: "Injected failure".to_string(),
                },
            );
        }

        // Write queue-operation before recording error
        self.write_queue_operation()?;

        // Record error to session JSONL before exiting
        FailureExecutor::execute_with_session(&spec, &mut stderr, self.state.as_ref()).await?;
        Ok(())
    }

    /// Execute the main response loop using Runtime::execute().
    async fn execute_response_loop(
        &mut self,
        initial_prompt: &str,
        captured_args: CapturedArgs,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut current_prompt = initial_prompt.to_string();

        loop {
            // Execute turn (handles failure detection and JSONL recording)
            let result = match self.execute(&current_prompt).await {
                Ok(result) => result,
                Err(failure_spec) => {
                    // Failure detected - JSONL already recorded by execute()
                    // Record to capture log
                    self.record_failure_capture(&captured_args, &failure_spec);
                    // Write to stderr and exit
                    let mut stderr = io::stderr();
                    FailureExecutor::execute(&failure_spec, &mut stderr).await?;
                    return Err("Scenario failure triggered".into());
                }
            };

            // Record capture (only on first iteration, not hook continuations)
            if !result.is_hook_continuation {
                self.record_capture(&captured_args, &result);
            }

            // Write output
            self.write_turn_result(&result)?;

            // Handle hook continuation
            match result.hook_continuation {
                Some(continuation) => current_prompt = continuation,
                None => break,
            }
        }

        Ok(())
    }

    /// Record failure to capture log.
    fn record_failure_capture(
        &self,
        captured_args: &CapturedArgs,
        failure_spec: &crate::config::FailureSpec,
    ) {
        if let Some(ref log) = self.capture {
            log.record(
                captured_args.clone(),
                CapturedOutcome::Failure {
                    failure_type: format!("{:?}", failure_spec),
                    message: "Scenario failure".to_string(),
                },
            );
        }
    }

    /// Record outcome to capture log.
    fn record_capture(&self, captured_args: &CapturedArgs, result: &TurnResult) {
        if let Some(ref log) = self.capture {
            let outcome = if result.response_text().is_empty() {
                CapturedOutcome::NoMatch {
                    used_default: false,
                }
            } else {
                CapturedOutcome::Response {
                    text: result.response_text().to_string(),
                    matched_rule: Some("matched".to_string()),
                    delay_ms: result.response.delay_ms().unwrap_or(0),
                }
            };
            log.record(captured_args.clone(), outcome);
        }
    }

    /// Write a turn result to stdout.
    fn write_turn_result(&self, result: &TurnResult) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();

        // Use real Claude format
        let mut writer = OutputWriter::new(
            &mut stdout,
            self.cli.output.output_format.clone(),
            self.cli.model.clone(),
        );

        // Combine builtin tools with MCP tools
        let mut tools: Vec<String> = self.cli.allowed_tools.clone();
        tools.extend(self.get_mcp_tool_names());

        // Get MCP server info for init event
        let mcp_servers = self.get_mcp_server_info();

        writer.write_real_response_with_mcp(
            &result.response,
            &self.context.session_id.to_string(),
            tools,
            mcp_servers,
        )?;

        // Write tool results
        for tool_result in &result.tool_results {
            writer.write_tool_result(tool_result)?;
        }

        stdout.flush()?;
        Ok(())
    }

    /// Write queue-operation for print mode (unless persistence is disabled).
    fn write_queue_operation(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.cli.print && !self.cli.session.no_session_persistence {
            if let Some(ref writer) = self.state {
                writer.read().write_queue_operation()?;
            }
        }
        Ok(())
    }

    /// Get MCP tool names in qualified format.
    pub(super) fn get_mcp_tool_names(&self) -> Vec<String> {
        match &self.mcp_manager {
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

    /// Get MCP server info for init event.
    pub(super) fn get_mcp_server_info(&self) -> Vec<McpServerInfo> {
        use crate::mcp::McpServerStatus;

        match &self.mcp_manager {
            Some(manager) => {
                let guard = manager.read();
                guard
                    .servers()
                    .iter()
                    .filter_map(|server| match &server.status {
                        McpServerStatus::Running => Some(McpServerInfo::connected(&server.name)),
                        McpServerStatus::Failed(_) => Some(McpServerInfo::failed(&server.name)),
                        McpServerStatus::Disconnected => {
                            Some(McpServerInfo::disconnected(&server.name))
                        }
                        McpServerStatus::Uninitialized => None,
                    })
                    .collect()
            }
            None => vec![],
        }
    }
}
