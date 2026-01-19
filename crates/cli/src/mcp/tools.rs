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
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_tools() {
        let tools = McpToolTemplates::filesystem_tools("fs");
        assert_eq!(tools.len(), 3);

        let names: Vec<_> = tools.iter().map(|t| &t.name).collect();
        assert!(names.contains(&&"read_file".to_string()));
        assert!(names.contains(&&"write_file".to_string()));
        assert!(names.contains(&&"list_directory".to_string()));

        for tool in &tools {
            assert_eq!(tool.server_name, "fs");
        }
    }

    #[test]
    fn test_github_tools() {
        let tools = McpToolTemplates::github_tools("github");
        assert_eq!(tools.len(), 2);

        let names: Vec<_> = tools.iter().map(|t| &t.name).collect();
        assert!(names.contains(&&"create_issue".to_string()));
        assert!(names.contains(&&"create_pull_request".to_string()));
    }

    #[test]
    fn test_database_tools() {
        let tools = McpToolTemplates::database_tools("db");
        assert_eq!(tools.len(), 2);

        let names: Vec<_> = tools.iter().map(|t| &t.name).collect();
        assert!(names.contains(&&"query".to_string()));
        assert!(names.contains(&&"list_tables".to_string()));
    }

    #[test]
    fn test_tool_call_serialization() {
        let call = McpToolCall {
            name: "read_file".into(),
            arguments: serde_json::json!({"path": "/tmp/test.txt"}),
            server: Some("fs".into()),
        };

        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("read_file"));
        assert!(json.contains("/tmp/test.txt"));
    }

    #[test]
    fn test_tool_result_success() {
        let result = McpToolResult::success(serde_json::json!("file contents"));
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_result_failure() {
        let result = McpToolResult::failure("file not found");
        assert!(!result.success);
        assert_eq!(result.error, Some("file not found".into()));
    }
}
