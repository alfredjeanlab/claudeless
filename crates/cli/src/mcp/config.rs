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
    std::env::var("CLAUDELESS_MCP_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30000)
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

impl McpToolDef {
    /// Returns the qualified tool name in real Claude CLI format.
    ///
    /// Real Claude CLI uses the format `mcp__<server>__<tool>` for MCP tools.
    /// For example, a tool "read_file" from server "filesystem" becomes
    /// "mcp__filesystem__read_file".
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeless::mcp::config::McpToolDef;
    ///
    /// let tool = McpToolDef {
    ///     name: "read_file".into(),
    ///     description: "Read a file".into(),
    ///     input_schema: serde_json::json!({}),
    ///     server_name: "filesystem".into(),
    /// };
    /// assert_eq!(tool.qualified_name(), "mcp__filesystem__read_file");
    /// ```
    pub fn qualified_name(&self) -> String {
        format!("mcp__{}__{}", self.server_name, self.name)
    }

    /// Parse a qualified tool name back into server and tool names.
    ///
    /// Returns `Some((server_name, tool_name))` if the name matches the
    /// `mcp__<server>__<tool>` format, or `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeless::mcp::config::McpToolDef;
    ///
    /// let parts = McpToolDef::parse_qualified_name("mcp__filesystem__read_file");
    /// assert_eq!(parts, Some(("filesystem".to_string(), "read_file".to_string())));
    ///
    /// let parts = McpToolDef::parse_qualified_name("Read");
    /// assert_eq!(parts, None);
    /// ```
    pub fn parse_qualified_name(qualified: &str) -> Option<(String, String)> {
        if !qualified.starts_with("mcp__") {
            return None;
        }
        let rest = &qualified[5..]; // Skip "mcp__"
        let parts: Vec<&str> = rest.splitn(2, "__").collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }
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
#[path = "config_tests.rs"]
mod tests;
