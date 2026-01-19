// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! MCP configuration file parsing.
//!
//! Supports parsing MCP configuration files in JSON and JSON5 formats,
//! compatible with Claude's `--mcp-config` flag.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// MCP configuration file root.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfig {
    /// Server definitions by name.
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerDef>,
}

/// Definition of an MCP server from configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerDef {
    /// Command to execute.
    pub command: String,

    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Optional working directory.
    #[serde(default)]
    pub cwd: Option<String>,

    /// Timeout in milliseconds.
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    30000
}

impl Default for McpServerDef {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            timeout_ms: default_timeout(),
        }
    }
}

/// Tool definition extracted from MCP server.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpToolDef {
    /// Tool name (e.g., "read_file", "write_file").
    pub name: String,

    /// Human-readable description.
    pub description: String,

    /// JSON Schema for input parameters.
    pub input_schema: serde_json::Value,

    /// Which server provides this tool.
    pub server_name: String,
}

impl McpConfig {
    /// Load from file path (supports JSON and JSON5).
    pub fn load(path: &Path) -> Result<Self, McpConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| McpConfigError::Io(path.display().to_string(), e.to_string()))?;

        Self::parse(&content)
    }

    /// Parse from string content.
    pub fn parse(content: &str) -> Result<Self, McpConfigError> {
        // Try JSON5 first (supports comments), fall back to strict JSON
        json5::from_str(content)
            .or_else(|_| serde_json::from_str(content))
            .map_err(|e| McpConfigError::Parse(e.to_string()))
    }

    /// Load from JSON string (for --mcp-config inline JSON).
    pub fn from_json_str(json: &str) -> Result<Self, McpConfigError> {
        serde_json::from_str(json).map_err(|e| McpConfigError::Parse(e.to_string()))
    }

    /// Merge multiple configs (later configs override earlier).
    pub fn merge(configs: impl IntoIterator<Item = Self>) -> Self {
        let mut merged = Self::default();
        for config in configs {
            merged.mcp_servers.extend(config.mcp_servers);
        }
        merged
    }

    /// Get server names.
    pub fn server_names(&self) -> Vec<&str> {
        self.mcp_servers.keys().map(|s| s.as_str()).collect()
    }

    /// Check if config has any servers.
    pub fn has_servers(&self) -> bool {
        !self.mcp_servers.is_empty()
    }
}

/// Errors that can occur when loading MCP configuration.
#[derive(Debug, thiserror::Error)]
pub enum McpConfigError {
    #[error("Failed to read MCP config from {0}: {1}")]
    Io(String, String),

    #[error("Failed to parse MCP config: {0}")]
    Parse(String),

    #[error("Invalid MCP server definition: {0}")]
    InvalidServer(String),
}

/// Load MCP configuration from a path or inline JSON string.
///
/// Determines whether the input is a file path or inline JSON based on content.
pub fn load_mcp_config(input: &str) -> Result<McpConfig, McpConfigError> {
    // If it looks like JSON (starts with { or [), parse as inline
    let trimmed = input.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        McpConfig::parse(trimmed)
    } else {
        // Treat as file path
        McpConfig::load(Path::new(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_config() {
        let json = r#"
        {
            "mcpServers": {
                "test": {
                    "command": "node",
                    "args": ["server.js"]
                }
            }
        }
        "#;

        let config = McpConfig::parse(json).unwrap();
        assert!(config.mcp_servers.contains_key("test"));
        assert_eq!(config.mcp_servers["test"].command, "node");
        assert_eq!(config.mcp_servers["test"].args, vec!["server.js"]);
    }

    #[test]
    fn test_parse_json5_with_comments() {
        let json5 = r#"
        {
            // This is a comment
            "mcpServers": {
                "test": {
                    "command": "node",
                    "args": ["server.js"], // trailing comma OK
                }
            }
        }
        "#;

        let config = McpConfig::parse(json5).unwrap();
        assert!(config.mcp_servers.contains_key("test"));
    }

    #[test]
    fn test_merge_configs() {
        let config1 = McpConfig::parse(r#"{"mcpServers": {"a": {"command": "a"}}}"#).unwrap();
        let config2 = McpConfig::parse(r#"{"mcpServers": {"b": {"command": "b"}}}"#).unwrap();

        let merged = McpConfig::merge([config1, config2]);
        assert_eq!(merged.mcp_servers.len(), 2);
        assert!(merged.mcp_servers.contains_key("a"));
        assert!(merged.mcp_servers.contains_key("b"));
    }

    #[test]
    fn test_merge_override() {
        let config1 = McpConfig::parse(r#"{"mcpServers": {"a": {"command": "old"}}}"#).unwrap();
        let config2 = McpConfig::parse(r#"{"mcpServers": {"a": {"command": "new"}}}"#).unwrap();

        let merged = McpConfig::merge([config1, config2]);
        assert_eq!(merged.mcp_servers.len(), 1);
        assert_eq!(merged.mcp_servers["a"].command, "new");
    }

    #[test]
    fn test_env_var_in_config() {
        let json = r#"
        {
            "mcpServers": {
                "github": {
                    "command": "npx",
                    "env": {
                        "GITHUB_TOKEN": "${GITHUB_TOKEN}"
                    }
                }
            }
        }
        "#;

        let config = McpConfig::parse(json).unwrap();
        assert_eq!(
            config.mcp_servers["github"].env.get("GITHUB_TOKEN"),
            Some(&"${GITHUB_TOKEN}".to_string())
        );
    }

    #[test]
    fn test_default_timeout() {
        let json = r#"{"mcpServers": {"test": {"command": "node"}}}"#;
        let config = McpConfig::parse(json).unwrap();
        assert_eq!(config.mcp_servers["test"].timeout_ms, 30000);
    }

    #[test]
    fn test_custom_timeout() {
        let json = r#"{"mcpServers": {"test": {"command": "node", "timeoutMs": 60000}}}"#;
        let config = McpConfig::parse(json).unwrap();
        assert_eq!(config.mcp_servers["test"].timeout_ms, 60000);
    }

    #[test]
    fn test_server_names() {
        let json = r#"{"mcpServers": {"a": {"command": "a"}, "b": {"command": "b"}}}"#;
        let config = McpConfig::parse(json).unwrap();
        let names = config.server_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_empty_config() {
        let json = r#"{}"#;
        let config = McpConfig::parse(json).unwrap();
        assert!(!config.has_servers());
    }

    #[test]
    fn test_load_inline_json() {
        let json = r#"{"mcpServers": {"test": {"command": "echo"}}}"#;
        let config = load_mcp_config(json).unwrap();
        assert!(config.has_servers());
    }

    #[test]
    fn test_invalid_json_error() {
        let result = McpConfig::parse("not valid json");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("parse"));
    }
}
