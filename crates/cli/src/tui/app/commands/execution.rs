// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Prompt and shell command execution.
//!
//! Contains:
//! - `execute_shell_command` - Shell command execution via Bash tool
//! - `process_prompt` - Prompt processing and response generation
//! - `start_streaming_inner` - Streaming response handling
//! - Test permission triggers for fixture tests

use crate::hooks::{HookEvent, HookMessage, StopHookResponse};
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

        // Add the shell command to conversation display with ! prefix
        if !inner.display.conversation_display.is_empty() {
            inner.display.conversation_display.push_str("\n\n");
        }
        inner
            .display
            .conversation_display
            .push_str(&format!("❯ ! {}", command));

        // Check if bypass mode - execute directly without permission dialog
        if inner.permission_mode.allows_all() {
            // Show bash output directly
            inner
                .display
                .conversation_display
                .push_str(&format!("\n\n⏺ Bash({})", command));

            // Get scenario response for the command
            let response_text = {
                let text = inner.scenario.response_text_or_default(&command);
                if text.is_empty() {
                    format!("$ {}", command)
                } else {
                    text
                }
            };

            // Start streaming the response
            inner.display.response_content.clear();
            inner.display.is_command_output = false;
            start_streaming_inner(&mut inner, response_text);
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

        // Add the new user prompt to conversation display
        if !inner.display.conversation_display.is_empty() {
            inner.display.conversation_display.push_str("\n\n");
        }
        inner
            .display
            .conversation_display
            .push_str(&format!("❯ {}", prompt));

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

        // Check if Runtime is available for shared execution
        if inner.runtime.is_some() {
            // Use Runtime::execute() for shared agent loop
            execute_with_runtime(&mut inner, prompt);
        } else {
            // Fallback: Record user message to JSONL and store UUID for linking
            inner.display.pending_user_uuid = if let Some(ref writer) = inner.state_writer {
                writer.write().record_user_message(&prompt).ok()
            } else {
                None
            };

            // Match scenario
            let response_text = {
                let text = inner.scenario.response_text_or_default(&prompt);
                if text.is_empty() {
                    "I'm not sure how to help with that.".to_string()
                } else {
                    text
                }
            };

            // Start streaming (legacy path)
            start_streaming_inner(&mut inner, response_text);
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
fn execute_with_runtime(inner: &mut TuiAppStateInner, prompt: String) {
    // Take the runtime temporarily to call execute()
    let mut runtime = match inner.runtime.take() {
        Some(r) => r,
        None => {
            // Fallback if runtime was somehow taken
            let response_text = inner.scenario.response_text_or_default(&prompt);
            start_streaming_inner(inner, response_text);
            return;
        }
    };

    // Execute using runtime (via block_in_place + block_on since we're in sync context)
    let result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // Use block_in_place to allow blocking from within an async context
        tokio::task::block_in_place(|| handle.block_on(async { runtime.execute(&prompt).await }))
    } else {
        // No tokio runtime available, fall back to legacy
        inner.runtime = Some(runtime);
        let response_text = inner.scenario.response_text_or_default(&prompt);
        start_streaming_inner(inner, response_text);
        return;
    };

    // Put runtime back
    inner.runtime = Some(runtime);

    // Handle the turn result
    handle_turn_result(inner, result);
}

/// Handle the result of a Runtime::execute() call.
fn handle_turn_result(inner: &mut TuiAppStateInner, result: TurnResult) {
    // Update display with response
    inner.mode = AppMode::Responding;
    inner.display.is_streaming = true;
    inner.display.spinner_frame = 0;
    inner.display.spinner_verb = spinner::random_verb().to_string();

    // Set response content
    let response_text = if result.response_text.is_empty() {
        "I'm not sure how to help with that.".to_string()
    } else {
        result.response_text.clone()
    };

    // Use StreamingResponse for token counting
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

    // Handle hook continuation if present
    if let Some(continuation) = result.hook_continuation {
        inner.pending_hook_message = Some(continuation);
        inner.stop_hook_active = true;
        // Stay in responding mode - the render loop will detect pending_hook_message
        return;
    }

    // Reset stop_hook_active on normal completion
    inner.stop_hook_active = false;
    inner.mode = AppMode::Input;

    // Auto-restore stashed text after response completes
    if let Some(stashed) = inner.input.stash.take() {
        inner.input.buffer = stashed;
        inner.input.cursor_pos = inner.input.buffer.len();
        inner.input.show_stash_indicator = false;
    }
}

/// Start streaming a response
pub(super) fn start_streaming_inner(inner: &mut TuiAppStateInner, text: String) {
    inner.mode = AppMode::Responding;
    inner.display.is_streaming = true;
    // Reset spinner for responding animation
    inner.display.spinner_frame = 0;
    inner.display.spinner_verb = spinner::random_verb().to_string();

    let config = StreamingConfig;
    let clock = inner.clock.clone();
    let response = StreamingResponse::new(text, config, clock);

    // For synchronous operation, just set the full text
    // In async mode, this would use the TokenStream
    inner.display.response_content = response.full_text().to_string();
    inner.display.is_streaming = false;

    // Update token counts
    inner.status.output_tokens += response.tokens_streamed();
    inner.status.input_tokens += (inner.input.buffer.len() / 4).max(1) as u32;

    // Update session with response
    if let Some(turn) = inner.sessions.current_session().turns.last_mut() {
        turn.response = inner.display.response_content.clone();
    }

    // Record assistant response to JSONL (errors are ignored to not disrupt TUI)
    if let (Some(ref writer), Some(ref user_uuid)) =
        (&inner.state_writer, &inner.display.pending_user_uuid)
    {
        let _ = writer
            .write()
            .record_assistant_response(user_uuid, &inner.display.response_content);
    }
    inner.display.pending_user_uuid = None;

    // Fire Stop hook if configured (synchronously using block_on for TUI)
    // Note: This is a simplified implementation for the simulator
    if let Some(ref executor) = inner.config.hook_executor {
        if executor.has_hooks(&HookEvent::Stop) {
            // Generate a session ID for the hook message
            let session_id = inner
                .status
                .session_id
                .clone()
                .unwrap_or_else(|| "tui-session".to_string());
            let stop_msg = HookMessage::stop(&session_id, inner.stop_hook_active);

            // Execute hook synchronously using tokio block_on
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let responses = handle.block_on(async { executor.execute(&stop_msg).await });
                if let Ok(responses) = responses {
                    for resp in responses {
                        if let Some(data) = resp.data {
                            if let Ok(stop_resp) = serde_json::from_value::<StopHookResponse>(data)
                            {
                                if stop_resp.is_blocked() {
                                    // Queue the reason as next user message
                                    inner.pending_hook_message = Some(
                                        stop_resp.reason.unwrap_or_else(|| "continue".to_string()),
                                    );
                                    inner.stop_hook_active = true;
                                    // Stay in responding mode briefly to trigger re-process
                                    // The input handler will detect pending_hook_message
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Reset stop_hook_active on normal completion
    inner.stop_hook_active = false;
    inner.mode = AppMode::Input;

    // Auto-restore stashed text after response completes
    if let Some(stashed) = inner.input.stash.take() {
        inner.input.buffer = stashed;
        inner.input.cursor_pos = inner.input.buffer.len();
        inner.input.show_stash_indicator = false;
    }
}

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
