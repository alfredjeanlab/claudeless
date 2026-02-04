// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JSONL persistence for session state.
//!
//! This module provides types and functions for reading and writing session data
//! in JSONL format, matching Claude CLI v2.1.12.

use crate::event_types::{line_type, message_type, role, user_type};
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
        line_type: line_type::QUEUE_OPERATION,
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
        line_type: line_type::RESULT.to_string(),
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

    let envelope_base = |lt: &str, uuid: &str, parent_uuid| MessageEnvelope {
        line_type: lt.to_string(),
        uuid: uuid.to_string(),
        timestamp: timestamp_str.clone(),
        session_id: params.session_id.to_string(),
        cwd: params.cwd.to_string(),
        version: params.version.to_string(),
        git_branch: params.git_branch.to_string(),
        parent_uuid,
        is_sidechain: false,
        user_type: user_type::EXTERNAL.to_string(),
    };

    let user_line = UserMessageLine {
        envelope: envelope_base(line_type::USER, params.user_uuid, None),
        message: UserMessage {
            role: role::USER,
            content: params.prompt.to_string(),
        },
    };
    write_jsonl_line(&mut file, &user_line)?;

    let assistant_line = AssistantMessageLine {
        envelope: envelope_base(
            line_type::ASSISTANT,
            params.assistant_uuid,
            Some(params.user_uuid.to_string()),
        ),
        message: AssistantMessage {
            model: params.model.to_string(),
            id: params.message_id.to_string(),
            message_type: message_type::MESSAGE,
            role: role::ASSISTANT,
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
                    line_type: line_type::USER.to_string(),
                    uuid: params.user_uuid.to_string(),
                    timestamp: timestamp_str,
                    session_id: params.session_id.to_string(),
                    cwd: params.cwd.to_string(),
                    version: params.version.to_string(),
                    git_branch: params.git_branch.to_string(),
                    parent_uuid: params.parent_uuid.map(String::from),
                    is_sidechain: false,
                    user_type: user_type::EXTERNAL.to_string(),
                },
                message: UserMessage {
                    role: role::USER,
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
                    line_type: line_type::USER.to_string(),
                    uuid: params.user_uuid.to_string(),
                    timestamp: timestamp_str,
                    session_id: params.session_id.to_string(),
                    cwd: params.cwd.to_string(),
                    version: params.version.to_string(),
                    git_branch: params.git_branch.to_string(),
                    parent_uuid: Some(parent),
                    is_sidechain: false,
                    user_type: user_type::EXTERNAL.to_string(),
                },
                message: ToolResultUserMessage {
                    role: role::USER,
                    content: vec![ToolResultContent {
                        tool_use_id: (*tool_use_id).to_string(),
                        content_type: message_type::TOOL_RESULT,
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

/// Server tool use counts (zero-valued for synthetic error messages).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ServerToolUse {
    pub web_search_requests: u32,
    pub web_fetch_requests: u32,
}

/// All-zero usage matching real Claude error format.
///
/// Differs from normal `Usage` by including `server_tool_use` and using
/// `Option` for `service_tier` (serializes as null when None).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SyntheticUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
    pub server_tool_use: ServerToolUse,
    pub service_tier: Option<String>,
    pub cache_creation: CacheCreation,
}

/// Synthetic assistant message body for API error lines.
///
/// Uses `model: "<synthetic>"` and `stop_reason: "stop_sequence"` to match
/// the format real Claude Code writes for error conditions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiErrorAssistantMessage {
    pub id: String,
    pub container: Option<String>,
    pub model: String,
    pub role: &'static str,
    pub stop_reason: &'static str,
    pub stop_sequence: String,
    #[serde(rename = "type")]
    pub message_type: &'static str,
    pub usage: SyntheticUsage,
    pub content: Vec<ContentBlock>,
    pub context_management: Option<String>,
}

/// Full JSONL line for API error messages.
///
/// Matches real Claude Code's error format: `type: "assistant"` with
/// `isApiErrorMessage: true` and a synthetic assistant message.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorMessageLine {
    #[serde(flatten)]
    pub envelope: MessageEnvelope,
    pub message: ApiErrorAssistantMessage,
    pub error: String,
    pub is_api_error_message: bool,
}

/// Parameters for writing an API error message JSONL line.
#[derive(Clone, Debug)]
pub struct ErrorMessageParams<'a> {
    pub session_id: &'a str,
    pub uuid: &'a str,
    pub parent_uuid: Option<&'a str>,
    pub message_id: &'a str,
    pub error_text: &'a str,
    pub error_class: &'a str,
    pub cwd: &'a str,
    pub version: &'a str,
    pub git_branch: &'a str,
    pub timestamp: DateTime<Utc>,
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
            line_type: line_type::ASSISTANT.to_string(),
            uuid: params.assistant_uuid.to_string(),
            timestamp: timestamp_str,
            session_id: params.session_id.to_string(),
            cwd: params.cwd.to_string(),
            version: params.version.to_string(),
            git_branch: params.git_branch.to_string(),
            parent_uuid: Some(params.parent_uuid.to_string()),
            is_sidechain: false,
            user_type: user_type::EXTERNAL.to_string(),
        },
        message: AssistantMessage {
            model: params.model.to_string(),
            id: params.message_id.to_string(),
            message_type: message_type::MESSAGE,
            role: role::ASSISTANT,
            content: params.content.clone(),
            stop_reason: params.stop_reason.map(String::from),
            stop_sequence: None,
            usage: Usage::new(2, 1),
        },
        request_id: params.request_id.to_string(),
    };
    write_jsonl_line(&mut file, &assistant_line)
}

/// Append an API error message to a JSONL file.
///
/// Writes an error line in the real Claude Code format: `type: "assistant"` with
/// `isApiErrorMessage: true` and a synthetic assistant message body.
pub fn append_api_error_jsonl(path: &Path, params: &ErrorMessageParams) -> std::io::Result<()> {
    let mut file = open_append(path)?;
    let timestamp_str = params.timestamp.to_rfc3339();

    let line = ApiErrorMessageLine {
        envelope: MessageEnvelope {
            line_type: line_type::ASSISTANT.to_string(),
            uuid: params.uuid.to_string(),
            timestamp: timestamp_str,
            session_id: params.session_id.to_string(),
            cwd: params.cwd.to_string(),
            version: params.version.to_string(),
            git_branch: params.git_branch.to_string(),
            parent_uuid: params.parent_uuid.map(String::from),
            is_sidechain: false,
            user_type: user_type::EXTERNAL.to_string(),
        },
        message: ApiErrorAssistantMessage {
            id: params.message_id.to_string(),
            container: None,
            model: "<synthetic>".to_string(),
            role: role::ASSISTANT,
            stop_reason: "stop_sequence",
            stop_sequence: String::new(),
            message_type: message_type::MESSAGE,
            usage: SyntheticUsage::default(),
            content: vec![ContentBlock::Text {
                text: params.error_text.to_string(),
            }],
            context_management: None,
        },
        error: params.error_class.to_string(),
        is_api_error_message: true,
    };

    write_jsonl_line(&mut file, &line)
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
