// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JSONL format types for session logging (matching Claude CLI v2.1.12).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// Common envelope fields shared by all message line types.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEnvelope {
    #[serde(rename = "type")]
    pub line_type: String,
    pub uuid: String,
    pub timestamp: String,
    pub session_id: String,
    pub cwd: String,
    pub version: String,
    pub git_branch: String,
    pub parent_uuid: Option<String>,
    pub is_sidechain: bool,
    pub user_type: String,
}

/// User message content for JSONL.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserMessage {
    pub role: &'static str,
    pub content: String,
}

/// User message line in JSONL format.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMessageLine {
    #[serde(flatten)]
    pub envelope: MessageEnvelope,
    pub message: UserMessage,
}

/// Tool result content block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResultContent {
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub content_type: &'static str,
    pub content: String,
}

/// User message with tool result content.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResultUserMessage {
    pub role: &'static str,
    pub content: Vec<ToolResultContent>,
}

/// Tool result message line in JSONL format.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultMessageLine {
    #[serde(flatten)]
    pub envelope: MessageEnvelope,
    pub message: ToolResultUserMessage,
    pub tool_use_result: serde_json::Value,
    #[serde(rename = "sourceToolAssistantUUID")]
    pub source_tool_assistant_uuid: String,
}

/// Content block for assistant messages.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// Cache creation breakdown for usage statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CacheCreation {
    #[serde(default)]
    pub ephemeral_5m_input_tokens: u32,
    #[serde(default)]
    pub ephemeral_1h_input_tokens: u32,
}

/// Usage statistics for API response.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    #[serde(default)]
    pub cache_read_input_tokens: u32,
    #[serde(default)]
    pub cache_creation: CacheCreation,
    pub output_tokens: u32,
    #[serde(default)]
    pub service_tier: String,
}

impl Usage {
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_creation: CacheCreation::default(),
            output_tokens,
            service_tier: "standard".to_string(),
        }
    }
}

/// Assistant message content for JSONL (includes API envelope fields).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub model: String,
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: &'static str,
    pub role: &'static str,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

/// Assistant message line in JSONL format.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessageLine {
    #[serde(flatten)]
    pub envelope: MessageEnvelope,
    pub message: AssistantMessage,
    pub request_id: String,
}

/// Queue operation for -p mode (first line in session).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueOperationLine {
    #[serde(rename = "type")]
    pub line_type: &'static str,
    pub operation: String,
    pub timestamp: String,
    pub session_id: String,
}

/// Tool result record for log extraction.
///
/// This is a separate record type (in addition to user messages with tool_result content)
/// that enables otters log extraction to find tool results easily.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultLine {
    #[serde(rename = "type")]
    pub line_type: String,
    pub tool_use_id: String,
    pub content: String,
    pub timestamp: String,
}

/// Helper to open a file for appending.
fn open_append(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
}

/// Helper to write a single JSONL line to a file.
fn write_jsonl_line<T: Serialize>(file: &mut std::fs::File, value: &T) -> std::io::Result<()> {
    let json = serde_json::to_string(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    writeln!(file, "{}", json)
}

/// Write queue-operation line for -p mode.
pub fn write_queue_operation(
    path: &Path,
    session_id: &str,
    operation: &str,
    timestamp: DateTime<Utc>,
) -> std::io::Result<()> {
    let mut file = open_append(path)?;
    let line = QueueOperationLine {
        line_type: "queue-operation",
        operation: operation.to_string(),
        timestamp: timestamp.to_rfc3339(),
        session_id: session_id.to_string(),
    };
    write_jsonl_line(&mut file, &line)
}

/// Append a result record to a JSONL file.
///
/// This writes a `type: "result"` record that enables log extraction tools
/// to easily find tool results and parse exit codes.
pub fn append_result_jsonl(
    path: &Path,
    tool_use_id: &str,
    content: &str,
    timestamp: DateTime<Utc>,
) -> std::io::Result<()> {
    let mut file = open_append(path)?;
    let line = ResultLine {
        line_type: "result".to_string(),
        tool_use_id: tool_use_id.to_string(),
        content: content.to_string(),
        timestamp: timestamp.to_rfc3339(),
    };
    write_jsonl_line(&mut file, &line)
}

/// Parameters for writing JSONL turns.
#[derive(Clone, Debug)]
pub struct TurnParams<'a> {
    pub session_id: &'a str,
    pub user_uuid: &'a str,
    pub assistant_uuid: &'a str,
    pub request_id: &'a str,
    pub prompt: &'a str,
    pub response: &'a str,
    pub model: &'a str,
    pub cwd: &'a str,
    pub version: &'a str,
    pub git_branch: &'a str,
    pub message_id: &'a str,
    pub timestamp: DateTime<Utc>,
}

