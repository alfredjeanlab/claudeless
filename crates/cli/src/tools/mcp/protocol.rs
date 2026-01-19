// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! MCP protocol types (JSON-RPC 2.0).
//!
//! Implements the Model Context Protocol specification (2025-11-25).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,

    /// Request ID for matching responses.
    pub id: u64,

    /// Method name.
    pub method: String,

    /// Method parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request.
    pub fn new(id: u64, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }

    /// Create an initialize request.
    pub fn initialize(id: u64, params: McpInitializeParams) -> Result<Self, serde_json::Error> {
        Ok(Self::new(
            id,
            "initialize",
            Some(serde_json::to_value(params)?),
        ))
    }

    /// Create a tools/list request.
    pub fn tools_list(id: u64) -> Self {
        Self::new(id, "tools/list", Some(serde_json::json!({})))
    }

    /// Create a tools/call request.
    pub fn tools_call(id: u64, params: McpToolCallParams) -> Result<Self, serde_json::Error> {
        Ok(Self::new(
            id,
            "tools/call",
            Some(serde_json::to_value(params)?),
        ))
    }
}

/// JSON-RPC 2.0 response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,

    /// Request ID this response corresponds to.
    pub id: u64,

    /// Result (on success).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error (on failure).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Check if this is an error response.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the result, if successful.
    pub fn result<T: for<'de> Deserialize<'de>>(&self) -> Result<T, String> {
        match &self.result {
            Some(v) => serde_json::from_value(v.clone())
                .map_err(|e| format!("Failed to deserialize result: {}", e)),
            None => Err(self
                .error
                .as_ref()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "No result".to_string())),
        }
    }
}

/// JSON-RPC 2.0 error.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i32,

    /// Error message.
    pub message: String,

    /// Additional error data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP initialize request parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInitializeParams {
    /// Protocol version.
    pub protocol_version: String,

    /// Client capabilities.
    pub capabilities: McpClientCapabilities,

    /// Client information.
    pub client_info: McpClientInfo,
}

impl Default for McpInitializeParams {
    fn default() -> Self {
        Self {
            protocol_version: "2025-11-25".to_string(),
            capabilities: McpClientCapabilities::default(),
            client_info: McpClientInfo::default(),
        }
    }
}

/// MCP client capabilities.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct McpClientCapabilities {
    /// Experimental capabilities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
}

/// MCP client information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpClientInfo {
    /// Client name.
    pub name: String,

    /// Client version.
    pub version: String,
}

impl Default for McpClientInfo {
    fn default() -> Self {
        Self {
            name: "claudeless".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// MCP initialize response result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInitializeResult {
    /// Protocol version.
    pub protocol_version: String,

    /// Server capabilities.
    pub capabilities: McpServerCapabilities,

    /// Server information.
    pub server_info: McpServerInfo,
}

/// MCP server capabilities.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct McpServerCapabilities {
    /// Tool capabilities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,

    /// Resource capabilities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<Value>,

    /// Prompt capabilities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Value>,
}

/// MCP server information.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct McpServerInfo {
    /// Server name.
    pub name: String,

    /// Server version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// MCP tools/list response result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpToolsListResult {
    /// Available tools.
    pub tools: Vec<McpToolInfo>,
}

/// MCP tool information.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolInfo {
    /// Tool name.
    pub name: String,

    /// Tool description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Input schema (JSON Schema).
    pub input_schema: Value,
}

/// MCP tools/call request parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpToolCallParams {
    /// Tool name.
    pub name: String,

    /// Tool arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl McpToolCallParams {
    /// Create new tool call parameters.
    pub fn new(name: impl Into<String>, arguments: Option<Value>) -> Self {
        Self {
            name: name.into(),
            arguments,
        }
    }
}

/// MCP tools/call response result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallResult {
    /// Result content.
    pub content: Vec<McpContent>,

    /// Whether this is an error result.
    #[serde(default)]
    pub is_error: bool,
}

/// MCP content item.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpContent {
    /// Text content.
    Text { text: String },

    /// Image content.
    Image { data: String, mime_type: String },

    /// Resource content.
    Resource { uri: String, text: Option<String> },
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
