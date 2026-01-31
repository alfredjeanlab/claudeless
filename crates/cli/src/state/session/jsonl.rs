// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JSONL format types for session logging (matching Claude CLI v2.1.12).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// User message content for JSONL.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserMessage {
    pub role: &'static str,
    pub content: String,
}

/// User message line in JSONL format.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMessageLine {
    pub parent_uuid: Option<String>,
    pub is_sidechain: bool,
    pub user_type: String,
    pub cwd: String,
    pub session_id: String,
    pub version: String,
    pub git_branch: String,
    #[serde(rename = "type")]
    pub line_type: &'static str,
    pub message: UserMessage,
    pub uuid: String,
    pub timestamp: String,
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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultMessageLine {
    pub parent_uuid: String,
    pub is_sidechain: bool,
    pub user_type: String,
    pub cwd: String,
    pub session_id: String,
    pub version: String,
    pub git_branch: String,
    #[serde(rename = "type")]
    pub line_type: &'static str,
    pub message: ToolResultUserMessage,
    pub uuid: String,
    pub timestamp: String,
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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessageLine {
    pub parent_uuid: String,
    pub is_sidechain: bool,
    pub user_type: String,
    pub cwd: String,
    pub session_id: String,
    pub version: String,
    pub git_branch: String,
    pub message: AssistantMessage,
    pub request_id: String,
    #[serde(rename = "type")]
    pub line_type: &'static str,
    pub uuid: String,
    pub timestamp: String,
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
    writeln!(file, "{}", serde_json::to_string(&line)?)?;
    Ok(())
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
    writeln!(file, "{}", serde_json::to_string(&line)?)?;
    Ok(())
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

    let user_line = UserMessageLine {
        parent_uuid: None,
        is_sidechain: false,
        user_type: "external".to_string(),
        cwd: params.cwd.to_string(),
        session_id: params.session_id.to_string(),
        version: params.version.to_string(),
        git_branch: params.git_branch.to_string(),
        line_type: "user",
        message: UserMessage {
            role: "user",
            content: params.prompt.to_string(),
        },
        uuid: params.user_uuid.to_string(),
        timestamp: timestamp_str.clone(),
    };
    writeln!(file, "{}", serde_json::to_string(&user_line)?)?;

    let assistant_line = AssistantMessageLine {
        parent_uuid: params.user_uuid.to_string(),
        is_sidechain: false,
        user_type: "external".to_string(),
        cwd: params.cwd.to_string(),
        session_id: params.session_id.to_string(),
        version: params.version.to_string(),
        git_branch: params.git_branch.to_string(),
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
        line_type: "assistant",
        uuid: params.assistant_uuid.to_string(),
        timestamp: timestamp_str,
    };
    writeln!(file, "{}", serde_json::to_string(&assistant_line)?)?;

    Ok(())
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
                parent_uuid: params.parent_uuid.map(String::from),
                is_sidechain: false,
                user_type: "external".to_string(),
                cwd: params.cwd.to_string(),
                session_id: params.session_id.to_string(),
                version: params.version.to_string(),
                git_branch: params.git_branch.to_string(),
                line_type: "user",
                message: UserMessage {
                    role: "user",
                    content: (*text).to_string(),
                },
                uuid: params.user_uuid.to_string(),
                timestamp: timestamp_str,
            };
            writeln!(file, "{}", serde_json::to_string(&user_line)?)?;
        }
        UserMessageContent::ToolResult {
            tool_use_id,
            content,
            tool_use_result,
            source_tool_assistant_uuid,
        } => {
            let tool_result_line = ToolResultMessageLine {
                parent_uuid: params
                    .parent_uuid
                    .map(String::from)
                    .unwrap_or_else(|| source_tool_assistant_uuid.to_string()),
                is_sidechain: false,
                user_type: "external".to_string(),
                cwd: params.cwd.to_string(),
                session_id: params.session_id.to_string(),
                version: params.version.to_string(),
                git_branch: params.git_branch.to_string(),
                line_type: "user",
                message: ToolResultUserMessage {
                    role: "user",
                    content: vec![ToolResultContent {
                        tool_use_id: (*tool_use_id).to_string(),
                        content_type: "tool_result",
                        content: (*content).to_string(),
                    }],
                },
                uuid: params.user_uuid.to_string(),
                timestamp: timestamp_str,
                tool_use_result: tool_use_result.clone(),
                source_tool_assistant_uuid: (*source_tool_assistant_uuid).to_string(),
            };
            writeln!(file, "{}", serde_json::to_string(&tool_result_line)?)?;
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

/// Append an assistant message to a JSONL file.
pub fn append_assistant_message_jsonl(
    path: &Path,
    params: &AssistantMessageParams,
) -> std::io::Result<()> {
    let mut file = open_append(path)?;
    let timestamp_str = params.timestamp.to_rfc3339();

    let assistant_line = AssistantMessageLine {
        parent_uuid: params.parent_uuid.to_string(),
        is_sidechain: false,
        user_type: "external".to_string(),
        cwd: params.cwd.to_string(),
        session_id: params.session_id.to_string(),
        version: params.version.to_string(),
        git_branch: params.git_branch.to_string(),
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
        line_type: "assistant",
        uuid: params.assistant_uuid.to_string(),
        timestamp: timestamp_str,
    };
    writeln!(file, "{}", serde_json::to_string(&assistant_line)?)?;

    Ok(())
}

#[cfg(test)]
#[path = "jsonl_tests.rs"]
mod tests;
