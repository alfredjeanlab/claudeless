// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! MCP client for communicating with MCP servers.
//!
//! This module provides the high-level client interface that combines the transport layer
//! and protocol types into a cohesive API for managing MCP server communication.
//!
//! # Example
//!
//! ```no_run
//! use claudeless::mcp::client::McpClient;
//! use claudeless::mcp::config::McpServerDef;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let def = McpServerDef {
//!     command: "mcp-server".to_string(),
//!     ..Default::default()
//! };
//!
//! // Quick setup with connect_and_initialize
//! let client = McpClient::connect_and_initialize(&def).await?;
//!
//! // Or step-by-step for more control
//! let mut client = McpClient::connect(&def).await?;
//! let server_info = client.initialize().await?;
//! let tools = client.list_tools().await?;
//!
//! // Call a tool
//! let result = client.call_tool("echo", serde_json::json!({"msg": "hello"})).await?;
//!
//! // Shutdown when done
//! client.shutdown().await?;
//! # Ok(())
//! # }
//! ```

use serde::{de::DeserializeOwned, Serialize};

use super::config::McpServerDef;
use super::protocol::{
    InitializeParams, InitializeResult, ServerInfo, ToolCallParams, ToolCallResult, ToolInfo,
    ToolsListResult, PROTOCOL_VERSION,
};
use super::transport::{JsonRpcNotification, StdioTransport, TransportError};
use thiserror::Error;

/// Errors that can occur during MCP client operations.
#[derive(Debug, Error)]
pub enum ClientError {
    /// Transport-level error.
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    /// Failed to parse server response.
    #[error("invalid response: {0}")]
    InvalidResponse(String),

    /// Server returned unsupported protocol version.
    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(String),

    /// Client is not initialized.
    #[error("client not initialized")]
    NotInitialized,

    /// Client is already initialized.
    #[error("client already initialized")]
    AlreadyInitialized,

    /// Tool not found on server.
    #[error("tool not found: {0}")]
    ToolNotFound(String),
}

/// MCP client for communicating with a server.
///
/// Manages the full lifecycle of an MCP connection:
/// 1. Spawn server process via [`connect`](Self::connect)
/// 2. Initialize protocol via [`initialize`](Self::initialize)
/// 3. Discover tools via [`list_tools`](Self::list_tools)
/// 4. Execute tools via [`call_tool`](Self::call_tool)
/// 5. Clean shutdown via [`shutdown`](Self::shutdown)
#[derive(Debug)]
pub struct McpClient {
    /// Underlying transport for JSON-RPC communication.
    transport: StdioTransport,

    /// Server definition used to create this client.
    definition: McpServerDef,

    /// Server info received during initialization.
    server_info: Option<ServerInfo>,

    /// Cached list of available tools.
    tools: Vec<ToolInfo>,

    /// Whether the client has completed initialization.
    initialized: bool,

    /// Default timeout for requests in milliseconds.
    timeout_ms: u64,
}

