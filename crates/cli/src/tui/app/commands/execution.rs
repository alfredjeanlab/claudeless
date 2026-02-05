// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Prompt and shell command execution.
//!
//! Contains:
//! - `execute_shell_command` - Shell command execution via Bash tool
//! - `process_prompt` - Prompt processing and response generation

use crate::hooks::{NOTIFICATION_ELICITATION_DIALOG, NOTIFICATION_IDLE_PROMPT};
use crate::runtime::TurnResult;
use crate::tui::spinner;
use crate::tui::streaming::{StreamingConfig, StreamingResponse};
use crate::tui::widgets::elicitation::{ElicitationResult, ElicitationState};
use crate::tui::widgets::permission::PermissionType;
use crate::tui::widgets::plan_approval::{PlanApprovalResult, PlanApprovalState};

use super::display::{
    format_completed_tool_display, format_tool_call_display, join_display_parts,
    tool_call_to_permission_type, wrap_response_paragraph,
};

use super::super::state::TuiAppState;
use super::super::state::TuiAppStateInner;
use super::super::types::AppMode;

/// Result of handling a turn, indicating what side-effect to fire.
enum TurnAction {
    /// Normal completion — agent is idle, fire idle_prompt notification.
    Done,
    /// A tool needs an interactive permission prompt.
    Permission(PermissionType),
    /// Fire a notification hook with the given notification_type.
    Notify(&'static str),
    /// Stop hook continuation — stay in responding mode.
    HookContinuation,
}

impl TuiAppState {
    /// Execute a shell command via Bash tool
    pub(in crate::tui::app) fn execute_shell_command(&self, command: String) {
        let mut inner = self.inner.lock();

        // Add previous response to conversation display if any
        append_previous_response(&mut inner);

        // Add the shell command to conversation display with ! prefix
        append_to_conversation(&mut inner, "❯ !", &command);

        // Check if bypass mode - execute directly without permission dialog
        if inner.permission_mode.allows_all() {
            // Show bash output directly
            inner
                .display
                .conversation_display
                .push_str(&format!("\n\n⏺ Bash({})", command));

            // Use shared execute path for consistency (hooks, JSONL recording, etc.)
            drop(inner);
            self.execute_with_runtime(command);
            return;
        }

        // Show bash permission dialog
        inner.mode = AppMode::Thinking;
        inner.display.response_content.clear();
        inner.display.is_command_output = false;
        // Reset spinner for thinking animation
        inner.display.spinner_frame = 0;
        inner.display.spinner_verb = spinner::random_verb().to_string();

        drop(inner);

        // Use existing bash permission flow
        self.show_bash_permission(command.clone(), Some(format!("Execute: {}", command)));
    }

    /// Process a prompt and generate response
    pub(in crate::tui::app) fn process_prompt(&self, prompt: String) {
        {
            let mut inner = self.inner.lock();

            // If there's previous response content, add it to conversation history first
            append_previous_response(&mut inner);

            // Add the new user prompt to conversation display
            append_to_conversation(&mut inner, "❯", &prompt);

            inner.mode = AppMode::Thinking;
            inner.is_compacting = false;
            inner.display.response_content.clear();
            inner.display.is_command_output = false;
            // Reset spinner for thinking animation
            inner.display.spinner_frame = 0;
            inner.display.spinner_verb = spinner::random_verb().to_string();

            // Record the turn
            inner
                .sessions
                .current_session()
                .add_turn(prompt.clone(), String::new());

            // Drop lock before execution so the render thread can see Thinking mode
        }

        // Use Runtime::execute() for shared agent loop
        self.execute_with_runtime(prompt);
    }

