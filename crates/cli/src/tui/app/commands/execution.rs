// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Prompt and shell command execution.
//!
//! Contains:
//! - `execute_shell_command` - Shell command execution via Bash tool
//! - `process_prompt` - Prompt processing and response generation
//! - `tool_call_to_permission_type` - Converts tool calls to permission dialogs

use crate::config::ToolCallSpec;
use crate::runtime::TurnResult;
use crate::tui::spinner;
use crate::tui::streaming::{StreamingConfig, StreamingResponse};
use crate::tui::widgets::permission::{DiffKind, DiffLine, PermissionType};

use super::super::state::TuiAppState;
use super::super::state::TuiAppStateInner;
use super::super::types::AppMode;

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
            let permission = execute_with_runtime(&mut inner, command);
            if let Some(perm) = permission {
                drop(inner);
                self.show_permission_request(perm);
            }
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
        let mut inner = self.inner.lock();

        // If there's previous response content, add it to conversation history first
        append_previous_response(&mut inner);

        // Add the new user prompt to conversation display
        append_to_conversation(&mut inner, "❯", &prompt);

        inner.mode = AppMode::Thinking;
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

        // Use Runtime::execute() for shared agent loop
        let permission = execute_with_runtime(&mut inner, prompt);
        if let Some(perm) = permission {
            drop(inner);
            self.show_permission_request(perm);
        }
    }
}

/// Execute a prompt using the shared Runtime.
///
/// This uses Runtime::execute() which handles:
/// - Scenario matching
/// - Tool execution
/// - Hook firing
/// - State recording (JSONL)
///
/// Returns `Some(PermissionType)` if a tool needs an interactive permission
/// prompt. The caller should drop the inner lock and call
/// `show_permission_request()` with the returned type.
fn execute_with_runtime(inner: &mut TuiAppStateInner, prompt: String) -> Option<PermissionType> {
    // Take the runtime temporarily to call execute()
    // Runtime is required in TUI mode
    let Some(mut runtime) = inner.runtime.take() else {
        setup_response_display(inner, "Error: Runtime not available".to_string());
        restore_input_state(inner);
        return None;
    };

    // Execute using runtime (via block_in_place + block_on since we're in sync context)
    let handle = tokio::runtime::Handle::current();
    let outcome =
        tokio::task::block_in_place(|| handle.block_on(async { runtime.execute(&prompt).await }));

    // Put runtime back
    inner.runtime = Some(runtime);

    // Handle the outcome (success or failure)
    match outcome {
        Ok(result) => handle_turn_result(inner, result),
        Err(failure_spec) => {
            handle_failure(inner, &failure_spec);
            None
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
/// Returns `Some(PermissionType)` if a tool needs a permission prompt.
fn handle_turn_result(inner: &mut TuiAppStateInner, result: TurnResult) -> Option<PermissionType> {
    // Check for pending permission before displaying response
    if let Some(ref pending) = result.pending_permission {
        if let Some(perm_type) = tool_call_to_permission_type(&pending.tool_call) {
            // Display the response text so far (before the tool call)
            let response_text = if result.response_text().is_empty() {
                String::new()
            } else {
                result.response_text().to_string()
            };
            if !response_text.is_empty() {
                setup_response_display(inner, response_text);
            }
            return Some(perm_type);
        }
    }

    // Set response content
    let response_text = if result.response_text().is_empty() {
        "I'm not sure how to help with that.".to_string()
    } else {
        result.response_text().to_string()
    };

    // Display the response
    setup_response_display(inner, response_text);

    // Handle hook continuation if present
    if let Some(continuation) = result.hook_continuation {
        inner.pending_hook_message = Some(continuation);
        inner.stop_hook_active = true;
        // Stay in responding mode - the render loop will detect pending_hook_message
        return None;
    }

    // Normal completion - restore input state
    restore_input_state(inner);
    None
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

// ============================================================================
// Tool Call → Permission Type Conversion
// ============================================================================

/// Convert a `ToolCallSpec` into a `PermissionType` for the TUI permission dialog.
///
/// Returns `None` for unknown tool names (e.g., MCP tools that don't have
/// a corresponding permission dialog).
pub(crate) fn tool_call_to_permission_type(call: &ToolCallSpec) -> Option<PermissionType> {
    match call.tool.as_str() {
        "Bash" => {
            let command = call.input.get("command")?.as_str()?.to_string();
            let description = call
                .input
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Some(PermissionType::Bash {
                command,
                description,
            })
        }
        "Write" => {
            let file_path = call.input.get("file_path")?.as_str()?.to_string();
            let content = call
                .input
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let content_lines = content.split('\n').map(|s| s.to_string()).collect();
            Some(PermissionType::Write {
                file_path,
                content_lines,
            })
        }
        "Edit" => {
            let file_path = call.input.get("file_path")?.as_str()?.to_string();
            let old_string = call
                .input
                .get("old_string")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let new_string = call
                .input
                .get("new_string")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let mut diff_lines = Vec::new();
            for (i, line) in old_string.lines().enumerate() {
                diff_lines.push(DiffLine {
                    line_num: Some((i + 1) as u32),
                    kind: DiffKind::Removed,
                    content: line.to_string(),
                });
            }
            for (i, line) in new_string.lines().enumerate() {
                diff_lines.push(DiffLine {
                    line_num: Some((i + 1) as u32),
                    kind: DiffKind::Added,
                    content: line.to_string(),
                });
            }

            Some(PermissionType::Edit {
                file_path,
                diff_lines,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
