// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

// NOTE(compat): Keep full API surface for future use
#![allow(dead_code)]

//! MCP tool executor for routing tool calls to MCP servers.

use std::path::Path;
use std::sync::Arc;

use parking_lot::RwLock;

use super::builtin::BuiltinExecutor;
use super::executor::{ExecutionContext, ToolExecutor};
use super::result::ToolExecutionResult;
use crate::config::ToolCallSpec;
use crate::mcp::config::McpToolDef;
use crate::mcp::McpManager;

/// Executor that handles MCP tool calls.
pub struct McpToolExecutor {
    /// Shared MCP manager with server connections.
    manager: Arc<RwLock<McpManager>>,
}

impl McpToolExecutor {
    /// Create a new MCP tool executor.
    pub fn new(manager: Arc<RwLock<McpManager>>) -> Self {
        Self { manager }
    }

    /// Check if a tool is handled by MCP.
    ///
    /// Handles both raw tool names (`read_file`) and qualified names
    /// (`mcp__filesystem__read_file`).
    pub fn has_tool(&self, name: &str) -> bool {
        let raw_name = Self::get_raw_tool_name(name);
        self.manager.read().has_tool(&raw_name)
    }

    /// Get the raw tool name from a potentially qualified name.
    ///
    /// Qualified names have the format `mcp__<server>__<tool>`.
    /// Raw names are returned as-is.
    fn get_raw_tool_name(name: &str) -> String {
        if let Some((_server, tool)) = McpToolDef::parse_qualified_name(name) {
            tool
        } else {
            name.to_string()
        }
    }
}

impl ToolExecutor for McpToolExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        _ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        // Extract raw tool name from potentially qualified name (mcp__server__tool)
        let raw_tool_name = Self::get_raw_tool_name(&call.tool);

        // Check if we handle this tool
        let manager = self.manager.read();
        if !manager.has_tool(&raw_tool_name) {
            return ToolExecutionResult::error(
                tool_use_id,
                format!("MCP tool not found: {}", call.tool),
            );
        }

        // Bridge async to sync safely within an async runtime
        // Use block_in_place to avoid "Cannot start a runtime from within a runtime" panic
        let handle = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => {
                return ToolExecutionResult::error(
                    tool_use_id,
                    "No tokio runtime available for MCP execution",
                );
            }
        };

        // Execute the async call using block_in_place to safely block
        // Canonicalize path arguments to handle symlinks (e.g., /tmp -> /private/tmp on macOS)
        let input = canonicalize_path_arguments(call.input.clone());
        let result = tokio::task::block_in_place(|| {
            handle.block_on(async { manager.call_tool(&raw_tool_name, input).await })
        });

        // Convert McpToolResult to ToolExecutionResult
        match result {
            Ok(mcp_result) => {
                if mcp_result.success {
                    // Format content as string for tool result
                    let text = format_mcp_content(&mcp_result.content);
                    ToolExecutionResult::success(tool_use_id, text)
                } else {
                    ToolExecutionResult::error(
                        tool_use_id,
                        mcp_result
                            .error
                            .unwrap_or_else(|| "MCP tool execution failed".into()),
                    )
                }
            }
            Err(e) => ToolExecutionResult::error(tool_use_id, e.to_string()),
        }
    }

    fn name(&self) -> &'static str {
        "mcp"
    }
}

/// Format MCP content Value as string for tool result.
fn format_mcp_content(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
    }
}

/// Known path argument names used by filesystem tools.
const PATH_ARGUMENT_NAMES: &[&str] = &[
    "path",
    "file_path",
    "directory",
    "source",
    "destination",
    "old_path",
    "new_path",
];

/// Canonicalize path arguments in tool input to resolve symlinks.
///
/// This handles cases like `/tmp` -> `/private/tmp` on macOS, ensuring
/// paths match what MCP servers expect after resolving their allowed directories.
fn canonicalize_path_arguments(mut input: serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::Object(ref mut map) = input {
        for key in PATH_ARGUMENT_NAMES {
            if let Some(serde_json::Value::String(path_str)) = map.get(*key) {
                let path = Path::new(path_str);
                // Only canonicalize if the path exists (canonicalize fails for non-existent paths)
                // For non-existent paths (e.g., write targets), try to canonicalize the parent
                if let Ok(canonical) = std::fs::canonicalize(path) {
                    map.insert(
                        (*key).to_string(),
                        serde_json::Value::String(canonical.to_string_lossy().into_owned()),
                    );
                } else if let Some(parent) = path.parent() {
                    // Path doesn't exist yet - canonicalize parent and append filename
                    if let Ok(canonical_parent) = std::fs::canonicalize(parent) {
                        if let Some(filename) = path.file_name() {
                            let canonical = canonical_parent.join(filename);
                            map.insert(
                                (*key).to_string(),
                                serde_json::Value::String(canonical.to_string_lossy().into_owned()),
                            );
                        }
                    }
                }
            }
        }
    }
    input
}

/// Executor that routes to MCP first, then falls back to builtin.
pub struct CompositeExecutor {
    /// Optional MCP executor (None if no MCP servers configured).
    mcp: Option<McpToolExecutor>,
    /// Builtin tool executor as fallback.
    builtin: BuiltinExecutor,
}

impl CompositeExecutor {
    /// Create a new composite executor.
    pub fn new(mcp: Option<McpToolExecutor>, builtin: BuiltinExecutor) -> Self {
        Self { mcp, builtin }
    }

    /// Create with just builtin (no MCP).
    pub fn builtin_only(builtin: BuiltinExecutor) -> Self {
        Self { mcp: None, builtin }
    }
}

impl ToolExecutor for CompositeExecutor {
    fn execute(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &ExecutionContext,
    ) -> ToolExecutionResult {
        // Check if this is an MCP tool (qualified name or known raw name)
        if let Some(ref mcp) = self.mcp {
            // Check for qualified name (mcp__server__tool) or raw MCP tool name
            let is_mcp_qualified = call.tool.starts_with("mcp__");
            let is_mcp_tool = mcp.has_tool(&call.tool);

            if is_mcp_qualified || is_mcp_tool {
                return mcp.execute(call, tool_use_id, ctx);
            }
        }

        // Fall back to builtin
        self.builtin.execute(call, tool_use_id, ctx)
    }

    fn name(&self) -> &'static str {
        "composite"
    }
}

#[cfg(test)]
#[path = "mcp_executor_tests.rs"]
mod tests;
