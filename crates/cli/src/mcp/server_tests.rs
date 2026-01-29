// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
    // Manually mark the server as running for the test
    manager.get_server_mut("fs").unwrap().start();

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
    // Servers are not auto-started now
    assert_eq!(manager.running_server_count(), 0);
    assert!(manager.has_servers());
}

#[test]
fn test_server_for_tool() {
    let config = McpConfig::parse(r#"{"mcpServers": {"fs": {"command": "node"}}}"#).unwrap();
    let mut manager = McpManager::from_config(&config);
    // Manually mark the server as running for the test
    manager.get_server_mut("fs").unwrap().start();

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

#[test]
fn test_call_tool_without_connection() {
    let def = McpServerDef::default();
    let server = McpServer::from_def("test", def);

    // call_tool should fail when not connected
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(server.call_tool("echo", serde_json::json!({})));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ClientError::NotInitialized));
}

// =============================================================================
// Integration Tests (require echo MCP server)
// =============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::path::PathBuf;

    fn echo_server_def() -> McpServerDef {
        // Use the test echo server script
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests/fixtures/echo_mcp_server.py");
        McpServerDef {
            command: "python3".into(),
            args: vec![script.to_string_lossy().to_string()],
            timeout_ms: 5000,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_server_spawn_and_tool_discovery() {
        let mut server = McpServer::from_def("echo", echo_server_def());
        assert_eq!(server.status, McpServerStatus::Uninitialized);

        server.spawn().await.expect("spawn failed");

        assert_eq!(server.status, McpServerStatus::Running);
        assert!(server.is_connected());
        assert!(!server.tools.is_empty());
        assert!(server.tools.iter().any(|t| t.name == "echo"));

        server.shutdown().await.expect("shutdown failed");
        assert_eq!(server.status, McpServerStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_server_call_tool() {
        let mut server = McpServer::from_def("echo", echo_server_def());
        server.spawn().await.expect("spawn failed");

        let result = server
            .call_tool("echo", serde_json::json!({"message": "hello"}))
            .await
            .expect("call_tool failed");

        assert!(result.success);
        server.shutdown().await.ok();
    }

    #[tokio::test]
    async fn test_server_call_tool_error() {
        let mut server = McpServer::from_def("echo", echo_server_def());
        server.spawn().await.expect("spawn failed");

        // The "fail" tool always returns an error
        let result = server
            .call_tool("fail", serde_json::json!({}))
            .await
            .expect("call_tool failed");

        assert!(!result.success);
        assert!(result.error.is_some());
        server.shutdown().await.ok();
    }

    #[tokio::test]
    async fn test_manager_initialize_and_call() {
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests/fixtures/echo_mcp_server.py");
        let config = McpConfig::parse(&format!(
            r#"{{"mcpServers": {{"echo": {{"command": "python3", "args": ["{}"]}}}}}}"#,
            script.to_string_lossy()
        ))
        .unwrap();

        let mut manager = McpManager::from_config(&config);
        assert_eq!(manager.running_server_count(), 0);

        let results = manager.initialize().await;
        assert!(results.iter().all(|(_, r)| r.is_ok()));
        assert_eq!(manager.running_server_count(), 1);
        assert!(manager.has_tool("echo"));

        let result = manager
            .call_tool("echo", serde_json::json!({"msg": "test"}))
            .await
            .expect("call failed");
        assert!(result.success);

        manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_manager_call_unknown_tool() {
        let manager = McpManager::new();

        let result = manager
            .call_tool("nonexistent", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClientError::ToolNotFound(_)));
    }

    #[tokio::test]
    async fn test_spawn_with_empty_command() {
        let def = McpServerDef::default();
        let mut server = McpServer::from_def("test", def);

        let result = server.spawn().await;
        assert!(result.is_err());
        // Should fail with transport error for empty command
        assert!(matches!(result.unwrap_err(), ClientError::Transport(_)));
    }
}
