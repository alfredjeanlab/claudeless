// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::mcp::config::McpServerDef;
use crate::mcp::transport::TransportError;
use serde_json::json;

/// Helper to get the path to the echo server fixture
fn echo_server_path() -> String {
    // Resolve relative to workspace root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/../../tests/fixtures/echo_mcp_server.py", manifest_dir)
}

fn echo_server_def() -> McpServerDef {
    McpServerDef {
        command: "python3".into(),
        args: vec![echo_server_path()],
        timeout_ms: 5000,
        ..Default::default()
    }
}

mod error_types {
    use super::*;

    #[test]
    fn client_errors_display_correctly() {
        let errors = vec![
            ClientError::NotInitialized,
            ClientError::AlreadyInitialized,
            ClientError::ToolNotFound("missing".into()),
            ClientError::InvalidResponse("bad json".into()),
            ClientError::UnsupportedVersion("1.0".into()),
        ];

        for err in errors {
            let msg = format!("{}", err);
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn transport_error_converts_to_client_error() {
        let transport_err = TransportError::Shutdown;
        let client_err: ClientError = transport_err.into();
        assert!(matches!(client_err, ClientError::Transport(_)));
    }
}

mod connect {
    use super::*;

    #[tokio::test]
    async fn connects_to_echo_server() {
        let client = McpClient::connect(&echo_server_def()).await.unwrap();
        assert!(!client.is_initialized());
        assert!(client.is_running().await);
        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn connect_fails_for_missing_command() {
        let def = McpServerDef {
            command: "nonexistent_command_xyz".into(),
            ..Default::default()
        };
        let result = McpClient::connect(&def).await;
        assert!(matches!(result, Err(ClientError::Transport(_))));
    }
}

mod initialize {
    use super::*;

    #[tokio::test]
    async fn initializes_successfully() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();

        let server_info = client.initialize().await.unwrap();
        assert_eq!(server_info.name, "echo-test");
        assert!(client.is_initialized());

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fails_double_initialization() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();

        let result = client.initialize().await;
        assert!(matches!(result, Err(ClientError::AlreadyInitialized)));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn server_info_available_after_init() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        assert!(client.server_info().is_none());

        client.initialize().await.unwrap();

        let info = client.server_info().unwrap();
        assert_eq!(info.name, "echo-test");
        assert_eq!(info.version.as_deref(), Some("1.0.0"));

        client.shutdown().await.unwrap();
    }
}

mod list_tools {
    use super::*;

    #[tokio::test]
    async fn lists_tools_after_init() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();

        let tools = client.list_tools().await.unwrap();
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "echo"));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fails_before_initialization() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();

        let result = client.list_tools().await;
        assert!(matches!(result, Err(ClientError::NotInitialized)));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn has_tool_checks_cached_list() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();
        client.list_tools().await.unwrap();

        assert!(client.has_tool("echo"));
        assert!(client.has_tool("fail"));
        assert!(!client.has_tool("nonexistent"));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn tools_accessor_returns_cached() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        assert!(client.tools().is_empty());

        client.initialize().await.unwrap();
        client.list_tools().await.unwrap();

        assert!(!client.tools().is_empty());

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn get_tool_returns_tool_info() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();
        client.list_tools().await.unwrap();

        let tool = client.get_tool("echo").unwrap();
        assert_eq!(tool.name, "echo");
        assert!(tool.description.is_some());

        assert!(client.get_tool("nonexistent").is_none());

        client.shutdown().await.unwrap();
    }
}

mod call_tool {
    use super::*;

    #[tokio::test]
    async fn calls_echo_tool() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();

        let result = client
            .call_tool("echo", json!({"message": "hello"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(!result.content.is_empty());

        // Verify echo returned our input
        let text = result
            .content
            .iter()
            .filter_map(|c| c.as_text())
            .next()
            .unwrap();
        assert!(text.contains("hello"));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn handles_tool_error() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();

        let result = client.call_tool("fail", json!({})).await.unwrap();
        assert!(result.is_error);

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fails_before_initialization() {
        let client = McpClient::connect(&echo_server_def()).await.unwrap();

        let result = client.call_tool("echo", json!({})).await;
        assert!(matches!(result, Err(ClientError::NotInitialized)));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn call_with_empty_arguments() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();

        let result = client.call_tool("echo", json!({})).await.unwrap();
        assert!(!result.is_error);

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn call_with_custom_timeout() {
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();
        client.initialize().await.unwrap();

        let result = client
            .call_tool_with_timeout("echo", json!({"message": "hello"}), 10000)
            .await
            .unwrap();

        assert!(!result.is_error);

        client.shutdown().await.unwrap();
    }
}

mod connect_and_initialize {
    use super::*;

    #[tokio::test]
    async fn convenience_method_works() {
        let client = McpClient::connect_and_initialize(&echo_server_def())
            .await
            .unwrap();

        assert!(client.is_initialized());
        assert!(!client.tools().is_empty());
        assert!(client.has_tool("echo"));

        client.shutdown().await.unwrap();
    }
}

mod shutdown {
    use super::*;

    #[tokio::test]
    async fn shutdown_terminates_process() {
        let client = McpClient::connect(&echo_server_def()).await.unwrap();
        assert!(client.is_running().await);

        client.shutdown().await.unwrap();
        // Client is consumed, can't check is_running
    }
}

mod protocol_flow {
    use super::*;

    #[tokio::test]
    async fn full_lifecycle() {
        // Complete protocol flow test
        let mut client = McpClient::connect(&echo_server_def()).await.unwrap();

        // Step 1: Initialize
        let server_info = client.initialize().await.unwrap();
        assert_eq!(server_info.name, "echo-test");

        // Step 2: Discover tools
        let tools = client.list_tools().await.unwrap();
        assert!(tools.len() >= 2);

        // Step 3: Call tool
        let result = client
            .call_tool("echo", json!({"test": true}))
            .await
            .unwrap();
        assert!(!result.is_error);

        // Step 4: Call another tool
        let result = client.call_tool("fail", json!({})).await.unwrap();
        assert!(result.is_error);

        // Step 5: Shutdown
        client.shutdown().await.unwrap();
    }
}

mod error_recovery {
    use super::*;

    #[tokio::test]
    async fn handles_server_crash() {
        let def = McpServerDef {
            command: "sh".into(),
            args: vec!["-c".into(), "exit 1".into()],
            timeout_ms: 1000,
            ..Default::default()
        };

        let mut client = McpClient::connect(&def).await.unwrap();

        // Give process time to exit
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Initialize should fail since process exited
        let result = client.initialize().await;
        assert!(result.is_err());
    }
}

mod accessors {
    use super::*;

    #[tokio::test]
    async fn definition_returns_server_def() {
        let def = echo_server_def();
        let client = McpClient::connect(&def).await.unwrap();

        assert_eq!(client.definition().command, "python3");
        assert!(!client.definition().args.is_empty());

        client.shutdown().await.unwrap();
    }
}
