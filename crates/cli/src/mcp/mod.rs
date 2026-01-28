// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! MCP (Model Context Protocol) simulation module.
//!
//! This module provides simulated MCP support for testing scenarios that involve
//! MCP tools. It does not implement actual MCP protocol communication, but
//! provides the necessary structures and configuration parsing to simulate
//! MCP server presence and tool availability.
//!
//! Key features:
//! - Configuration file parsing (JSON and JSON5)
//! - Server and tool state management
//! - Common tool templates for testing
//!
//! # Example
//!
//! ```
//! use claudeless::mcp::config::McpConfig;
//! use claudeless::mcp::server::McpManager;
//! use claudeless::mcp::tools::McpToolTemplates;
//!
//! // Parse MCP config
//! let config = McpConfig::parse(r#"{"mcpServers": {"fs": {"command": "node"}}}"#).unwrap();
//!
//! // Create manager and register tools
//! let mut manager = McpManager::from_config(&config);
//! for tool in McpToolTemplates::filesystem_tools("fs") {
//!     manager.register_tool("fs", tool);
//! }
//!
//! // Check tool availability
//! assert!(manager.has_tool("read_file"));
//! ```

pub mod config;
pub mod server;
pub mod tools;
pub mod transport;

pub use config::{load_mcp_config, McpConfig, McpConfigError, McpServerDef, McpToolDef};
pub use server::{McpManager, McpServer, McpServerStatus};
pub use tools::{McpToolCall, McpToolResult, McpToolTemplates};
pub use transport::{
    JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, StdioTransport,
    TransportError,
};