/// Append a conversation turn to a JSONL file.
pub fn append_turn_jsonl(path: &Path, params: &TurnParams) -> std::io::Result<()> {
    let mut file = open_append(path)?;
    let timestamp_str = params.timestamp.to_rfc3339();

    let envelope_base = |line_type: &str, uuid: &str, parent_uuid| MessageEnvelope {
        line_type: line_type.to_string(),
        uuid: uuid.to_string(),
        timestamp: timestamp_str.clone(),
        session_id: params.session_id.to_string(),
        cwd: params.cwd.to_string(),
        version: params.version.to_string(),
        git_branch: params.git_branch.to_string(),
        parent_uuid,
        is_sidechain: false,
        user_type: "external".to_string(),
    };

    let user_line = UserMessageLine {
        envelope: envelope_base("user", params.user_uuid, None),
        message: UserMessage {
            role: "user",
            content: params.prompt.to_string(),
        },
    };
    write_jsonl_line(&mut file, &user_line)?;

    let assistant_line = AssistantMessageLine {
        envelope: envelope_base(
            "assistant",
            params.assistant_uuid,
            Some(params.user_uuid.to_string()),
        ),
        message: AssistantMessage {
            model: params.model.to_string(),
            id: params.message_id.to_string(),
            message_type: "message",
            role: "assistant",
            content: vec![ContentBlock::Text {
                text: params.response.to_string(),
            }],
            stop_reason: None,
            stop_sequence: None,
            usage: Usage::new(2, 1),
        },
        request_id: params.request_id.to_string(),
    };
    write_jsonl_line(&mut file, &assistant_line)
}

/// Parameters for writing a user message JSONL line.
#[derive(Clone, Debug)]
pub struct UserMessageParams<'a> {
    pub session_id: &'a str,
    pub user_uuid: &'a str,
    pub parent_uuid: Option<&'a str>,
    pub content: UserMessageContent<'a>,
    pub cwd: &'a str,
    pub version: &'a str,
    pub git_branch: &'a str,
    pub timestamp: DateTime<Utc>,
}

/// User message content variants.
#[derive(Clone, Debug)]
pub enum UserMessageContent<'a> {
    Text(&'a str),
    ToolResult {
        tool_use_id: &'a str,
        content: &'a str,
        tool_use_result: serde_json::Value,
        source_tool_assistant_uuid: &'a str,
    },
}

/// Append a user message to a JSONL file.
pub fn append_user_message_jsonl(path: &Path, params: &UserMessageParams) -> std::io::Result<()> {
    let mut file = open_append(path)?;
    let timestamp_str = params.timestamp.to_rfc3339();

    match &params.content {
        UserMessageContent::Text(text) => {
            let user_line = UserMessageLine {
                envelope: MessageEnvelope {
                    line_type: "user".to_string(),
                    uuid: params.user_uuid.to_string(),
                    timestamp: timestamp_str,
                    session_id: params.session_id.to_string(),
                    cwd: params.cwd.to_string(),
                    version: params.version.to_string(),
                    git_branch: params.git_branch.to_string(),
                    parent_uuid: params.parent_uuid.map(String::from),
                    is_sidechain: false,
                    user_type: "external".to_string(),
                },
                message: UserMessage {
                    role: "user",
                    content: (*text).to_string(),
                },
            };
            write_jsonl_line(&mut file, &user_line)?;
        }
        UserMessageContent::ToolResult {
            tool_use_id,
            content,
            tool_use_result,
            source_tool_assistant_uuid,
        } => {
            let parent = params
                .parent_uuid
                .map(String::from)
                .unwrap_or_else(|| source_tool_assistant_uuid.to_string());
            let tool_result_line = ToolResultMessageLine {
                envelope: MessageEnvelope {
                    line_type: "user".to_string(),
                    uuid: params.user_uuid.to_string(),
                    timestamp: timestamp_str,
                    session_id: params.session_id.to_string(),
                    cwd: params.cwd.to_string(),
                    version: params.version.to_string(),
                    git_branch: params.git_branch.to_string(),
                    parent_uuid: Some(parent),
                    is_sidechain: false,
                    user_type: "external".to_string(),
                },
                message: ToolResultUserMessage {
                    role: "user",
                    content: vec![ToolResultContent {
                        tool_use_id: (*tool_use_id).to_string(),
                        content_type: "tool_result",
                        content: (*content).to_string(),
                    }],
                },
                tool_use_result: tool_use_result.clone(),
                source_tool_assistant_uuid: (*source_tool_assistant_uuid).to_string(),
            };
            write_jsonl_line(&mut file, &tool_result_line)?;
        }
    }

    Ok(())
}

