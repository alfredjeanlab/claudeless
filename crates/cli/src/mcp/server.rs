// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! MCP server state and management.
//!
//! This module provides MCP server functionality, managing server connections
//! and routing tool calls through the client layer.

use super::client::{ClientError, McpClient};
use super::config::{McpConfig, McpServerDef, McpToolDef};
use super::tools::McpToolResult;
use super::transport::TransportError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// MCP server with optional live client connection.
#[derive(Debug)]
pub struct McpServer {
    /// Server name.
    pub name: String,

    /// Server definition from config.
    pub definition: McpServerDef,

    /// Tools provided by this server.
    pub tools: Vec<McpToolDef>,

    /// Server status.
    pub status: McpServerStatus,

    /// Live client connection (None until spawned).
    client: Option<Arc<Mutex<McpClient>>>,
}

/// Status of an MCP server.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum McpServerStatus {
    /// Server not yet initialized.
    #[default]
    Uninitialized,
    /// Server running with active connection.
    Running,
    /// Server failed to start.
    Failed(String),
    /// Server disconnected.
    Disconnected,
}

impl McpServer {
    /// Create from definition.
    pub fn from_def(name: impl Into<String>, def: McpServerDef) -> Self {
        Self {
            name: name.into(),
            definition: def,
            tools: Vec::new(),
            status: McpServerStatus::Uninitialized,
            client: None,
        }
    }

    /// Spawn the MCP server process and initialize the connection.
    ///
    /// This spawns the actual MCP server process, initializes the MCP protocol,
    /// and discovers available tools via `tools/list`.
    ///
    /// # Arguments
    ///
    /// * `debug` - Enable JSON-RPC debug logging to stderr
    pub async fn spawn(&mut self, debug: bool) -> Result<(), ClientError> {
        // Validate definition
        if self.definition.command.is_empty() {
            return Err(ClientError::Transport(TransportError::Spawn(
                "No command specified".into(),
            )));
        }

        // Connect, initialize, and discover tools
        let client = McpClient::connect_and_initialize(&self.definition, &self.name, debug).await?;

        // Convert discovered tools to McpToolDef
        self.tools = client
            .tools()
            .iter()
            .map(|t| t.clone().into_tool_def(&self.name))
            .collect();

        self.client = Some(Arc::new(Mutex::new(client)));
        self.status = McpServerStatus::Running;
        Ok(())
    }

    /// Check if the server has an active client connection.
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Get the client (for internal use).
    // NOTE(compat): Reserved for future use by McpManager or advanced scenarios
    #[allow(dead_code)]
    pub(crate) fn client(&self) -> Option<&Arc<Mutex<McpClient>>> {
        self.client.as_ref()
    }

    /// Execute a tool call on this server.
    ///
    /// Returns error if server is not connected or tool execution fails.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, ClientError> {
        let client = self.client.as_ref().ok_or(ClientError::NotInitialized)?;
        let guard = client.lock().await;
        let result = guard.call_tool(name, arguments).await?;
        Ok(result.into_tool_result())
    }

    /// Execute a tool call with custom timeout.
    pub async fn call_tool_with_timeout(
        &self,
        name: &str,
        arguments: serde_json::Value,
        timeout_ms: u64,
    ) -> Result<McpToolResult, ClientError> {
        let client = self.client.as_ref().ok_or(ClientError::NotInitialized)?;
        let guard = client.lock().await;
        let result = guard
            .call_tool_with_timeout(name, arguments, timeout_ms)
            .await?;
        Ok(result.into_tool_result())
    }

    /// Shutdown the server connection gracefully.
    ///
    /// Takes ownership of the client and shuts it down. After this call,
    /// the server status changes to Disconnected and `call_tool` will fail.
    pub async fn shutdown(&mut self) -> Result<(), ClientError> {
        if let Some(client_arc) = self.client.take() {
            // Try to unwrap the Arc; if other references exist, we can't shutdown cleanly
            match Arc::try_unwrap(client_arc) {
                Ok(mutex) => {
                    let client = mutex.into_inner();
                    client.shutdown().await?;
                }
                Err(_arc) => {
                    // Other references exist; just drop our handle
                    // The process will be killed when all references are dropped
                }
            }
        }
        self.status = McpServerStatus::Disconnected;
        Ok(())
    }

    /// Register a tool with this server.
    pub fn register_tool(&mut self, tool: McpToolDef) {
        self.tools.push(tool);
    }

    /// Simulate server startup (for testing without real process).
    pub fn start(&mut self) {
        self.status = McpServerStatus::Running;
    }

    /// Simulate server failure.
    pub fn fail(&mut self, reason: impl Into<String>) {
        self.status = McpServerStatus::Failed(reason.into());
    }

    /// Simulate server disconnection.
    pub fn disconnect(&mut self) {
        self.status = McpServerStatus::Disconnected;
    }

    /// Check if server is running.
    pub fn is_running(&self) -> bool {
        self.status == McpServerStatus::Running
    }

    /// Get tool names provided by this server.
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name.as_str()).collect()
    }
}