impl McpClient {
    /// Spawn a server process and create a client.
    ///
    /// This only spawns the process. Call [`initialize`](Self::initialize)
    /// to complete the MCP handshake.
    pub async fn connect(def: &McpServerDef) -> Result<Self, ClientError> {
        let transport = StdioTransport::spawn(def).await?;

        Ok(Self {
            transport,
            definition: def.clone(),
            server_info: None,
            tools: Vec::new(),
            initialized: false,
            timeout_ms: def.timeout_ms,
        })
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    /// Check that the client is initialized, returning an error if not.
    fn require_initialized(&self) -> Result<(), ClientError> {
        if self.initialized {
            Ok(())
        } else {
            Err(ClientError::NotInitialized)
        }
    }

    /// Serialize params to JSON, mapping errors consistently.
    fn serialize_params<T: Serialize>(&self, params: &T) -> Result<serde_json::Value, ClientError> {
        serde_json::to_value(params).map_err(|e| ClientError::InvalidResponse(e.to_string()))
    }

    /// Deserialize a JSON response, mapping errors consistently.
    fn deserialize_response<T: DeserializeOwned>(
        &self,
        value: serde_json::Value,
    ) -> Result<T, ClientError> {
        serde_json::from_value(value).map_err(|e| ClientError::InvalidResponse(e.to_string()))
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /// Initialize the MCP protocol.
    ///
    /// Sends the `initialize` request and waits for server response.
    /// Must be called before `list_tools` or `call_tool`.
    pub async fn initialize(&mut self) -> Result<&ServerInfo, ClientError> {
        if self.initialized {
            return Err(ClientError::AlreadyInitialized);
        }

        let params = InitializeParams::default();
        let params_json = self.serialize_params(&params)?;

        let result = self
            .transport
            .request("initialize", Some(params_json), self.timeout_ms)
            .await?;

        let init_result: InitializeResult = self.deserialize_response(result)?;

        // Verify protocol version compatibility
        if init_result.protocol_version != PROTOCOL_VERSION {
            return Err(ClientError::UnsupportedVersion(
                init_result.protocol_version,
            ));
        }

        self.server_info = Some(init_result.server_info);
        self.initialized = true;

        // Send initialized notification (no response expected)
        self.send_initialized_notification().await?;

        // server_info is guaranteed to be Some since we just set it above
        Ok(self
            .server_info
            .as_ref()
            .unwrap_or_else(|| unreachable!("server_info was just set to Some")))
    }

    /// Send the `initialized` notification after successful init.
    async fn send_initialized_notification(&self) -> Result<(), ClientError> {
        let notification = JsonRpcNotification::new("notifications/initialized", None);
        self.transport.send_notification(&notification).await?;
        Ok(())
    }

    /// Discover available tools from the server.
    ///
    /// Calls the `tools/list` method and caches the results.
    /// Can be called multiple times to refresh the tool list.
    pub async fn list_tools(&mut self) -> Result<&[ToolInfo], ClientError> {
        self.require_initialized()?;

        let result = self
            .transport
            .request("tools/list", None, self.timeout_ms)
            .await?;

        let tools_result: ToolsListResult = self.deserialize_response(result)?;

        self.tools = tools_result.tools;
        Ok(&self.tools)
    }

    /// Get the cached tool list without making a request.
    pub fn tools(&self) -> &[ToolInfo] {
        &self.tools
    }

    /// Check if a tool is available by name.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name == name)
    }

    /// Get tool info by name.
    pub fn get_tool(&self, name: &str) -> Option<&ToolInfo> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// Execute a tool call using the default client timeout.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolCallResult, ClientError> {
        self.call_tool_with_timeout(name, arguments, self.timeout_ms)
            .await
    }

    /// Execute a tool call with a custom timeout.
    pub async fn call_tool_with_timeout(
        &self,
        name: &str,
        arguments: serde_json::Value,
        timeout_ms: u64,
    ) -> Result<ToolCallResult, ClientError> {
        self.require_initialized()?;

        let params = ToolCallParams {
            name: name.to_string(),
            arguments: Some(arguments),
        };
        let params_json = self.serialize_params(&params)?;

        let result = self
            .transport
            .request("tools/call", Some(params_json), timeout_ms)
            .await?;

        self.deserialize_response(result)
    }

    /// Gracefully shut down the client.
    ///
    /// Closes the transport and terminates the server process.
    /// After shutdown, the client cannot be used again.
    pub async fn shutdown(self) -> Result<(), ClientError> {
        self.transport.shutdown().await?;
        Ok(())
    }

    /// Check if the client is initialized and ready for tool operations.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Check if the server process is still running.
    pub async fn is_running(&self) -> bool {
        self.transport.is_running().await
    }

    /// Get the server info (available after initialization).
    pub fn server_info(&self) -> Option<&ServerInfo> {
        self.server_info.as_ref()
    }

    /// Get the server definition used to create this client.
    pub fn definition(&self) -> &McpServerDef {
        &self.definition
    }

    /// Connect to a server and initialize in one step.
    ///
    /// Convenience method that combines [`connect`](Self::connect),
    /// [`initialize`](Self::initialize), and [`list_tools`](Self::list_tools).
    ///
    /// Returns an initialized client with tools already discovered.
    pub async fn connect_and_initialize(def: &McpServerDef) -> Result<Self, ClientError> {
        let mut client = Self::connect(def).await?;
        client.initialize().await?;
        client.list_tools().await?;
        Ok(client)
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
