// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Core Runtime struct for orchestrating prompt execution.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;

use crate::capture::CaptureLog;
use crate::cli::Cli;
use crate::config::{FailureSpec, ResolvedTimeouts, ResponseSpec, ToolCallSpec};
use crate::failure::FailureExecutor;
use crate::hooks::{HookEvent, HookExecutor, HookMessage, StopHookResponse};
use crate::mcp::McpManager;
use crate::scenario::Scenario;
use crate::state::{ContentBlock, StateWriter};
use crate::tools::{ExecutionContext, ToolExecutionResult, ToolExecutor};

use super::RuntimeContext;

/// Result of a single agent turn.
///
/// This is the unified result type used by both print mode and TUI mode.
/// It contains everything needed to:
/// - Display the response
/// - Show tool results
/// - Handle hook continuations
#[derive(Debug)]
pub struct TurnResult {
    /// The full response from the assistant (includes usage stats for JSON output).
    pub response: ResponseSpec,
    /// Results from tool execution (if any tools were called).
    pub tool_results: Vec<ToolExecutionResult>,
    /// If a Stop hook blocked, this contains the continuation prompt.
    /// The caller should re-invoke execute() with this prompt.
    pub hook_continuation: Option<String>,
    /// Whether this turn was a hook continuation (not the initial prompt).
    pub is_hook_continuation: bool,
}

impl TurnResult {
    /// Get the response text.
    pub fn response_text(&self) -> &str {
        self.response.text()
    }
}

/// Core runtime for executing prompts.
///
/// Owns the composed subsystems: context, scenario, executor, state, and capture.
/// Provides a unified `execute` method for processing prompts.
pub struct Runtime {
    /// Merged runtime context from scenario + CLI.
    pub context: RuntimeContext,
    /// Loaded scenario (optional).
    pub scenario: Option<Scenario>,
    /// Tool executor (disabled, mock, or live with MCP).
    pub(super) executor: Box<dyn ToolExecutor>,
    /// State writer for JSONL persistence (optional).
    pub(super) state: Option<Arc<RwLock<StateWriter>>>,
    /// Capture log for test assertions (optional).
    pub(super) capture: Option<CaptureLog>,
    /// Hook executor for Stop hooks (optional).
    pub(super) hook_executor: Option<HookExecutor>,
    /// MCP manager for server lifecycle (optional).
    pub(super) mcp_manager: Option<Arc<RwLock<McpManager>>>,
    /// CLI configuration (needed for output format, cwd, etc.).
    pub(super) cli: Cli,
    /// Resolved timeouts from scenario.
    pub(super) timeouts: ResolvedTimeouts,
    /// Whether currently in a stop hook continuation.
    pub(super) stop_hook_active: bool,
}

