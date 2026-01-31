// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Event type string constants for JSONL and stream-json output formats.
//!
//! These constants provide a single source of truth for event type strings
//! used across output_events.rs and state/session/jsonl.rs.

/// Line/message type constants for envelope fields.
pub mod line_type {
    pub const SYSTEM: &str = "system";
    pub const USER: &str = "user";
    pub const ASSISTANT: &str = "assistant";
    pub const RESULT: &str = "result";
    pub const QUEUE_OPERATION: &str = "queue-operation";
}

/// Subtype constants for event subtypes.
pub mod subtype {
    pub const INIT: &str = "init";
    pub const MESSAGE_START: &str = "message_start";
    pub const MESSAGE_DELTA: &str = "message_delta";
    pub const MESSAGE_STOP: &str = "message_stop";
    pub const TEXT: &str = "text";
    pub const TEXT_DELTA: &str = "text_delta";
    pub const TOOL_USE: &str = "tool_use";
    pub const ERROR: &str = "error";
}

/// Content block event type constants.
pub mod content_block {
    pub const START: &str = "content_block_start";
    pub const DELTA: &str = "content_block_delta";
    pub const STOP: &str = "content_block_stop";
}

/// Role constants for message roles.
pub mod role {
    pub const USER: &str = "user";
    pub const ASSISTANT: &str = "assistant";
}

/// Message type constants.
pub mod message_type {
    pub const MESSAGE: &str = "message";
    pub const TOOL_RESULT: &str = "tool_result";
}

/// User type constants for envelope.
pub mod user_type {
    pub const EXTERNAL: &str = "external";
}

/// MCP server status constants.
pub mod mcp_status {
    pub const CONNECTED: &str = "connected";
    pub const FAILED: &str = "failed";
    pub const DISCONNECTED: &str = "disconnected";
}