    /// Confirm the elicitation dialog and re-execute with answers.
    pub(in crate::tui::app) fn confirm_elicitation(&self) {
        let elicitation = {
            let mut inner = self.inner.lock();
            if let super::super::state::DialogState::Elicitation(state) =
                std::mem::take(&mut inner.dialog)
            {
                inner.mode = super::super::types::AppMode::Thinking;
                Some(state)
            } else {
                None
            }
        };

        if let Some(state) = elicitation {
            let answers = match state.collect_answers() {
                ElicitationResult::Answered(answers) => answers,
                ElicitationResult::Cancelled => {
                    let mut inner = self.inner.lock();
                    inner.display.response_content =
                        "User declined to answer questions".to_string();
                    restore_input_state(&mut inner);
                    return;
                }
                ElicitationResult::ChatAboutThis => {
                    // Build clarification rejection message matching real Claude Code
                    let questions_summary: Vec<String> = state
                        .questions
                        .iter()
                        .map(|q| format!("- \"{}\"\n  (No answer provided)", q.question))
                        .collect();
                    let mut inner = self.inner.lock();
                    inner.display.response_content = format!(
                        "The user wants to clarify these questions.\n    \
                         This means they may have additional information, context or questions for you.\n    \
                         Take their response into account and then reformulate the questions if appropriate.\n    \
                         Start by asking them what they would like to clarify.\n\n    \
                         Questions asked:\n{}",
                        questions_summary.join("\n")
                    );
                    restore_input_state(&mut inner);
                    return;
                }
            };

            // Build a modified tool call with answers injected
            let mut input = state.tool_input.clone();
            input["answers"] = serde_json::json!(answers);

            // Re-execute via runtime with the answers
            let prompt = serde_json::to_string(&serde_json::json!({
                "questions": input.get("questions"),
                "answers": answers,
            }))
            .unwrap_or_default();
            self.execute_with_runtime(prompt);
        }
    }

    /// Cancel the elicitation dialog.
    pub(in crate::tui::app) fn cancel_elicitation(&self) {
        let mut inner = self.inner.lock();
        inner.dialog.dismiss();
        inner.display.response_content = "User declined to answer questions".to_string();
        restore_input_state(&mut inner);
    }

    /// Confirm the plan approval dialog and re-execute with approval.
    pub(in crate::tui::app) fn confirm_plan_approval(&self) {
        let plan_approval = {
            let mut inner = self.inner.lock();
            if let super::super::state::DialogState::PlanApproval(state) =
                std::mem::take(&mut inner.dialog)
            {
                inner.mode = super::super::types::AppMode::Thinking;
                Some(state)
            } else {
                None
            }
        };

        if let Some(state) = plan_approval {
            match state.collect_result() {
                PlanApprovalResult::Approved(mode) => {
                    // Build approval message for the tool result
                    let mode_str = match mode {
                        crate::tui::widgets::plan_approval::ApprovalMode::ClearContext => {
                            "clear_context_auto_accept"
                        }
                        crate::tui::widgets::plan_approval::ApprovalMode::AutoAccept => {
                            "auto_accept"
                        }
                        crate::tui::widgets::plan_approval::ApprovalMode::ManualApprove => {
                            "manual_approve"
                        }
                    };

                    // Re-execute with approval injected
                    let mut input = state.tool_input.clone();
                    input["approval"] = serde_json::json!(mode_str);

                    let prompt = serde_json::to_string(&serde_json::json!({
                        "plan_approved": true,
                        "approval_mode": mode_str,
                    }))
                    .unwrap_or_default();
                    self.execute_with_runtime(prompt);
                }
                PlanApprovalResult::Revised(feedback) => {
                    // Re-execute with feedback
                    let prompt = serde_json::to_string(&serde_json::json!({
                        "plan_feedback": feedback,
                    }))
                    .unwrap_or_default();
                    self.execute_with_runtime(prompt);
                }
                PlanApprovalResult::Cancelled => {
                    let mut inner = self.inner.lock();
                    inner.display.response_content = "User rejected tool use".to_string();
                    restore_input_state(&mut inner);
                }
            }
        }
    }

    /// Cancel the plan approval dialog.
    pub(in crate::tui::app) fn cancel_plan_approval(&self) {
        let mut inner = self.inner.lock();
        inner.dialog.dismiss();
        inner.display.response_content = "User rejected tool use".to_string();
        restore_input_state(&mut inner);
    }

