// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! MCP (Model Context Protocol) tool execution.
//!
//! This module provides real MCP server execution for integration testing.
//! It implements the full MCP protocol (JSON-RPC 2.0 over stdio) to spawn
//! and communicate with actual MCP servers.

mod client;
mod protocol;
mod transport;

pub use client::McpClient;
pub use protocol::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpContent, McpInitializeParams,
    McpInitializeResult, McpToolCallParams, McpToolCallResult, McpToolInfo, McpToolsListResult,
};
pub use transport::McpTransport;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::config::ToolCallSpec;
use crate::mcp::McpManager;
use crate::tools::executor::{ExecutionContext, ToolExecutor};
use crate::tools::result::ToolExecutionResult;

/// Executor that spawns real MCP servers for tool execution.
pub struct McpExecutor {
    /// MCP manager with server configurations.
    manager: Option<Arc<RwLock<McpManager>>>,
    /// Active MCP clients by server name.
    #[allow(dead_code)] // Will be used when MCP execution is fully implemented
    clients: HashMap<String, McpClient>,
}

impl Default for McpExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl McpExecutor {
    /// Create a new MCP executor.
    pub fn new() -> Self {
        Self {
            manager: None,
            clients: HashMap::new(),
        }
    }

    /// Set the MCP manager.
    pub fn with_manager(mut self, manager: Arc<RwLock<McpManager>>) -> Self {
        self.manager = Some(manager);
        self
    }

    /// Get or create a client for a server.
    #[allow(dead_code)] // Will be used when MCP execution is fully implemented
    fn get_or_create_client(&mut self, server_name: &str) -> Result<&mut McpClient, String> {
        if self.clients.contains_key(server_name) {
            return Ok(self.clients.get_mut(server_name).unwrap());
        }

        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| "No MCP manager configured".to_string())?;

        let manager_guard = manager
            .read()
            .map_err(|e| format!("Failed to acquire manager lock: {}", e))?;

        let server = manager_guard
            .get_server(server_name)
            .ok_or_else(|| format!("MCP server '{}' not found", server_name))?;

        // Spawn the MCP server process
        let client = McpClient::spawn(&server.definition)
            .map_err(|e| format!("Failed to spawn MCP server '{}': {}", server_name, e))?;

        drop(manager_guard);

        self.clients.insert(server_name.to_string(), client);
        Ok(self.clients.get_mut(server_name).unwrap())
    }

    /// Find which server provides a tool.
    fn find_server_for_tool(&self, tool_name: &str) -> Option<String> {
        let manager = self.manager.as_ref()?;
        let manager_guard = manager.read().ok()?;

        manager_guard
            .server_for_tool(tool_name)
            .map(|s| s.name.clone())
    }
}

impl ToolExecutor for McpExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        _ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        // First check for pre-configured mock result
        if let Some(result) = &call.result {
            return ToolExecutionResult::success(tool_use_id, result);
        }

        // Find which server provides this tool
        let server_name = match self.find_server_for_tool(&call.tool) {
            Some(name) => name,
            None => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    format!(
                        "No MCP server found for tool '{}'. \
                         Configure an MCP server or provide a mock result.",
                        call.tool
                    ),
                )
            }
        };

        // For now, return an error indicating real MCP execution is not yet fully implemented
        // This will be completed in Phase 4
        ToolExecutionResult::error(
            tool_use_id,
            format!(
                "Real MCP execution not yet implemented. Tool '{}' would be executed by server '{}'.",
                call.tool, server_name
            ),
        )
    }

    fn name(&self) -> &'static str {
        "mcp"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_executor_with_mock_result() {
        let executor = McpExecutor::new();
        let call = ToolCallSpec {
            tool: "read_file".to_string(),
            input: json!({ "path": "/tmp/test.txt" }),
            result: Some("file contents".to_string()),
        };
        let ctx = ExecutionContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(!result.is_error);
        assert_eq!(result.text(), Some("file contents"));
    }

    #[test]
    fn test_mcp_executor_no_manager() {
        let executor = McpExecutor::new();
        let call = ToolCallSpec {
            tool: "read_file".to_string(),
            input: json!({ "path": "/tmp/test.txt" }),
            result: None,
        };
        let ctx = ExecutionContext::default();
        let result = executor.execute(&call, "toolu_123", &ctx);

        assert!(result.is_error);
        assert!(result.text().unwrap().contains("No MCP server found"));
    }

    #[test]
    fn test_executor_name() {
        assert_eq!(McpExecutor::new().name(), "mcp");
    }
}