/// Parameters for writing an assistant message JSONL line.
#[derive(Clone, Debug)]
pub struct AssistantMessageParams<'a> {
    pub session_id: &'a str,
    pub assistant_uuid: &'a str,
    pub parent_uuid: &'a str,
    pub request_id: &'a str,
    pub message_id: &'a str,
    pub content: Vec<ContentBlock>,
    pub model: &'a str,
    pub stop_reason: Option<&'a str>,
    pub cwd: &'a str,
    pub version: &'a str,
    pub git_branch: &'a str,
    pub timestamp: DateTime<Utc>,
}

/// Error entry in JSONL format for failure events.
///
/// Uses the result wrapper format with `subtype: "error"` to match
/// the format expected by watchers that parse JSONL for error fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorLine {
    #[serde(rename = "type")]
    pub line_type: &'static str,
    pub subtype: String,
    pub is_error: bool,
    pub session_id: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
    pub duration_ms: u64,
    pub timestamp: String,
}

/// Append an assistant message to a JSONL file.
pub fn append_assistant_message_jsonl(
    path: &Path,
    params: &AssistantMessageParams,
) -> std::io::Result<()> {
    let mut file = open_append(path)?;
    let timestamp_str = params.timestamp.to_rfc3339();

    let assistant_line = AssistantMessageLine {
        envelope: MessageEnvelope {
            line_type: "assistant".to_string(),
            uuid: params.assistant_uuid.to_string(),
            timestamp: timestamp_str,
            session_id: params.session_id.to_string(),
            cwd: params.cwd.to_string(),
            version: params.version.to_string(),
            git_branch: params.git_branch.to_string(),
            parent_uuid: Some(params.parent_uuid.to_string()),
            is_sidechain: false,
            user_type: "external".to_string(),
        },
        message: AssistantMessage {
            model: params.model.to_string(),
            id: params.message_id.to_string(),
            message_type: "message",
            role: "assistant",
            content: params.content.clone(),
            stop_reason: params.stop_reason.map(String::from),
            stop_sequence: None,
            usage: Usage::new(2, 1),
        },
        request_id: params.request_id.to_string(),
    };
    write_jsonl_line(&mut file, &assistant_line)
}

/// Append an error entry to a JSONL file.
///
/// Writes an error line in the result wrapper format with `subtype: "error"`.
pub fn append_error_jsonl(
    path: &Path,
    session_id: &str,
    error: &str,
    error_type: Option<&str>,
    retry_after: Option<u64>,
    duration_ms: u64,
    timestamp: DateTime<Utc>,
) -> std::io::Result<()> {
    let mut file = open_append(path)?;

    let error_line = ErrorLine {
        line_type: "result",
        subtype: "error".to_string(),
        is_error: true,
        session_id: session_id.to_string(),
        error: error.to_string(),
        error_type: error_type.map(String::from),
        retry_after,
        duration_ms,
        timestamp: timestamp.to_rfc3339(),
    };

    write_jsonl_line(&mut file, &error_line)
}

#[cfg(test)]
#[path = "jsonl_tests.rs"]
mod tests;
