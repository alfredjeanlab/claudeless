// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Prompt and shell command execution.
//!
//! Contains:
//! - `execute_shell_command` - Shell command execution via Bash tool
//! - `process_prompt` - Prompt processing and response generation
//! - Test permission triggers for fixture tests

use crate::runtime::TurnResult;
use crate::tui::spinner;
use crate::tui::streaming::{StreamingConfig, StreamingResponse};
use crate::tui::widgets::permission::{DiffKind, DiffLine};

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

            // Get scenario response for the command from runtime
            let response_text = {
                let text = inner
                    .runtime
                    .as_mut()
                    .map(|r| r.response_text_or_default(&command))
                    .unwrap_or_default();
                if text.is_empty() {
                    format!("$ {}", command)
                } else {
                    text
                }
            };

            // Display the response
            setup_response_display(&mut inner, response_text);
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
        // Check for test permission triggers first (before acquiring inner lock)
        if handle_test_permission_triggers(self, &prompt) {
            return;
        }

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
        execute_with_runtime(&mut inner, prompt);
    }
}

/// Execute a prompt using the shared Runtime.
///
/// This uses Runtime::execute() which handles:
/// - Scenario matching
/// - Tool execution
/// - Hook firing
/// - State recording (JSONL)
fn execute_with_runtime(inner: &mut TuiAppStateInner, prompt: String) {
    // Take the runtime temporarily to call execute()
    // Runtime is required in TUI mode
    let Some(mut runtime) = inner.runtime.take() else {
        setup_response_display(inner, "Error: Runtime not available".to_string());
        restore_input_state(inner);
        return;
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
        Err(failure_spec) => handle_failure(inner, &failure_spec),
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
fn handle_turn_result(inner: &mut TuiAppStateInner, result: TurnResult) {
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
        return;
    }

    // Normal completion - restore input state
    restore_input_state(inner);
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
// Test Helpers
// ============================================================================

/// Handle test permission triggers for TUI fixture tests
/// Returns true if a permission dialog was triggered, false otherwise
fn handle_test_permission_triggers(state: &TuiAppState, prompt: &str) -> bool {
    // Test trigger: "test bash permission"
    if prompt.contains("test bash permission") {
        state.show_bash_permission(
            "cat /etc/passwd | head -5".to_string(),
            Some("Display first 5 lines of /etc/passwd".to_string()),
        );
        return true;
    }

    // Test trigger: "test edit permission"
    if prompt.contains("test edit permission") {
        let diff_lines = vec![
            DiffLine {
                line_num: Some(1),
                kind: DiffKind::Removed,
                content: "Hello World".to_string(),
            },
            DiffLine {
                line_num: Some(1),
                kind: DiffKind::NoNewline,
                content: " No newline at end of file".to_string(),
            },
            DiffLine {
                line_num: Some(2),
                kind: DiffKind::Added,
                content: "Hello Universe".to_string(),
            },
            DiffLine {
                line_num: Some(3),
                kind: DiffKind::NoNewline,
                content: " No newline at end of file".to_string(),
            },
        ];
        state.show_edit_permission("hello.txt".to_string(), diff_lines);
        return true;
    }

    // Test trigger: "test write permission"
    if prompt.contains("test write permission") {
        state.show_write_permission("hello.txt".to_string(), vec!["Hello World".to_string()]);
        return true;
    }

    false
}
