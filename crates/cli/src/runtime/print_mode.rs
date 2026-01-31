// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Print mode execution for non-interactive CLI use.

use std::io::{self, Write};
use std::time::Duration;

use crate::capture::{CapturedArgs, CapturedOutcome};
use crate::config::ResponseSpec;
use crate::failure::FailureExecutor;
use crate::hooks::{HookEvent, HookMessage, StopHookResponse};
use crate::output::{McpServerInfo, OutputWriter};
use crate::state::ContentBlock;
use crate::tools::ExecutionContext;

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

        // Apply response delay if configured
        if self.timeouts.response_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.timeouts.response_delay_ms)).await;
        }

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
        if self.cli.print {
            if let Some(ref writer) = self.state {
                writer.read().write_queue_operation()?;
            }
        }

        // Record error to session JSONL before exiting
        FailureExecutor::execute_with_session(&spec, &mut stderr, self.state.as_ref()).await?;
        Ok(())
    }

    /// Execute the main response loop.
    async fn execute_response_loop(
        &mut self,
        initial_prompt: &str,
        captured_args: CapturedArgs,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut current_prompt = initial_prompt.to_string();

        'response_loop: loop {
            // Match prompt to get response
            let (response, matched_rule) = self.match_prompt(&current_prompt).await?;

            // Get response delay from spec if detailed
            let response_delay = response.as_ref().and_then(|r| r.delay_ms());
            if let Some(delay) = response_delay {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            // Record outcome (only on first iteration, using original prompt)
            if !self.stop_hook_active {
                if let Some(ref log) = self.capture {
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
                    log.record(captured_args.clone(), outcome);
                }
            }

            // Write output
            let response = response.unwrap_or(ResponseSpec::Simple(String::new()));
            self.write_response(&current_prompt, &response).await?;

            // Fire Stop hook after response completes
            if let Some(ref executor) = self.hook_executor {
                if executor.has_hooks(&HookEvent::Stop) {
                    let stop_msg = HookMessage::stop(
                        self.context.session_id.to_string(),
                        self.stop_hook_active,
                    );
                    if let Ok(responses) = executor.execute(&stop_msg).await {
                        for resp in responses {
                            if let Some(data) = resp.data {
                                if let Ok(stop_resp) =
                                    serde_json::from_value::<StopHookResponse>(data)
                                {
                                    if stop_resp.is_blocked() {
                                        current_prompt = stop_resp
                                            .reason
                                            .unwrap_or_else(|| "continue".to_string());
                                        self.stop_hook_active = true;
                                        continue 'response_loop;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Exit loop - no Stop hook blocked
            break;
        }

        Ok(())
    }

    /// Match a prompt against the scenario.
    async fn match_prompt(
        &mut self,
        prompt: &str,
    ) -> Result<(Option<ResponseSpec>, Option<String>), Box<dyn std::error::Error>> {
        if let Some(ref mut scenario) = self.scenario {
            if let Some(result) = scenario.match_prompt(prompt) {
                // Check for failure in rule
                if let Some(failure_spec) = scenario.get_failure(&result) {
                    let mut stderr = io::stderr();

                    if let Some(ref log) = self.capture {
                        let args = CapturedArgs {
                            prompt: Some(prompt.to_string()),
                            model: self.cli.model.clone(),
                            output_format: format!("{:?}", self.cli.output.output_format)
                                .to_lowercase(),
                            print_mode: self.cli.print,
                            continue_conversation: self.cli.session.continue_conversation,
                            resume: self.cli.session.resume.clone(),
                            allowed_tools: self.cli.allowed_tools.clone(),
                            cwd: self.cli.cwd.clone(),
                        };
                        log.record(
                            args,
                            CapturedOutcome::Failure {
                                failure_type: format!("{:?}", failure_spec),
                                message: "Scenario failure".to_string(),
                            },
                        );
                    }

                    // Write queue-operation before recording error
                    if self.cli.print {
                        if let Some(ref writer) = self.state {
                            writer.read().write_queue_operation()?;
                        }
                    }

                    // Record error to session JSONL before exiting
                    FailureExecutor::execute_with_session(
                        failure_spec,
                        &mut stderr,
                        self.state.as_ref(),
                    )
                    .await?;

                    // Return error to stop execution
                    return Err("Scenario failure triggered".into());
                }

                Ok((
                    scenario.get_response(&result).cloned(),
                    Some("matched".to_string()),
                ))
            } else if let Some(default) = scenario.default_response() {
                Ok((Some(default.clone()), Some("default".to_string())))
            } else {
                Ok((None, None))
            }
        } else {
            // No scenario - use a default response
            Ok((
                Some(ResponseSpec::Simple("Hello! I'm Claudeless!".to_string())),
                None,
            ))
        }
    }

    /// Write response to stdout.
    async fn write_response(
        &mut self,
        prompt: &str,
        response: &ResponseSpec,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        let tool_calls = response.tool_calls().to_vec();

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
            response,
            &self.context.session_id.to_string(),
            tools,
            mcp_servers,
        )?;

        // Write queue-operation for print mode unless persistence is disabled
        if self.cli.print && !self.cli.session.no_session_persistence {
            if let Some(ref writer) = self.state {
                writer.read().write_queue_operation()?;
            }
        }

        // Record the turn to state directory
        let response_text = response.text().to_string();

        if tool_calls.is_empty() {
            // Simple turn without tool calls
            if let Some(ref writer) = self.state {
                writer.write().record_turn(prompt, &response_text)?;
            }
        } else {
            // Turn with tool calls - use granular recording
            self.execute_tool_calls(prompt, &response_text, &tool_calls, &mut writer)
                .await?;
        }

        stdout.flush()?;
        Ok(())
    }

    /// Execute tool calls and record results.
    async fn execute_tool_calls<W: Write>(
        &mut self,
        prompt: &str,
        response_text: &str,
        tool_calls: &[crate::config::ToolCallSpec],
        writer: &mut OutputWriter<W>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Record user message (if persistence enabled)
        let user_uuid = if let Some(ref state_writer) = self.state {
            Some(state_writer.write().record_user_message(prompt)?)
        } else {
            None
        };

        // 2. Record initial assistant text (if any and persistence enabled)
        if !response_text.is_empty() {
            if let (Some(ref state_writer), Some(ref uuid)) = (&self.state, &user_uuid) {
                state_writer
                    .write()
                    .record_assistant_response(uuid, response_text)?;
            }
        }

        // Get execution mode from scenario (defaults to Live)
        let _execution_mode = self
            .scenario
            .as_ref()
            .and_then(|s| s.config().tool_execution.as_ref())
            .map(|te| te.mode.clone())
            .unwrap_or_default();

        // Create execution context
        {
            let mut ctx = ExecutionContext::default();
            if let Some(ref cwd) = self.cli.cwd {
                ctx = ctx.with_cwd(cwd);
            }

            // Execute each tool call
            for (i, call) in tool_calls.iter().enumerate() {
                let tool_use_id = format!("toolu_{:08x}", i);

                // 3. Record assistant message with tool_use block
                let assistant_uuid =
                    if let (Some(ref state_writer), Some(ref uuid)) = (&self.state, &user_uuid) {
                        let tool_use_block = ContentBlock::ToolUse {
                            id: tool_use_id.clone(),
                            name: call.tool.clone(),
                            input: call.input.clone(),
                        };
                        Some(
                            state_writer
                                .write()
                                .record_assistant_tool_use(uuid, vec![tool_use_block])?,
                        )
                    } else {
                        None
                    };

                // 4. Execute tool
                let result = self.executor.execute(call, &tool_use_id, &ctx);
                writer.write_tool_result(&result)?;

                // 5. Record tool result to JSONL
                if let (Some(ref state_writer), Some(ref asst_uuid)) =
                    (&self.state, &assistant_uuid)
                {
                    let result_content = result.text().unwrap_or("");
                    let tool_use_result = result.tool_use_result().unwrap_or(serde_json::json!({}));
                    state_writer.write().record_tool_result(
                        &tool_use_id,
                        result_content,
                        asst_uuid,
                        tool_use_result,
                    )?;
                }
            }

            // 6. Record final assistant response after tool execution
            if let (Some(ref state_writer), Some(ref uuid)) = (&self.state, &user_uuid) {
                let final_response =
                    "Done! The requested operation has been completed successfully.";
                state_writer
                    .write()
                    .record_assistant_response(uuid, final_response)?;
            }
        } // Close the block

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
