// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Global settings management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Global settings
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    /// User-configured settings
    #[serde(flatten)]
    pub values: HashMap<String, serde_json::Value>,
}

impl Settings {
    /// Create empty settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Load settings from file
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save settings to file
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    /// Get a setting value
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.values.get(key)
    }

    /// Get a setting as string
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.values.get(key).and_then(|v| v.as_str())
    }

    /// Get a setting as bool
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.values.get(key).and_then(|v| v.as_bool())
    }

    /// Get a setting as i64
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.values.get(key).and_then(|v| v.as_i64())
    }

    /// Set a setting value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.values.insert(key.into(), value.into());
    }

    /// Remove a setting
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.values.remove(key)
    }

    /// Check if a setting exists
    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Clear all settings
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(|s| s.as_str())
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Try to parse as ClaudeSettings schema.
    pub fn as_claude_settings(&self) -> Option<ClaudeSettings> {
        // Convert the generic HashMap to the typed schema
        let json = serde_json::to_value(&self.values).ok()?;
        serde_json::from_value(json).ok()
    }
}

/// Claude Code permission settings schema.
///
/// Matches the structure of `permissions` in settings.json:
/// ```json
/// {
///   "permissions": {
///     "allow": ["Bash(npm test)", "Read"],
///     "deny": ["Bash(rm *)"],
///     "additionalDirectories": ["/tmp/workspace"]
///   }
/// }
/// ```
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionSettings {
    /// Tool patterns to auto-approve (skip permission prompt)
    #[serde(default)]
    pub allow: Vec<String>,

    /// Tool patterns to always reject
    #[serde(default)]
    pub deny: Vec<String>,

    /// Additional directories Claude can access beyond the project
    #[serde(default)]
    pub additional_directories: Vec<String>,
}

/// MCP server configuration (parse only, don't spawn).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    /// Command to spawn the server
    #[serde(default)]
    pub command: Option<String>,

    /// Arguments for the command
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables for the server
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Working directory
    #[serde(default)]
    pub cwd: Option<String>,
}

/// Full settings file schema.
///
/// This is permissive - unknown fields are ignored to handle
/// future Claude Code versions gracefully.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSettings {
    /// Permission configuration
    #[serde(default)]
    pub permissions: PermissionSettings,

    /// MCP server definitions
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,

    /// Environment variable overrides
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Capture unknown fields for forward compatibility
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ClaudeSettings {
    /// Load settings from a JSON file.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Merge another settings file on top of this one.
    /// Later values override earlier ones (array fields are replaced, not merged).
    pub fn merge(&mut self, other: Self) {
        // Permissions: replace arrays if non-empty in other
        if !other.permissions.allow.is_empty() {
            self.permissions.allow = other.permissions.allow;
        }
        if !other.permissions.deny.is_empty() {
            self.permissions.deny = other.permissions.deny;
        }
        if !other.permissions.additional_directories.is_empty() {
            self.permissions.additional_directories = other.permissions.additional_directories;
        }

        // MCP servers: merge maps (later overrides)
        for (name, config) in other.mcp_servers {
            self.mcp_servers.insert(name, config);
        }

        // Env: merge maps (later overrides)
        for (key, value) in other.env {
            self.env.insert(key, value);
        }

        // Extra fields: merge maps
        for (key, value) in other.extra {
            self.extra.insert(key, value);
        }
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
