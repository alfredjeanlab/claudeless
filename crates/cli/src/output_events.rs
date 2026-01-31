// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Event types matching real Claude CLI output format.

use crate::event_types::{content_block, line_type, mcp_status, subtype};
use serde::{Deserialize, Serialize};

/// Generate a deterministic UUID-like stub for testing.
fn uuid_stub() -> String {
    "01234567890abcdef".to_string()
}

/// MCP server info for init event (matches real Claude CLI format).
///
/// Real Claude CLI outputs mcp_servers as an array of objects:
/// ```json
/// {"mcp_servers": [{"name": "filesystem", "status": "connected"}]}
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct McpServerInfo {
    /// Server name.
    pub name: String,
    /// Server status: "connected", "failed", or "disconnected".
    pub status: String,
}

impl McpServerInfo {
    /// Create a connected server info.
    pub fn connected(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: mcp_status::CONNECTED.to_string(),
        }
    }

    /// Create a failed server info.
    pub fn failed(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: mcp_status::FAILED.to_string(),
        }
    }

    /// Create a disconnected server info.
    pub fn disconnected(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: mcp_status::DISCONNECTED.to_string(),
        }
    }
}

/// System init event for stream-json
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemInitEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub subtype: String,
    pub session_id: String,
    pub tools: Vec<String>,
    pub mcp_servers: Vec<McpServerInfo>,
}

impl SystemInitEvent {
    pub fn new(session_id: impl Into<String>, tools: Vec<String>) -> Self {
        Self {
            event_type: line_type::SYSTEM.to_string(),
            subtype: subtype::INIT.to_string(),
            session_id: session_id.into(),
            tools,
            mcp_servers: vec![],
        }
    }

    /// Create with MCP servers included.
    pub fn with_mcp_servers(
        session_id: impl Into<String>,
        tools: Vec<String>,
        mcp_servers: Vec<McpServerInfo>,
    ) -> Self {
        Self {
            event_type: line_type::SYSTEM.to_string(),
            subtype: subtype::INIT.to_string(),
            session_id: session_id.into(),
            tools,
            mcp_servers,
        }
    }
}

/// Assistant message event for stream-json
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssistantEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub subtype: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<AssistantMessageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ExtendedUsage>,
}

impl AssistantEvent {
    pub fn message_start(message: AssistantMessageContent) -> Self {
        Self {
            event_type: line_type::ASSISTANT.to_string(),
            subtype: subtype::MESSAGE_START.to_string(),
            message: Some(message),
            usage: None,
        }
    }

    pub fn message_delta(usage: ExtendedUsage) -> Self {
        Self {
            event_type: line_type::ASSISTANT.to_string(),
            subtype: subtype::MESSAGE_DELTA.to_string(),
            message: None,
            usage: Some(usage),
        }
    }

    pub fn message_stop() -> Self {
        Self {
            event_type: line_type::ASSISTANT.to_string(),
            subtype: subtype::MESSAGE_STOP.to_string(),
            message: None,
            usage: None,
        }
    }
}

/// Content of assistant message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssistantMessageContent {
    pub id: String,
    pub model: String,
    pub role: String,
    pub content: Vec<serde_json::Value>,
    pub stop_reason: Option<String>,
}

/// Condensed assistant event for stream-json (matches real Claude output)
/// This is the format used in real Claude CLI output - no subtype, includes full message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CondensedAssistantEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub message: CondensedMessage,
    pub session_id: String,
    pub uuid: String,
}

impl CondensedAssistantEvent {
    pub fn new(message: CondensedMessage, session_id: impl Into<String>) -> Self {
        Self {
            event_type: line_type::ASSISTANT.to_string(),
            message,
            session_id: session_id.into(),
            uuid: uuid_stub(),
        }
    }
}

/// Message content for condensed assistant event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CondensedMessage {
    pub id: String,
    pub model: String,
    pub role: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub content: serde_json::Value,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: serde_json::Value,
}

/// Extended usage info matching real Claude
pub use crate::usage::ExtendedTokenCounts as ExtendedUsage;

/// Content block start event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentBlockStartEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub subtype: String,
    pub index: u32,
}

impl ContentBlockStartEvent {
    pub fn text(index: u32) -> Self {
        Self {
            event_type: content_block::START.to_string(),
            subtype: subtype::TEXT.to_string(),
            index,
        }
    }

    pub fn tool_use(index: u32) -> Self {
        Self {
            event_type: content_block::START.to_string(),
            subtype: subtype::TOOL_USE.to_string(),
            index,
        }
    }
}

/// Content block delta event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentBlockDeltaEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub subtype: String,
    pub index: u32,
    pub delta: String,
}

impl ContentBlockDeltaEvent {
    pub fn text(index: u32, delta: impl Into<String>) -> Self {
        Self {
            event_type: content_block::DELTA.to_string(),
            subtype: subtype::TEXT_DELTA.to_string(),
            index,
            delta: delta.into(),
        }
    }
}

/// Content block stop event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentBlockStopEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub index: u32,
}

impl ContentBlockStopEvent {
    pub fn new(index: u32) -> Self {
        Self {
            event_type: content_block::STOP.to_string(),
            index,
        }
    }
}
