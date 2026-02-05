// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Hook message protocol types.

use serde::{Deserialize, Serialize};

/// Hook event types
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    /// Before tool execution
    PreToolExecution,
    /// After tool execution
    PostToolExecution,
    /// Notification to user
    Notification,
    /// Permission request
    PermissionRequest,
    /// Session start
    SessionStart,
    /// Session end
    SessionEnd,
    /// Prompt submitted (before processing)
    PromptSubmit,
    /// Before context compaction
    PreCompact,
    /// Claude finishes responding
    Stop,
}

/// Hook message sent to hook script
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HookMessage {
    /// Event type
    pub event: HookEvent,

    /// Session ID
    pub session_id: String,

    /// Event-specific payload
    pub payload: HookPayload,
}

impl HookMessage {
    /// Create a tool execution message
    pub fn tool_execution(
        session_id: impl Into<String>,
        event: HookEvent,
        tool_name: impl Into<String>,
        tool_input: serde_json::Value,
        tool_output: Option<String>,
    ) -> Self {
        Self {
            event,
            session_id: session_id.into(),
            payload: HookPayload::ToolExecution {
                tool_name: tool_name.into(),
                tool_input,
                tool_output,
            },
        }
    }

    /// Create a notification message
    pub fn notification(
        session_id: impl Into<String>,
        notification_type: impl Into<String>,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            event: HookEvent::Notification,
            session_id: session_id.into(),
            payload: HookPayload::Notification {
                notification_type: notification_type.into(),
                title: title.into(),
                message: message.into(),
            },
        }
    }

    /// Create a permission request message
    pub fn permission(
        session_id: impl Into<String>,
        tool_name: impl Into<String>,
        action: impl Into<String>,
        context: serde_json::Value,
    ) -> Self {
        Self {
            event: HookEvent::PermissionRequest,
            session_id: session_id.into(),
            payload: HookPayload::Permission {
                tool_name: tool_name.into(),
                action: action.into(),
                context,
            },
        }
    }

    /// Create a session lifecycle message
    pub fn session(
        session_id: impl Into<String>,
        event: HookEvent,
        project_path: Option<String>,
    ) -> Self {
        Self {
            event,
            session_id: session_id.into(),
            payload: HookPayload::Session { project_path },
        }
    }

    /// Create a prompt submit message
    pub fn prompt_submit(session_id: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            event: HookEvent::PromptSubmit,
            session_id: session_id.into(),
            payload: HookPayload::Prompt {
                prompt: prompt.into(),
            },
        }
    }

    /// Create a pre-compaction message
    pub fn compaction(
        session_id: impl Into<String>,
        trigger: CompactionTrigger,
        custom_instructions: Option<String>,
    ) -> Self {
        Self {
            event: HookEvent::PreCompact,
            session_id: session_id.into(),
            payload: HookPayload::Compaction {
                trigger,
                custom_instructions,
            },
        }
    }

    /// Create a stop message
    pub fn stop(session_id: impl Into<String>, stop_hook_active: bool) -> Self {
        Self {
            event: HookEvent::Stop,
            session_id: session_id.into(),
            payload: HookPayload::Stop { stop_hook_active },
        }
    }
}

/// Hook payload variants
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookPayload {
    /// Tool execution context
    ToolExecution {
        tool_name: String,
        tool_input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_output: Option<String>,
    },

    /// Notification content
    Notification {
        notification_type: String,
        title: String,
        message: String,
    },

    /// Permission request
    Permission {
        tool_name: String,
        action: String,
        context: serde_json::Value,
    },

    /// Session lifecycle
    Session {
        #[serde(skip_serializing_if = "Option::is_none")]
        project_path: Option<String>,
    },

    /// Prompt submission
    Prompt { prompt: String },

    /// Context compaction
    Compaction {
        trigger: CompactionTrigger,
        #[serde(skip_serializing_if = "Option::is_none")]
        custom_instructions: Option<String>,
    },

    /// Stop event (Claude finishes responding)
    Stop {
        /// True when Claude Code is already continuing as a result of a stop hook.
        /// Check this value or process the transcript to prevent infinite loops.
        stop_hook_active: bool,
    },
}

/// Compaction trigger type
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionTrigger {
    /// Manual compaction via /compact command
    Manual,
    /// Automatic compaction when context is full
    Auto,
}

/// Notification type: permission prompt is waiting for user input.
pub const NOTIFICATION_PERMISSION_PROMPT: &str = "permission_prompt";
/// Notification type: agent is idle, waiting for the next user prompt.
pub const NOTIFICATION_IDLE_PROMPT: &str = "idle_prompt";
/// Notification type: an elicitation dialog (AskUserQuestion) is shown.
pub const NOTIFICATION_ELICITATION_DIALOG: &str = "elicitation_dialog";
/// Notification type: authentication / trust was successfully granted.
pub const NOTIFICATION_AUTH_SUCCESS: &str = "auth_success";

/// Hook response from hook script
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HookResponse {
    /// Whether to proceed (for pre-hooks)
    #[serde(default = "default_proceed")]
    pub proceed: bool,

    /// Modified payload (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_payload: Option<serde_json::Value>,

    /// Error message if hook failed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Additional data returned by hook
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

fn default_proceed() -> bool {
    true
}

impl HookResponse {
    /// Create a successful proceed response
    pub fn proceed() -> Self {
        Self {
            proceed: true,
            modified_payload: None,
            error: None,
            data: None,
        }
    }

    /// Create a blocking response
    pub fn block(reason: impl Into<String>) -> Self {
        Self {
            proceed: false,
            modified_payload: None,
            error: Some(reason.into()),
            data: None,
        }
    }

    /// Create a response with modified payload
    pub fn with_modified(mut self, payload: serde_json::Value) -> Self {
        self.modified_payload = Some(payload);
        self
    }

    /// Create a response with additional data
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

impl Default for HookResponse {
    fn default() -> Self {
        Self::proceed()
    }
}

/// Stop hook response with decision field.
///
/// Stop hooks can return this format to continue the conversation:
/// ```json
/// {"decision": "block", "reason": "Please verify the changes"}
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StopHookResponse {
    /// Decision: "block" to continue conversation, "allow" to proceed with exit
    #[serde(default = "default_decision")]
    pub decision: String,

    /// Reason for blocking (used as next user message when blocked)
    #[serde(default)]
    pub reason: Option<String>,
}

fn default_decision() -> String {
    "allow".to_string()
}

impl StopHookResponse {
    /// Check if the hook response blocks (continues conversation)
    pub fn is_blocked(&self) -> bool {
        self.decision == "block"
    }

    /// Create an allow response
    pub fn allow() -> Self {
        Self {
            decision: "allow".to_string(),
            reason: None,
        }
    }

    /// Create a block response with reason
    pub fn block(reason: impl Into<String>) -> Self {
        Self {
            decision: "block".to_string(),
            reason: Some(reason.into()),
        }
    }
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
