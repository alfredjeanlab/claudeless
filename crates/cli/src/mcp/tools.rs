// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool definitions for simulated MCP servers.
//!
//! Provides common tool templates that can be used to quickly set up
//! simulated MCP servers with typical tool sets.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::config::McpToolDef;

/// Common MCP tool definition templates.
pub struct McpToolTemplates;

impl McpToolTemplates {
    /// Filesystem tools (common MCP pattern).
    pub fn filesystem_tools(server_name: &str) -> Vec<McpToolDef> {
        vec![
            McpToolDef {
                name: "read_file".into(),
                description: "Read the contents of a file".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "File path" }
                    },
                    "required": ["path"]
                }),
                server_name: server_name.into(),
            },
            McpToolDef {
                name: "write_file".into(),
                description: "Write content to a file".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string" }
                    },
                    "required": ["path", "content"]
                }),
                server_name: server_name.into(),
            },
            McpToolDef {
                name: "list_directory".into(),
                description: "List files in a directory".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                }),
                server_name: server_name.into(),
            },
        ]
    }

    /// GitHub tools (common MCP pattern).
    pub fn github_tools(server_name: &str) -> Vec<McpToolDef> {
        vec![
            McpToolDef {
                name: "create_issue".into(),
                description: "Create a GitHub issue".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "repo": { "type": "string" },
                        "title": { "type": "string" },
                        "body": { "type": "string" }
                    },
                    "required": ["repo", "title"]
                }),
                server_name: server_name.into(),
            },
            McpToolDef {
                name: "create_pull_request".into(),
                description: "Create a pull request".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "repo": { "type": "string" },
                        "title": { "type": "string" },
                        "head": { "type": "string" },
                        "base": { "type": "string" }
                    },
                    "required": ["repo", "title", "head", "base"]
                }),
                server_name: server_name.into(),
            },
        ]
    }

    /// Database tools.
    pub fn database_tools(server_name: &str) -> Vec<McpToolDef> {
        vec![
            McpToolDef {
                name: "query".into(),
                description: "Execute a database query".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "sql": { "type": "string", "description": "SQL query to execute" }
                    },
                    "required": ["sql"]
                }),
                server_name: server_name.into(),
            },
            McpToolDef {
                name: "list_tables".into(),
                description: "List database tables".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
                server_name: server_name.into(),
            },
        ]
    }
}

/// Tool call request (for scenario matching).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpToolCall {
    /// Tool name.
    pub name: String,

    /// Input arguments.
    pub arguments: Value,

    /// Server that handled the call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
}

/// Tool call result.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpToolResult {
    /// Result content.
    pub content: Value,

    /// Whether the call succeeded.
    #[serde(default = "default_success")]
    pub success: bool,

    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn default_success() -> bool {
    true
}

impl McpToolResult {
    /// Create a successful result.
    pub fn success(content: Value) -> Self {
        Self {
            content,
            success: true,
            error: None,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            content: Value::Null,
            success: false,
            error: Some(error.into()),
        }
    }
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
