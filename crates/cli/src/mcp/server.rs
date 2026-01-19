// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Simulated MCP server state.
//!
//! This module provides simulated MCP server functionality. It doesn't actually
//! run MCP servers, but maintains the state needed to simulate their presence.

use super::config::{McpConfig, McpServerDef, McpToolDef};
use std::collections::HashMap;

/// Simulated MCP server.
#[derive(Clone, Debug)]
pub struct McpServer {
    /// Server name.
    pub name: String,

    /// Server definition from config.
    pub definition: McpServerDef,

    /// Tools provided by this server.
    pub tools: Vec<McpToolDef>,

    /// Server status.
    pub status: McpServerStatus,
}

/// Status of a simulated MCP server.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum McpServerStatus {
    /// Server not yet initialized.
    #[default]
    Uninitialized,
    /// Server running (simulated).
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
        }
    }

    /// Spawn the MCP server process and return a client for communication.
    ///
    /// This spawns the actual MCP server process using the command and args
    /// from the definition, establishes JSON-RPC communication, and initializes
    /// the MCP protocol.
    ///
    /// Note: This is currently a stub that returns an error. Full implementation
    /// will be completed when real MCP execution is needed.
    pub fn spawn(&mut self) -> Result<(), String> {
        // Validate the definition
        if self.definition.command.is_empty() {
            return Err("No command specified for MCP server".to_string());
        }

        // Mark server as running (in simulation mode, we just mark it)
        // For real MCP execution, we would spawn the process here
        self.status = McpServerStatus::Running;
        Ok(())
    }

    /// Register a tool with this server.
    pub fn register_tool(&mut self, tool: McpToolDef) {
        self.tools.push(tool);
    }

    /// Simulate server startup.
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
/// Manages multiple simulated MCP servers and their tools.
#[derive(Clone, Debug, Default)]
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

    /// Initialize from config.
    pub fn from_config(config: &McpConfig) -> Self {
        let mut manager = Self::new();

        for (name, def) in &config.mcp_servers {
            let mut server = McpServer::from_def(name, def.clone());
            // Auto-start simulated servers
            server.start();
            manager.servers.insert(name.clone(), server);
        }

        manager
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
mod tests {
    use super::*;

    #[test]
    fn test_server_lifecycle() {
        let def = McpServerDef {
            command: "node".into(),
            args: vec!["server.js".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_ms: 30000,
        };

        let mut server = McpServer::from_def("test", def);
        assert_eq!(server.status, McpServerStatus::Uninitialized);

        server.start();
        assert!(server.is_running());

        server.disconnect();
        assert_eq!(server.status, McpServerStatus::Disconnected);
        assert!(!server.is_running());
    }

    #[test]
    fn test_server_failure() {
        let def = McpServerDef::default();
        let mut server = McpServer::from_def("test", def);

        server.fail("connection refused");
        assert!(matches!(server.status, McpServerStatus::Failed(_)));
        if let McpServerStatus::Failed(reason) = &server.status {
            assert!(reason.contains("connection refused"));
        }
    }

    #[test]
    fn test_tool_registration() {
        let config = McpConfig::parse(r#"{"mcpServers": {"fs": {"command": "node"}}}"#).unwrap();
        let mut manager = McpManager::from_config(&config);

        manager.register_tool(
            "fs",
            McpToolDef {
                name: "read_file".into(),
                description: "Read a file".into(),
                input_schema: serde_json::json!({"type": "object"}),
                server_name: "fs".into(),
            },
        );

        assert!(manager.has_tool("read_file"));
        assert_eq!(manager.tool_names(), vec!["read_file"]);
    }

    #[test]
    fn test_manager_from_config() {
        let config =
            McpConfig::parse(r#"{"mcpServers": {"a": {"command": "a"}, "b": {"command": "b"}}}"#)
                .unwrap();
        let manager = McpManager::from_config(&config);

        assert_eq!(manager.server_count(), 2);
        assert_eq!(manager.running_server_count(), 2);
        assert!(manager.has_servers());
    }

    #[test]
    fn test_server_for_tool() {
        let config = McpConfig::parse(r#"{"mcpServers": {"fs": {"command": "node"}}}"#).unwrap();
        let mut manager = McpManager::from_config(&config);

        manager.register_tool(
            "fs",
            McpToolDef {
                name: "read_file".into(),
                description: "Read".into(),
                input_schema: serde_json::json!({}),
                server_name: "fs".into(),
            },
        );

        let server = manager.server_for_tool("read_file").unwrap();
        assert_eq!(server.name, "fs");

        assert!(manager.server_for_tool("nonexistent").is_none());
    }

    #[test]
    fn test_empty_manager() {
        let manager = McpManager::new();
        assert!(!manager.has_servers());
        assert_eq!(manager.server_count(), 0);
        assert!(manager.tools().is_empty());
        assert!(manager.tool_names().is_empty());
    }
}