/// MCP server manager.
///
/// Manages multiple MCP servers and their tools.
#[derive(Debug, Default)]
pub struct McpManager {
    /// Active servers by name.
    servers: HashMap<String, McpServer>,

    /// Tool to server mapping.
    tool_server_map: HashMap<String, String>,
}

impl McpManager {
    /// Create empty manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize from config (does not spawn servers).
    ///
    /// Call [`initialize()`](Self::initialize) to spawn server processes.
    pub fn from_config(config: &McpConfig) -> Self {
        let mut manager = Self::new();

        for (name, def) in &config.mcp_servers {
            let server = McpServer::from_def(name, def.clone());
            // Don't auto-start; let caller call initialize()
            manager.servers.insert(name.clone(), server);
        }

        manager
    }

    /// Initialize all servers by spawning their processes.
    ///
    /// Returns a list of (server_name, result) pairs. Servers that fail to
    /// initialize are marked as Failed but remain in the manager.
    ///
    /// # Arguments
    ///
    /// * `debug` - Enable JSON-RPC debug logging to stderr
    pub async fn initialize(&mut self, debug: bool) -> Vec<(String, Result<(), ClientError>)> {
        let mut results = Vec::new();

        // Collect server names to avoid borrow issues
        let names: Vec<String> = self.servers.keys().cloned().collect();

        for name in names {
            let result = if let Some(server) = self.servers.get_mut(&name) {
                match server.spawn(debug).await {
                    Ok(()) => {
                        // Register discovered tools in the mapping
                        for tool in &server.tools {
                            self.tool_server_map.insert(tool.name.clone(), name.clone());
                        }
                        Ok(())
                    }
                    Err(e) => {
                        server.status = McpServerStatus::Failed(e.to_string());
                        Err(e)
                    }
                }
            } else {
                continue;
            };
            results.push((name, result));
        }

        results
    }

    /// Execute a tool call, routing to the appropriate server.
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, ClientError> {
        let server_name = self
            .tool_server_map
            .get(tool_name)
            .ok_or_else(|| ClientError::ToolNotFound(tool_name.to_string()))?;
        let server = self.servers.get(server_name).ok_or_else(|| {
            ClientError::ToolNotFound(format!("server '{}' not found", server_name))
        })?;
        server.call_tool(tool_name, arguments).await
    }

    /// Shutdown all server connections gracefully.
    pub async fn shutdown(&mut self) {
        for server in self.servers.values_mut() {
            let _ = server.shutdown().await;
        }
    }

    /// Add a server to the manager.
    pub fn add_server(&mut self, server: McpServer) {
        self.servers.insert(server.name.clone(), server);
    }

    /// Get a server by name.
    pub fn get_server(&self, name: &str) -> Option<&McpServer> {
        self.servers.get(name)
    }

    /// Get a mutable server by name.
    pub fn get_server_mut(&mut self, name: &str) -> Option<&mut McpServer> {
        self.servers.get_mut(name)
    }

    /// Register a tool for a server.
    pub fn register_tool(&mut self, server_name: &str, tool: McpToolDef) {
        self.tool_server_map
            .insert(tool.name.clone(), server_name.to_string());
        if let Some(server) = self.servers.get_mut(server_name) {
            server.register_tool(tool);
        }
    }

    /// Get all available tools from running servers.
    pub fn tools(&self) -> Vec<&McpToolDef> {
        self.servers
            .values()
            .filter(|s| s.is_running())
            .flat_map(|s| &s.tools)
            .collect()
    }

    /// Get all servers.
    pub fn servers(&self) -> Vec<&McpServer> {
        self.servers.values().collect()
    }

    /// Get tool names for output.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools().iter().map(|t| t.name.clone()).collect()
    }

    /// Get server names for output.
    pub fn server_names(&self) -> Vec<String> {
        self.servers.keys().cloned().collect()
    }

    /// Get running server names.
    pub fn running_server_names(&self) -> Vec<String> {
        self.servers
            .iter()
            .filter(|(_, s)| s.is_running())
            .map(|(n, _)| n.clone())
            .collect()
    }

    /// Check if a tool exists.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tool_server_map.contains_key(name)
    }

    /// Get server for a tool.
    pub fn server_for_tool(&self, tool_name: &str) -> Option<&McpServer> {
        let server_name = self.tool_server_map.get(tool_name)?;
        self.servers.get(server_name)
    }

    /// Check if manager has any servers.
    pub fn has_servers(&self) -> bool {
        !self.servers.is_empty()
    }

    /// Get number of servers.
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// Get number of running servers.
    pub fn running_server_count(&self) -> usize {
        self.servers.values().filter(|s| s.is_running()).count()
    }
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
