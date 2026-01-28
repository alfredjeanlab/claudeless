// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! MCP protocol message types.
//!
//! This module defines the wire format for MCP (Model Context Protocol) client-server
//! communication. These types are used for JSON-RPC serialization/deserialization
//! when communicating with MCP servers.
//!
//! # Wire Format
//!
//! MCP uses JSON-RPC 2.0 as its transport format. The types in this module represent
//! the MCP-specific parameters and results that layer on top of JSON-RPC.
//!
//! # Type Bridging
//!
//! Protocol types are wire-format focused. Conversion methods bridge to internal types:
//! - [`ToolInfo`] converts to [`super::config::McpToolDef`]
//! - [`ToolCallResult`] converts to [`super::tools::McpToolResult`]

use serde::{Deserialize, Serialize};

use super::config::McpToolDef;
use super::tools::McpToolResult;

/// MCP protocol version we support.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

// =============================================================================
// Initialization Types
// =============================================================================

/// Client information sent during initialization.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    /// Client name.
    pub name: String,
    /// Client version.
    pub version: String,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            name: "claudeless".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

/// Client capabilities sent during initialization.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    /// We don't support any optional capabilities yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
}

/// Initialize request parameters.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// Protocol version the client supports.
    pub protocol_version: String,
    /// Client capabilities.
    pub capabilities: ClientCapabilities,
    /// Client information.
    pub client_info: ClientInfo,
}

impl Default for InitializeParams {
    fn default() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION.into(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo::default(),
        }
    }
}

// =============================================================================
// Server Response Types
// =============================================================================

/// Server information from initialize response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    /// Server name.
    pub name: String,
    /// Server version (optional).
    #[serde(default)]
    pub version: Option<String>,
}

/// Server capabilities from initialize response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Whether server supports tools.
    #[serde(default)]
    pub tools: Option<ToolsCapability>,
}

/// Tools capability details.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// Whether tool list may change.
    #[serde(default)]
    pub list_changed: bool,
}

/// Initialize response result.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Protocol version the server supports.
    pub protocol_version: String,
    /// Server capabilities.
    pub capabilities: ServerCapabilities,
    /// Server information.
    pub server_info: ServerInfo,
}

// =============================================================================
// Tool Discovery Types
// =============================================================================

/// Tool information from MCP server.
///
/// Note: This parallels [`McpToolDef`] in config.rs but represents
/// the wire format. Conversion methods bridge the two.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    /// Unique tool name.
    pub name: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// JSON Schema for tool input.
    pub input_schema: serde_json::Value,
}

impl ToolInfo {
    /// Convert to [`McpToolDef`] for internal use.
    pub fn into_tool_def(self, server_name: &str) -> McpToolDef {
        McpToolDef {
            name: self.name,
            description: self.description.unwrap_or_default(),
            input_schema: self.input_schema,
            server_name: server_name.into(),
        }
    }
}

/// Response from tools/list method.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolsListResult {
    /// List of tools available on the server.
    pub tools: Vec<ToolInfo>,
}

// =============================================================================
// Tool Execution Types
// =============================================================================

/// Parameters for tools/call method.
#[derive(Debug, Clone, Serialize)]
pub struct ToolCallParams {
    /// Tool name to invoke.
    pub name: String,

    /// Arguments matching the tool's input schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// Content block in tool response.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentBlock {
    /// Plain text content.
    Text {
        /// The text content.
        text: String,
    },

    /// Base64-encoded image.
    Image {
        /// Base64-encoded image data.
        data: String,
        /// MIME type of the image.
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    /// Resource reference.
    Resource {
        /// Resource URI.
        uri: String,
        /// Optional text content of the resource.
        #[serde(default)]
        text: Option<String>,
        /// Optional MIME type.
        #[serde(default, rename = "mimeType")]
        mime_type: Option<String>,
    },
}

impl ContentBlock {
    /// Extract text content if this is a text block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            ContentBlock::Text { text } => Some(text),
            _ => None,
        }
    }
}

/// Response from tools/call method.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    /// Content blocks returned by the tool.
    pub content: Vec<ContentBlock>,

    /// Whether the tool execution resulted in an error.
    #[serde(default)]
    pub is_error: bool,
}

impl ToolCallResult {
    /// Convert to [`McpToolResult`] for internal use.
    pub fn into_tool_result(self) -> McpToolResult {
        if self.is_error {
            let error_text = self
                .content
                .iter()
                .filter_map(|c| match c {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            McpToolResult::failure(error_text)
        } else {
            McpToolResult::success(serde_json::to_value(&self.content).unwrap_or_default())
        }
    }
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
