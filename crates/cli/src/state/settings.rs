// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Global settings management.

use super::io::{parse_json5_or_json, to_io_error, JsonLoad};
use crate::mcp::config::McpServerDef;
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

impl JsonLoad for Settings {}

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

/// Hook matcher configuration (legacy array format)
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HookMatcher {
    /// Event type to match (e.g., "Stop", "PreToolUse", etc.)
    pub event: String,

    /// Optional pipe-separated pattern for sub-event matching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
}

/// Hook command definition
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HookCommand {
    /// Command type (e.g., "bash" or "command")
    #[serde(rename = "type")]
    pub command_type: String,

    /// Command to run
    pub command: String,

    /// Timeout in milliseconds (default: 60000)
    #[serde(default = "default_hook_timeout")]
    pub timeout: u64,
}

fn default_hook_timeout() -> u64 {
    60000
}

/// Hook definition (legacy array format)
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HookDef {
    /// Matcher for when to trigger the hook
    pub matcher: HookMatcher,

    /// Commands to execute when triggered
    pub hooks: Vec<HookCommand>,
}

/// Hook definition entry (real Claude Code map format).
///
/// In real Claude Code settings, hooks are keyed by event name:
/// ```json
/// {"hooks": {"PreToolUse": [{"matcher": "Bash|Read", "hooks": [{"type": "command", "command": "..."}]}]}}
/// ```
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct HookDefEntry {
    /// Optional pipe-separated tool name pattern for sub-event matching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,

    /// Commands to execute when triggered
    pub hooks: Vec<HookCommand>,
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
    pub mcp_servers: HashMap<String, McpServerDef>,

    /// Environment variable overrides
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Hook configurations keyed by event name (e.g., "PreToolUse", "Stop").
    ///
    /// Accepts both formats:
    /// - Map format (real Claude Code): `{"PreToolUse": [{"matcher": "Bash", "hooks": [...]}]}`
    /// - Legacy array format: `[{"matcher": {"event": "Stop"}, "hooks": [...]}]`
    #[serde(default, deserialize_with = "deserialize_hooks")]
    pub hooks: HashMap<String, Vec<HookDefEntry>>,

    /// Capture unknown fields for forward compatibility
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ClaudeSettings {
    /// Load settings from a JSON file.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse settings from a JSON/JSON5 string.
    pub fn parse(content: &str) -> std::io::Result<Self> {
        parse_json5_or_json(content).map_err(to_io_error)
    }

    /// Merge another settings file on top of this one.
    /// Later values override earlier ones. Hooks merge by event key.
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

        // Hooks: merge by event key, then by matcher within each event
        for (event, other_entries) in other.hooks {
            let entries = self.hooks.entry(event).or_default();
            for other_entry in other_entries {
                if let Some(existing) = entries
                    .iter_mut()
                    .find(|e| e.matcher == other_entry.matcher)
                {
                    *existing = other_entry;
                } else {
                    entries.push(other_entry);
                }
            }
        }

        // Extra fields: merge maps
        for (key, value) in other.extra {
            self.extra.insert(key, value);
        }
    }
}

/// Custom deserializer for hooks that accepts both map and legacy array formats.
fn deserialize_hooks<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, Vec<HookDefEntry>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Object(map) => {
            // New map format: {"PreToolUse": [{...}], "Stop": [{...}]}
            let mut result = HashMap::new();
            for (key, entries) in map {
                let entries: Vec<HookDefEntry> =
                    serde_json::from_value(entries).map_err(D::Error::custom)?;
                result.insert(key, entries);
            }
            Ok(result)
        }
        serde_json::Value::Array(arr) => {
            // Legacy array format: [{"matcher": {"event": "Stop"}, "hooks": [...]}]
            let mut result: HashMap<String, Vec<HookDefEntry>> = HashMap::new();
            for item in arr {
                let hook_def: HookDef = serde_json::from_value(item).map_err(D::Error::custom)?;
                let entry = HookDefEntry {
                    matcher: hook_def.matcher.matcher,
                    hooks: hook_def.hooks,
                };
                result
                    .entry(hook_def.matcher.event)
                    .or_default()
                    .push(entry);
            }
            Ok(result)
        }
        serde_json::Value::Null => Ok(HashMap::new()),
        _ => Err(D::Error::custom("hooks must be an object or array")),
    }
}

/// Load settings from a file path or inline JSON string.
///
/// Determines whether input is a file path or inline JSON based on content:
/// - Starts with `{` -> parse as inline JSON
/// - Otherwise -> treat as file path
pub fn load_settings_input(input: &str) -> std::io::Result<ClaudeSettings> {
    let trimmed = input.trim();
    if trimmed.starts_with('{') {
        ClaudeSettings::parse(trimmed)
    } else {
        ClaudeSettings::load(Path::new(input))
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
