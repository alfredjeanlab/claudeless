// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::config::ToolCallSpec;
use crate::mcp::config::{McpServerDef, McpToolDef};
use crate::mcp::server::{McpManager, McpServer, McpServerStatus};
use parking_lot::RwLock;
use std::sync::Arc;

fn mock_manager_with_tool(tool_name: &str) -> Arc<RwLock<McpManager>> {
    let mut manager = McpManager::new();

    let mut server = McpServer::from_def("test-server", McpServerDef::default());
    server.status = McpServerStatus::Running;
    server.register_tool(McpToolDef {
        name: tool_name.to_string(),
        description: "Test tool".to_string(),
        input_schema: serde_json::json!({"type": "object"}),
        server_name: "test-server".to_string(),
    });

    manager.add_server(server);
    manager.register_tool(
        "test-server",
        McpToolDef {
            name: tool_name.to_string(),
            description: "Test tool".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            server_name: "test-server".to_string(),
        },
    );

    Arc::new(RwLock::new(manager))
}

#[test]
fn has_tool_returns_true_for_registered_tool() {
    let manager = mock_manager_with_tool("my_tool");
    let executor = McpToolExecutor::new(manager);

    assert!(executor.has_tool("my_tool"));
    assert!(!executor.has_tool("other_tool"));
}

#[test]
fn mcp_executor_name() {
    let manager = mock_manager_with_tool("test");
    let executor = McpToolExecutor::new(manager);
    assert_eq!(executor.name(), "mcp");
}

#[test]
fn composite_executor_name() {
    let builtin = BuiltinExecutor::new();
    let composite = CompositeExecutor::builtin_only(builtin);
    assert_eq!(composite.name(), "composite");
}

#[test]
fn composite_routes_to_builtin_for_unknown_mcp_tool() {
    let manager = mock_manager_with_tool("mcp_tool");
    let mcp = McpToolExecutor::new(manager);
    let builtin = BuiltinExecutor::new();
    let composite = CompositeExecutor::new(Some(mcp), builtin);

    let call = ToolCallSpec {
        tool: "Read".to_string(),
        input: serde_json::json!({"file_path": "/tmp/test"}),
        result: Some("test content".to_string()),
    };

    let result = composite.execute(&call, "test-id", &ExecutionContext::default());

    // Should fall through to builtin (which uses mock result)
    assert!(!result.is_error);
    assert_eq!(result.text(), Some("test content"));
}

#[test]
fn composite_routes_mcp_tool_to_mcp() {
    let manager = mock_manager_with_tool("custom_tool");
    let mcp = McpToolExecutor::new(manager);
    let builtin = BuiltinExecutor::new();
    let composite = CompositeExecutor::new(Some(mcp), builtin);

    // MCP tool should be recognized
    assert!(composite.mcp.as_ref().unwrap().has_tool("custom_tool"));
}

#[test]
fn composite_without_mcp_works() {
    let builtin = BuiltinExecutor::new();
    let composite = CompositeExecutor::builtin_only(builtin);

    let call = ToolCallSpec {
        tool: "Read".to_string(),
        input: serde_json::json!({"file_path": "/tmp/test"}),
        result: Some("test content".to_string()),
    };

    let result = composite.execute(&call, "test-id", &ExecutionContext::default());
    assert!(!result.is_error);
    assert_eq!(result.text(), Some("test content"));
}

#[test]
fn format_mcp_content_string() {
    let content = serde_json::Value::String("hello world".to_string());
    assert_eq!(format_mcp_content(&content), "hello world");
}

#[test]
fn format_mcp_content_null() {
    let content = serde_json::Value::Null;
    assert_eq!(format_mcp_content(&content), "");
}

#[test]
fn format_mcp_content_object() {
    let content = serde_json::json!({"key": "value"});
    let result = format_mcp_content(&content);
    assert!(result.contains("key"));
    assert!(result.contains("value"));
}

#[test]
fn format_mcp_content_array() {
    let content = serde_json::json!([1, 2, 3]);
    let result = format_mcp_content(&content);
    assert!(result.contains("1"));
    assert!(result.contains("2"));
    assert!(result.contains("3"));
}
