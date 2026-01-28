// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn test_server_lifecycle() {
    let def = McpServerDef {
        command: "node".into(),
        args: vec!["server.js".into()],
        env: HashMap::new(),
        cwd: None,
        timeout_ms: 30000,
    };

    let mut server = McpServer::from_def("test", def);
    assert_eq!(server.status, McpServerStatus::Uninitialized);

    server.start();
    assert!(server.is_running());

    server.disconnect();
    assert_eq!(server.status, McpServerStatus::Disconnected);
    assert!(!server.is_running());
}

#[test]
fn test_server_failure() {
    let def = McpServerDef::default();
    let mut server = McpServer::from_def("test", def);

    server.fail("connection refused");
    assert!(matches!(server.status, McpServerStatus::Failed(_)));
    if let McpServerStatus::Failed(reason) = &server.status {
        assert!(reason.contains("connection refused"));
    }
}

#[test]
fn test_tool_registration() {
    let config = McpConfig::parse(r#"{"mcpServers": {"fs": {"command": "node"}}}"#).unwrap();
    let mut manager = McpManager::from_config(&config);

    manager.register_tool(
        "fs",
        McpToolDef {
            name: "read_file".into(),
            description: "Read a file".into(),
            input_schema: serde_json::json!({"type": "object"}),
            server_name: "fs".into(),
        },
    );

    assert!(manager.has_tool("read_file"));
    assert_eq!(manager.tool_names(), vec!["read_file"]);
}

#[test]
fn test_manager_from_config() {
    let config =
        McpConfig::parse(r#"{"mcpServers": {"a": {"command": "a"}, "b": {"command": "b"}}}"#)
            .unwrap();
    let manager = McpManager::from_config(&config);

    assert_eq!(manager.server_count(), 2);
    assert_eq!(manager.running_server_count(), 2);
    assert!(manager.has_servers());
}

#[test]
fn test_server_for_tool() {
    let config = McpConfig::parse(r#"{"mcpServers": {"fs": {"command": "node"}}}"#).unwrap();
    let mut manager = McpManager::from_config(&config);

    manager.register_tool(
        "fs",
        McpToolDef {
            name: "read_file".into(),
            description: "Read".into(),
            input_schema: serde_json::json!({}),
            server_name: "fs".into(),
        },
    );

    let server = manager.server_for_tool("read_file").unwrap();
    assert_eq!(server.name, "fs");

    assert!(manager.server_for_tool("nonexistent").is_none());
}

#[test]
fn test_empty_manager() {
    let manager = McpManager::new();
    assert!(!manager.has_servers());
    assert_eq!(manager.server_count(), 0);
    assert!(manager.tools().is_empty());
    assert!(manager.tool_names().is_empty());
}
