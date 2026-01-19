// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! MCP client for communicating with MCP servers.

use super::protocol::{
    JsonRpcRequest, McpInitializeParams, McpInitializeResult, McpToolCallParams, McpToolCallResult,
    McpToolInfo, McpToolsListResult,
};
use super::transport::{McpTransport, McpTransportError};
use crate::mcp::McpServerDef;

/// MCP client for a single server.
pub struct McpClient {
    /// Transport layer.
    transport: McpTransport,

    /// Server name (for debugging).
    server_name: String,

    /// Whether the client has been initialized.
    initialized: bool,

    /// Available tools (cached after tools/list).
    tools: Vec<McpToolInfo>,
}

impl McpClient {
    /// Spawn a new MCP client connected to a server.
    pub fn spawn(def: &McpServerDef) -> Result<Self, McpClientError> {
        let transport = McpTransport::spawn(def)?;

        let mut client = Self {
            transport,
            server_name: def.command.clone(),
            initialized: false,
            tools: Vec::new(),
        };

        // Initialize the connection
        client.initialize()?;

        // List available tools
        client.list_tools()?;

        Ok(client)
    }

    /// Initialize the MCP connection.
    fn initialize(&mut self) -> Result<McpInitializeResult, McpClientError> {
        let id = self.transport.next_id();
        let request = JsonRpcRequest::initialize(id, McpInitializeParams::default())?;

        let response = self.transport.request(request)?;

        if response.is_error() {
            return Err(McpClientError::InitializeFailed(
                response
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let result: McpInitializeResult = response.result()?;
        self.initialized = true;

        Ok(result)
    }

    /// List available tools from the server.
    fn list_tools(&mut self) -> Result<Vec<McpToolInfo>, McpClientError> {
        if !self.initialized {
            return Err(McpClientError::NotInitialized);
        }

        let id = self.transport.next_id();
        let request = JsonRpcRequest::tools_list(id);

        let response = self.transport.request(request)?;

        if response.is_error() {
            return Err(McpClientError::ToolsListFailed(
                response
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let result: McpToolsListResult = response.result()?;
        self.tools = result.tools.clone();

        Ok(result.tools)
    }

    /// Call a tool on the server.
    pub fn call_tool(
        &mut self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<McpToolCallResult, McpClientError> {
        if !self.initialized {
            return Err(McpClientError::NotInitialized);
        }

        let id = self.transport.next_id();
        let params = McpToolCallParams::new(name, arguments);
        let request = JsonRpcRequest::tools_call(id, params)?;

        let response = self.transport.request(request)?;

        if response.is_error() {
            return Err(McpClientError::ToolCallFailed {
                tool: name.to_string(),
                message: response
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| "Unknown error".to_string()),
            });
        }

        let result: McpToolCallResult = response.result()?;
        Ok(result)
    }

    /// Get the list of available tools.
    pub fn tools(&self) -> &[McpToolInfo] {
        &self.tools
    }

    /// Check if the server provides a specific tool.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name == name)
    }

    /// Check if the client is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the server name.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }
}

/// MCP client errors.
#[derive(Debug)]
pub enum McpClientError {
    /// Transport error.
    Transport(McpTransportError),

    /// Failed to initialize.
    InitializeFailed(String),

    /// Not initialized.
    NotInitialized,

    /// Failed to list tools.
    ToolsListFailed(String),

    /// Failed to call tool.
    ToolCallFailed { tool: String, message: String },

    /// Failed to parse response.
    ParseFailed(String),

    /// Failed to serialize request parameters.
    SerializeFailed(serde_json::Error),
}

impl From<McpTransportError> for McpClientError {
    fn from(err: McpTransportError) -> Self {
        Self::Transport(err)
    }
}

impl From<String> for McpClientError {
    fn from(err: String) -> Self {
        Self::ParseFailed(err)
    }
}

impl From<serde_json::Error> for McpClientError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializeFailed(err)
    }
}

impl std::fmt::Display for McpClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "Transport error: {}", e),
            Self::InitializeFailed(msg) => write!(f, "Initialize failed: {}", msg),
            Self::NotInitialized => write!(f, "Client not initialized"),
            Self::ToolsListFailed(msg) => write!(f, "Tools list failed: {}", msg),
            Self::ToolCallFailed { tool, message } => {
                write!(f, "Tool '{}' call failed: {}", tool, message)
            }
            Self::ParseFailed(msg) => write!(f, "Parse failed: {}", msg),
            Self::SerializeFailed(e) => write!(f, "Serialize failed: {}", e),
        }
    }
}

impl std::error::Error for McpClientError {}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