    /// Execute a prompt using the shared Runtime.
    ///
    /// This uses Runtime::execute() which handles:
    /// - Scenario matching
    /// - Tool execution
    /// - Hook firing
    /// - State recording (JSONL)
    ///
    /// The lock is dropped during execution so the render thread can observe
    /// intermediate mode changes (e.g., Thinking mode).
    ///
    /// Handles all post-turn actions internally: permission prompts,
    /// notification hooks, and stop hook continuations.
    fn execute_with_runtime(&self, prompt: String) {
        // Take the runtime out while holding the lock briefly
        let mut runtime = {
            let mut inner = self.inner.lock();
            let Some(runtime) = inner.runtime.take() else {
                setup_response_display(&mut inner, "Error: Runtime not available".to_string());
                restore_input_state(&mut inner);
                return;
            };
            runtime
            // Lock is dropped here - render thread can now see Thinking mode
        };

        // Execute using runtime (via block_in_place + block_on since we're in sync context)
        // Lock is NOT held during this blocking call
        let handle = tokio::runtime::Handle::current();
        let outcome = tokio::task::block_in_place(|| {
            handle.block_on(async { runtime.execute(&prompt).await })
        });

        // Re-acquire lock to put runtime back and handle the result
        let mut inner = self.inner.lock();
        inner.runtime = Some(runtime);

        // Handle the outcome (success or failure)
        let action = match outcome {
            Ok(result) => handle_turn_result(&mut inner, result),
            Err(failure_spec) => {
                handle_failure(&mut inner, &failure_spec);
                TurnAction::Done
            }
        };
        drop(inner);

        // Fire side-effects outside the lock
        match action {
            TurnAction::Permission(perm) => {
                self.show_permission_request(perm);
            }
            TurnAction::Done => {
                self.fire_notification(
                    NOTIFICATION_IDLE_PROMPT,
                    "Agent Idle",
                    "Claude is waiting for input",
                );
            }
            TurnAction::Notify(notification_type) => {
                self.fire_notification(notification_type, notification_type, "");
            }
            TurnAction::HookContinuation => {
                // No notification needed — stop hook continuation is in progress
            }
        }
    }
}

/// Handle a failure from Runtime::execute().
///
/// In TUI mode, we display the error message and return to input mode.
/// The JSONL recording was already done by execute().
fn handle_failure(inner: &mut TuiAppStateInner, failure_spec: &crate::config::FailureSpec) {
    use crate::config::FailureSpec;

    // Convert failure to user-friendly error message
    let error_message = match failure_spec {
        FailureSpec::NetworkUnreachable => "Error: Network is unreachable".to_string(),
        FailureSpec::ConnectionTimeout { after_ms } => {
            format!("Error: Connection timed out after {}ms", after_ms)
        }
        FailureSpec::AuthError { message } => format!("Error: {}", message),
        FailureSpec::RateLimit { retry_after } => {
            format!("Error: Rate limited. Retry after {} seconds.", retry_after)
        }
        FailureSpec::OutOfCredits => "Error: No credits remaining".to_string(),
        FailureSpec::PartialResponse { partial_text } => {
            format!("Partial response: {}", partial_text)
        }
        FailureSpec::MalformedJson { raw } => format!("Malformed response: {}", raw),
    };

    // Display error as response and return to input
    setup_response_display(inner, error_message);
    restore_input_state(inner);
}

/// Handle the result of a Runtime::execute() call.
///
/// Returns a `TurnAction` indicating what side-effect to fire.
fn handle_turn_result(inner: &mut TuiAppStateInner, result: TurnResult) -> TurnAction {
    // Build display parts from completed tool calls
    let tool_calls = result.response.tool_calls().to_vec();
    let completed_count = result.tool_results.len();

    // Check for pending permission before displaying response
    if let Some(ref pending) = result.pending_permission {
        // ExitPlanMode uses plan approval dialog
        if pending.tool_call.tool == "ExitPlanMode" {
            // Extract plan file path from the tool call's pre-configured result
            // or generate a placeholder path
            let plan_file_path = pending
                .tool_call
                .result
                .as_deref()
                .and_then(|r| {
                    // Try to parse as "Plan saved as X.md" to extract path
                    r.strip_prefix("Plan saved as ")
                        .map(|name| format!("~/.claude/plans/{}", name))
                })
                .unwrap_or_else(|| "~/.claude/plans/plan.md".to_string());

            let state = PlanApprovalState::from_tool_input(
                &pending.tool_call.input,
                pending.tool_use_id.clone(),
                plan_file_path,
            );
            inner.dialog = super::super::state::DialogState::PlanApproval(state);
            inner.mode = super::super::types::AppMode::PlanApproval;

            // Build display for completed tool calls before the plan approval
            let mut parts = Vec::new();
            for (i, call) in tool_calls.iter().take(completed_count).enumerate() {
                let result_text = result.tool_results.get(i).and_then(|r| r.text());
                parts.push(format_completed_tool_display(call, result_text));
            }
            let response_text = result.response_text();
            if !response_text.is_empty() {
                parts.push(wrap_response_paragraph(
                    response_text,
                    inner.display.terminal_width as usize,
                ));
            }
            if !parts.is_empty() {
                setup_response_display(inner, join_display_parts(&parts));
            }
            return TurnAction::Notify(NOTIFICATION_ELICITATION_DIALOG);
        }

        // AskUserQuestion uses elicitation dialog instead of permission dialog
        if pending.tool_call.tool == "AskUserQuestion" {
            let state = ElicitationState::from_tool_input(
                &pending.tool_call.input,
                pending.tool_use_id.clone(),
            );
            inner.dialog = super::super::state::DialogState::Elicitation(state);
            inner.mode = super::super::types::AppMode::Elicitation;

            // Build display for completed tool calls before the elicitation
            let mut parts = Vec::new();
            for (i, call) in tool_calls.iter().take(completed_count).enumerate() {
                let result_text = result.tool_results.get(i).and_then(|r| r.text());
                parts.push(format_completed_tool_display(call, result_text));
            }
            let response_text = result.response_text();
            if !response_text.is_empty() {
                parts.push(wrap_response_paragraph(
                    response_text,
                    inner.display.terminal_width as usize,
                ));
            }
            if !parts.is_empty() {
                setup_response_display(inner, join_display_parts(&parts));
            }
            return TurnAction::Notify(NOTIFICATION_ELICITATION_DIALOG);
        }

        if let Some(perm_type) = tool_call_to_permission_type(&pending.tool_call) {
            // Build display: completed tool calls + pending tool call context
            let mut parts = Vec::new();

            // Format completed tool calls with their results
            for (i, call) in tool_calls.iter().take(completed_count).enumerate() {
                let result_text = result.tool_results.get(i).and_then(|r| r.text());
                parts.push(format_completed_tool_display(call, result_text));
            }

            // Add response text (if any), word-wrapped at terminal width
            let response_text = result.response_text();
            if !response_text.is_empty() {
                parts.push(wrap_response_paragraph(
                    response_text,
                    inner.display.terminal_width as usize,
                ));
            }

            // Add pending tool call context display.
            // For Bash, only show "Running…" display when it's the sole tool;
            // for Edit/Write, always show the pending tool name.
            if completed_count == 0 || pending.tool_call.tool != "Bash" {
                parts.push(format_tool_call_display(&pending.tool_call));
            }

            // Join with ⏺ prefix on subsequent parts (first gets prefix from display layer)
            let display = join_display_parts(&parts);
            if !display.is_empty() {
                setup_response_display(inner, display);
            }

            // Build post-grant display: what to show after permission is granted.
            // This replaces the pending tool display with its completed form.
            let mut post_parts = Vec::new();
            for (i, call) in tool_calls.iter().take(completed_count).enumerate() {
                let result_text = result.tool_results.get(i).and_then(|r| r.text());
                post_parts.push(format_completed_tool_display(call, result_text));
            }
            // Format the pending tool as completed (using its mock result if available)
            let pending_result = pending.tool_call.result.as_deref();
            post_parts.push(format_completed_tool_display(
                &pending.tool_call,
                pending_result,
            ));
            if !response_text.is_empty() {
                post_parts.push(wrap_response_paragraph(
                    response_text,
                    inner.display.terminal_width as usize,
                ));
            }
            inner.display.pending_post_grant_display = Some(join_display_parts(&post_parts));

            return TurnAction::Permission(perm_type);
        }
    }

    // Build display: completed tool calls + response text
    let mut parts = Vec::new();

    // Format completed tool calls with their results
    for (i, call) in tool_calls.iter().take(completed_count).enumerate() {
        let result_text = result.tool_results.get(i).and_then(|r| r.text());
        parts.push(format_completed_tool_display(call, result_text));
    }

    // Add response text, word-wrapped at terminal width
    let response_text = if result.response_text().is_empty() && parts.is_empty() {
        "I'm not sure how to help with that.".to_string()
    } else {
        result.response_text().to_string()
    };
    if !response_text.is_empty() {
        parts.push(wrap_response_paragraph(
            &response_text,
            inner.display.terminal_width as usize,
        ));
    }

    // Build display
    let display = if parts.is_empty() {
        "I'm not sure how to help with that.".to_string()
    } else {
        join_display_parts(&parts)
    };

    setup_response_display(inner, display);

    // Handle hook continuation if present
    if let Some(continuation) = result.hook_continuation {
        inner.pending_hook_message = Some(continuation);
        inner.stop_hook_active = true;
        // Stay in responding mode - the render loop will detect pending_hook_message
        return TurnAction::HookContinuation;
    }

    // Normal completion - restore input state
    restore_input_state(inner);
    TurnAction::Done
}

// ============================================================================
// Helper Functions - Extracted to reduce duplication
// ============================================================================

/// Append previous response content to conversation display (if any).
fn append_previous_response(inner: &mut TuiAppStateInner) {
    if !inner.display.response_content.is_empty() && !inner.display.is_command_output {
        let response = inner.display.response_content.clone();
        if !inner.display.conversation_display.is_empty() {
            inner.display.conversation_display.push_str("\n\n");
        }
        inner
            .display
            .conversation_display
            .push_str(&format!("⏺ {}", response));
    }
}

/// Append a message to the conversation display with the given prefix.
fn append_to_conversation(inner: &mut TuiAppStateInner, prefix: &str, content: &str) {
    if !inner.display.conversation_display.is_empty() {
        inner.display.conversation_display.push_str("\n\n");
    }
    inner
        .display
        .conversation_display
        .push_str(&format!("{} {}", prefix, content));
}

/// Set up response display with streaming simulation.
fn setup_response_display(inner: &mut TuiAppStateInner, response_text: String) {
    inner.mode = AppMode::Responding;
    inner.display.is_streaming = true;
    inner.display.spinner_frame = 0;
    inner.display.spinner_verb = spinner::random_verb().to_string();

    let config = StreamingConfig;
    let clock = inner.clock.clone();
    let response = StreamingResponse::new(response_text, config, clock);

    inner.display.response_content = response.full_text().to_string();
    inner.display.is_streaming = false;

    // Update token counts
    inner.status.output_tokens += response.tokens_streamed();
    inner.status.input_tokens += (inner.input.buffer.len() / 4).max(1) as u32;

    // Update session with response
    if let Some(turn) = inner.sessions.current_session().turns.last_mut() {
        turn.response = inner.display.response_content.clone();
    }
}

/// Restore input state after response completes.
fn restore_input_state(inner: &mut TuiAppStateInner) {
    inner.stop_hook_active = false;
    inner.mode = AppMode::Input;

    // Auto-restore stashed text after response completes
    if let Some(stashed) = inner.input.stash.take() {
        inner.input.buffer = stashed;
        inner.input.cursor_pos = inner.input.buffer.len();
        inner.input.show_stash_indicator = false;
    }
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
