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
            drop(inner);
            let permission = self.execute_with_runtime(command);
            if let Some(perm) = permission {
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
        let permission = self.execute_with_runtime(prompt);
        if let Some(perm) = permission {
            self.show_permission_request(perm);
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
    /// The lock is dropped during execution so the render thread can observe
    /// intermediate mode changes (e.g., Thinking mode).
    ///
    /// Returns `Some(PermissionType)` if a tool needs an interactive permission
    /// prompt. The caller should call `show_permission_request()` with the
    /// returned type.
    fn execute_with_runtime(&self, prompt: String) -> Option<PermissionType> {
        // Take the runtime out while holding the lock briefly
        let mut runtime = {
            let mut inner = self.inner.lock();
            let Some(runtime) = inner.runtime.take() else {
                setup_response_display(&mut inner, "Error: Runtime not available".to_string());
                restore_input_state(&mut inner);
                return None;
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
        match outcome {
            Ok(result) => handle_turn_result(&mut inner, result),
            Err(failure_spec) => {
                handle_failure(&mut inner, &failure_spec);
                None
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
/// Returns `Some(PermissionType)` if a tool needs a permission prompt.
fn handle_turn_result(inner: &mut TuiAppStateInner, result: TurnResult) -> Option<PermissionType> {
    // Build display parts from completed tool calls
    let tool_calls = result.response.tool_calls().to_vec();
    let completed_count = result.tool_results.len();

    // Check for pending permission before displaying response
    if let Some(ref pending) = result.pending_permission {
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

            return Some(perm_type);
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
// Tool Call Display Formatting
// ============================================================================

/// Word-wrap a text paragraph for display after a `⏺ ` prefix.
///
/// Real Claude Code wraps response text at the terminal width with a 2-space
/// continuation indent. The first line has a 2-char prefix (`⏺ `), and
/// continuation lines use `  ` (2 spaces) indent.
fn wrap_response_paragraph(text: &str, terminal_width: usize) -> String {
    // Account for "⏺ " prefix on first line (2 visual columns)
    let first_line_width = terminal_width.saturating_sub(2);
    // Continuation indent is "  " (2 spaces)
    let continuation_width = terminal_width.saturating_sub(2);

    if first_line_width == 0 || text.chars().count() <= first_line_width {
        return text.to_string();
    }

    let mut result = String::new();
    let mut current_line_len = 0;
    let mut is_first_line = true;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        let max_width = if is_first_line {
            first_line_width
        } else {
            continuation_width
        };

        if current_line_len == 0 {
            result.push_str(word);
            current_line_len = word_len;
        } else if current_line_len + 1 + word_len <= max_width {
            result.push(' ');
            result.push_str(word);
            current_line_len += 1 + word_len;
        } else {
            result.push_str("\n  ");
            result.push_str(word);
            current_line_len = word_len;
            is_first_line = false;
        }
    }

    result
}

/// Join display parts where the first part is unprefixed (gets ⏺ from display layer)
/// and subsequent parts get their own ⏺ prefix.
fn join_display_parts(parts: &[String]) -> String {
    let mut result = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            result.push_str(part);
        } else {
            result.push_str("\n\n⏺ ");
            result.push_str(part);
        }
    }
    result
}

/// Format a completed tool call with its result for display.
fn format_completed_tool_display(call: &ToolCallSpec, result_text: Option<&str>) -> String {
    match call.tool.as_str() {
        "Write" => {
            let file_path = call
                .input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let mut display = format!("Write({})", file_path);
            if let Some(result) = result_text {
                display.push_str(&format!("\n  \u{23bf} \u{a0}{}", result));
                // Show content lines indented under the result
                if let Some(content) = call.input.get("content").and_then(|v| v.as_str()) {
                    for (i, line) in content.split('\n').enumerate() {
                        display.push_str(&format!("\n      {} {}", i + 1, line));
                    }
                }
            }
            display
        }
        "Read" => {
            if let Some(result) = result_text {
                // Results ending with "…" indicate a streaming/in-progress read
                if result.ends_with('\u{2026}') {
                    format!("Reading {} (ctrl+o to expand)", result)
                } else {
                    format!("Read {} (ctrl+o to expand)", result)
                }
            } else {
                "Read (ctrl+o to expand)".to_string()
            }
        }
        "Bash" => {
            let command = call
                .input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let mut display = format!("Bash({})", command);
            if let Some(result) = result_text {
                display.push_str(&format!("\n  \u{23bf} \u{a0}{}", result));
            }
            display
        }
        _ => {
            if let Some(result) = result_text {
                result.to_string()
            } else {
                call.tool.clone()
            }
        }
    }
}

/// Format a tool call for display above the permission dialog.
fn format_tool_call_display(call: &ToolCallSpec) -> String {
    match call.tool.as_str() {
        "Bash" => {
            let command = call
                .input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Bash({})\n  \u{23bf} \u{a0}Running\u{2026}", command)
        }
        "Edit" => {
            let file_path = call
                .input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Update({})", file_path)
        }
        "Write" => {
            let file_path = call
                .input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Write({})", file_path)
        }
        _ => call.tool.clone(),
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
            let mut line_num: u32 = 1;

            // Removed lines
            for line in old_string.lines() {
                diff_lines.push(DiffLine {
                    line_num: Some(line_num),
                    kind: DiffKind::Removed,
                    content: line.to_string(),
                });
                line_num += 1;
            }
            // NoNewline marker after removed lines if old_string doesn't end with newline
            if !old_string.is_empty() && !old_string.ends_with('\n') {
                diff_lines.push(DiffLine {
                    line_num: Some(line_num - 1),
                    kind: DiffKind::NoNewline,
                    content: "No newline at end of file".to_string(),
                });
            }

            // Added lines (line numbering continues from removed)
            let added_start = line_num;
            for (i, line) in new_string.lines().enumerate() {
                diff_lines.push(DiffLine {
                    line_num: Some(added_start + i as u32),
                    kind: DiffKind::Added,
                    content: line.to_string(),
                });
            }
            // NoNewline marker after added lines
            let added_count = new_string.lines().count();
            if !new_string.is_empty() && !new_string.ends_with('\n') {
                diff_lines.push(DiffLine {
                    line_num: Some(added_start + added_count as u32),
                    kind: DiffKind::NoNewline,
                    content: "No newline at end of file".to_string(),
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
