// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool result types for MCP servers.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
