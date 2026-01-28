// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! MCP (Model Context Protocol) client module.
//!
//! This module provides MCP client support for communicating with MCP servers.
//! It includes the transport layer for JSON-RPC over stdio, protocol types for
//! MCP messages, and the client interface for managing server lifecycle.
//!
//! Key features:
//! - Configuration file parsing (JSON and JSON5)
//! - JSON-RPC stdio transport layer
//! - MCP protocol message types
//! - Client for server lifecycle management
//! - Tool discovery and execution
//!
//! # Example
//!
//! ```ignore
//! use claudeless::mcp::client::McpClient;
//! use claudeless::mcp::config::McpServerDef;
//!
//! // Connect to an MCP server
//! let def = McpServerDef {
//!     command: "my-mcp-server".into(),
//!     ..Default::default()
//! };
//!
//! let client = McpClient::connect_and_initialize(&def).await?;
//! let result = client.call_tool("my_tool", serde_json::json!({})).await?;
//! client.shutdown().await?;
//! ```

pub mod client;
pub mod config;
pub mod protocol;
pub mod server;
pub mod tools;
pub mod transport;

pub use client::{ClientError, McpClient};
pub use config::{load_mcp_config, McpConfig, McpConfigError, McpServerDef, McpToolDef};
pub use protocol::{
    ClientCapabilities, ClientInfo, ContentBlock, InitializeParams, InitializeResult,
    ServerCapabilities, ServerInfo, ToolCallParams, ToolCallResult, ToolInfo, ToolsCapability,
    ToolsListResult, PROTOCOL_VERSION,
};
pub use server::{McpManager, McpServer, McpServerStatus};
pub use tools::{McpToolCall, McpToolResult, McpToolTemplates};
pub use transport::{
    JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, StdioTransport,
    TransportError,
};