impl Runtime {
    /// Create a new Runtime with all dependencies.
    // TODO(refactor): Group related parameters into RuntimeComponents struct
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        context: RuntimeContext,
        scenario: Option<Scenario>,
        executor: Box<dyn ToolExecutor>,
        state: Option<Arc<RwLock<StateWriter>>>,
        capture: Option<CaptureLog>,
        hook_executor: Option<HookExecutor>,
        mcp_manager: Option<Arc<RwLock<McpManager>>>,
        cli: Cli,
        timeouts: ResolvedTimeouts,
    ) -> Self {
        Self {
            context,
            scenario,
            executor,
            state,
            capture,
            hook_executor,
            mcp_manager,
            cli,
            timeouts,
            stop_hook_active: false,
        }
    }

    /// Get the session ID.
    pub fn session_id(&self) -> String {
        self.context.session_id.to_string()
    }

    /// Check if this runtime should use TUI mode.
    pub fn should_use_tui(&self) -> bool {
        self.cli.should_use_tui()
    }

    /// Get the MCP manager (for TUI mode handoff).
    pub fn mcp_manager(&self) -> Option<Arc<RwLock<McpManager>>> {
        self.mcp_manager.as_ref().map(Arc::clone)
    }

    /// Get the state writer (for TUI mode handoff).
    pub fn state_writer(&self) -> Option<Arc<RwLock<StateWriter>>> {
        self.state.as_ref().map(Arc::clone)
    }

    /// Get the hook executor (for TUI mode handoff).
    pub fn hook_executor(&self) -> Option<&HookExecutor> {
        self.hook_executor.as_ref()
    }

    /// Get the CLI reference.
    pub fn cli(&self) -> &Cli {
        &self.cli
    }

    /// Get the resolved timeouts.
    pub fn timeouts(&self) -> &ResolvedTimeouts {
        &self.timeouts
    }

    /// Get the scenario config (for TUI mode config extraction).
    pub fn scenario_config(&self) -> &crate::config::ScenarioConfig {
        self.scenario
            .as_ref()
            .map(|s| s.config())
            .unwrap_or_else(|| {
                // Use a static default config if no scenario
                static DEFAULT_CONFIG: std::sync::OnceLock<crate::config::ScenarioConfig> =
                    std::sync::OnceLock::new();
                DEFAULT_CONFIG.get_or_init(crate::config::ScenarioConfig::default)
            })
    }

    /// Take ownership of the scenario (for TUI mode handoff).
    ///
    /// This removes the scenario from Runtime, allowing TUI to own it.
    pub fn take_scenario(&mut self) -> Option<Scenario> {
        self.scenario.take()
    }

    /// Execute a single agent turn.
    ///
    /// This is the core shared method for prompt execution. Both print mode
    /// and TUI mode should use this for agent turns.
    ///
    /// The method:
    /// 1. Matches the prompt against the scenario
    /// 2. Executes any tool calls in the response
    /// 3. Fires Stop hooks
    /// 4. Records state to JSONL
    ///
    /// Returns `Ok(TurnResult)` on success, or `Err(FailureSpec)` if the scenario
    /// specifies a failure. On failure, error is recorded to JSONL before returning.
    pub async fn execute(&mut self, prompt: &str) -> Result<TurnResult, FailureSpec> {
        // Apply response delay if configured (only on initial prompts)
        if !self.stop_hook_active && self.timeouts.response_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.timeouts.response_delay_ms)).await;
        }

        // Match prompt to get response (or failure)
        let response_spec = match self.match_prompt_for_turn(prompt) {
            Ok(spec) => spec,
            Err(failure_spec) => {
                // Record error to JSONL before returning
                self.record_failure_to_jsonl(&failure_spec);
                return Err(failure_spec);
            }
        };

        // Get response delay from spec if detailed
        let response_delay = response_spec.as_ref().and_then(|r| r.delay_ms());
        if let Some(delay) = response_delay {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        // Get response and tool calls
        let response = response_spec.unwrap_or(ResponseSpec::Simple(String::new()));
        let response_text = response.text().to_string();
        let tool_calls = response.tool_calls().to_vec();

        // Execute tools and collect results
        let tool_results = self.execute_tools_for_turn(prompt, &response_text, &tool_calls);

        // Record the turn to state if no tool calls (tool calls record their own state)
        if tool_calls.is_empty() {
            if let Some(ref writer) = self.state {
                let _ = writer.write().record_turn(prompt, &response_text);
            }
        }

        // Fire Stop hook
        let hook_continuation = self.fire_stop_hook().await;

        // Capture whether THIS turn was a hook continuation (before updating for next turn)
        let is_hook_continuation = self.stop_hook_active;

        // Update stop_hook_active for the NEXT turn
        self.stop_hook_active = hook_continuation.is_some();

        Ok(TurnResult {
            response,
            tool_results,
            hook_continuation,
            is_hook_continuation,
        })
    }

    /// Match prompt against scenario (for execute()).
    fn match_prompt_for_turn(
        &mut self,
        prompt: &str,
    ) -> Result<Option<ResponseSpec>, FailureSpec> {
        if let Some(ref mut scenario) = self.scenario {
            if let Some(result) = scenario.match_prompt(prompt) {
                // Check for failure in rule
                if let Some(failure_spec) = scenario.get_failure(&result) {
                    return Err(failure_spec.clone());
                }

                Ok(scenario.get_response(&result).cloned())
            } else if let Some(default) = scenario.default_response() {
                Ok(Some(default.clone()))
            } else {
                Ok(None)
            }
        } else {
            // No scenario - use a default response
            Ok(Some(ResponseSpec::Simple(
                "Hello! I'm Claudeless!".to_string(),
            )))
        }
    }

    /// Record failure to JSONL (shared behavior for both print mode and TUI).
    fn record_failure_to_jsonl(&self, failure_spec: &FailureSpec) {
        if let Some(ref writer) = self.state {
            let _ = FailureExecutor::record_to_jsonl(failure_spec, writer);
        }
    }

    /// Execute tool calls and return results (for execute()).
    fn execute_tools_for_turn(
        &mut self,
        prompt: &str,
        response_text: &str,
        tool_calls: &[ToolCallSpec],
    ) -> Vec<ToolExecutionResult> {
        if tool_calls.is_empty() {
            return vec![];
        }

        // Record user message
        let user_uuid = if let Some(ref state_writer) = self.state {
            state_writer.write().record_user_message(prompt).ok()
        } else {
            None
        };

        // Record initial assistant text (if any)
        if !response_text.is_empty() {
            if let (Some(ref state_writer), Some(ref uuid)) = (&self.state, &user_uuid) {
                let _ = state_writer
                    .write()
                    .record_assistant_response(uuid, response_text);
            }
        }

        // Create execution context
        let mut ctx = ExecutionContext::default();
        if let Some(ref cwd) = self.cli.cwd {
            ctx = ctx.with_cwd(cwd);
        }

        let mut results = Vec::with_capacity(tool_calls.len());

        for (i, call) in tool_calls.iter().enumerate() {
            let tool_use_id = format!("toolu_{:08x}", i);

            // Record assistant message with tool_use block
            let assistant_uuid =
                if let (Some(ref state_writer), Some(ref uuid)) = (&self.state, &user_uuid) {
                    let tool_use_block = ContentBlock::ToolUse {
                        id: tool_use_id.clone(),
                        name: call.tool.clone(),
                        input: call.input.clone(),
                    };
                    state_writer
                        .write()
                        .record_assistant_tool_use(uuid, vec![tool_use_block])
                        .ok()
                } else {
                    None
                };

            // Execute tool
            let result = self.executor.execute(call, &tool_use_id, &ctx);

            // Record tool result to JSONL
            if let (Some(ref state_writer), Some(ref asst_uuid)) = (&self.state, &assistant_uuid) {
                let result_content = result.text().unwrap_or("");
                let tool_use_result = result.tool_use_result().unwrap_or(serde_json::json!({}));
                let _ = state_writer.write().record_tool_result(
                    &tool_use_id,
                    result_content,
                    asst_uuid,
                    tool_use_result,
                );
            }

            results.push(result);
        }

        // Record final assistant response after tool execution
        if let (Some(ref state_writer), Some(ref uuid)) = (&self.state, &user_uuid) {
            let final_response = "Done! The requested operation has been completed successfully.";
            let _ = state_writer
                .write()
                .record_assistant_response(uuid, final_response);
        }

        results
    }

    /// Fire Stop hook and return continuation prompt if blocked.
    async fn fire_stop_hook(&self) -> Option<String> {
        if let Some(ref executor) = self.hook_executor {
            if executor.has_hooks(&HookEvent::Stop) {
                let stop_msg =
                    HookMessage::stop(self.context.session_id.to_string(), self.stop_hook_active);
                if let Ok(responses) = executor.execute(&stop_msg).await {
                    for resp in responses {
                        if let Some(data) = resp.data {
                            if let Ok(stop_resp) = serde_json::from_value::<StopHookResponse>(data)
                            {
                                if stop_resp.is_blocked() {
                                    return Some(
                                        stop_resp.reason.unwrap_or_else(|| "continue".to_string()),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Shutdown MCP manager gracefully.
    #[allow(clippy::await_holding_lock)]
    pub async fn shutdown_mcp(&self) {
        if let Some(ref mgr) = self.mcp_manager {
            // SAFETY(await_holding_lock): Holding write lock across await is acceptable here:
            // - Runs once at process exit, no concurrent lock acquisition
            // - parking_lot::RwLock guards are Send
            mgr.write().shutdown().await;
        }
    }
}

#[cfg(test)]
#[path = "core_tests.rs"]
mod tests;
