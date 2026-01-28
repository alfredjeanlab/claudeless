#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use serde_json::json;

mod error_handling {
    use super::*;

    #[test]
    fn client_error_displays_correctly() {
        let err = ClientError::NotInitialized;
        assert!(err.to_string().contains("not initialized"));

        let err = ClientError::ToolNotFound("test".into());
        assert!(err.to_string().contains("test"));

        let err = ClientError::AlreadyInitialized;
        assert!(err.to_string().contains("already initialized"));

        let err = ClientError::UnsupportedVersion("1.0".into());
        assert!(err.to_string().contains("1.0"));
    }
}

mod integration {
    use super::*;
    use crate::mcp::config::McpServerDef;

    fn echo_server_def() -> McpServerDef {
        McpServerDef {
            command: "python3".into(),
            args: vec![
                "-c".into(),
                r#"
import json
import sys

def main():
    for line in sys.stdin:
        try:
            req = json.loads(line.strip())
            method = req.get("method", "")
            req_id = req.get("id")

            if method == "initialize":
                result = {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "echo", "version": "1.0.0"}
                }
            elif method == "notifications/initialized":
                continue  # No response for notifications
            elif method == "tools/list":
                result = {
                    "tools": [
                        {
                            "name": "echo",
                            "description": "Echo back input",
                            "inputSchema": {"type": "object"}
                        }
                    ]
                }
            elif method == "tools/call":
                params = req.get("params", {})
                result = {
                    "content": [{"type": "text", "text": json.dumps(params)}],
                    "isError": False
                }
            else:
                result = {}

            if req_id is not None:
                resp = {"jsonrpc": "2.0", "id": req_id, "result": result}
                print(json.dumps(resp), flush=True)
        except Exception as e:
            if req.get("id"):
                err = {
                    "jsonrpc": "2.0",
                    "id": req.get("id"),
                    "error": {"code": -32600, "message": str(e)}
                }
                print(json.dumps(err), flush=True)

if __name__ == "__main__":
    main()
"#
                .into(),
            ],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn connects_and_initializes() {
        let def = echo_server_def();
        let mut client = McpClient::connect(&def).await.unwrap();

        let server_info = client.initialize().await.unwrap();
        assert_eq!(server_info.name, "echo");
        assert!(client.is_initialized());

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn discovers_tools() {
        let def = echo_server_def();
        let client = McpClient::connect_and_initialize(&def).await.unwrap();

        assert!(client.has_tool("echo"));
        assert!(!client.has_tool("nonexistent"));

        let tool = client.get_tool("echo").unwrap();
        assert_eq!(tool.name, "echo");

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn calls_tool() {
        let def = echo_server_def();
        let client = McpClient::connect_and_initialize(&def).await.unwrap();

        let result = client
            .call_tool("echo", json!({"msg": "hello"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(!result.content.is_empty());

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn calls_tool_with_timeout() {
        let def = echo_server_def();
        let client = McpClient::connect_and_initialize(&def).await.unwrap();

        let result = client
            .call_tool_with_timeout("echo", json!({"msg": "hello"}), 5000)
            .await
            .unwrap();
        assert!(!result.is_error);

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn connect_and_initialize_convenience() {
        let def = echo_server_def();
        let client = McpClient::connect_and_initialize(&def).await.unwrap();

        assert!(client.is_initialized());
        assert!(!client.tools().is_empty());
        assert!(client.server_info().is_some());

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fails_without_initialization() {
        let def = echo_server_def();
        let client = McpClient::connect(&def).await.unwrap();

        let result = client.call_tool("echo", json!({})).await;
        assert!(matches!(result, Err(ClientError::NotInitialized)));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fails_double_initialization() {
        let def = echo_server_def();
        let mut client = McpClient::connect(&def).await.unwrap();
        client.initialize().await.unwrap();

        let result = client.initialize().await;
        assert!(matches!(result, Err(ClientError::AlreadyInitialized)));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn list_tools_requires_initialization() {
        let def = echo_server_def();
        let mut client = McpClient::connect(&def).await.unwrap();

        let result = client.list_tools().await;
        assert!(matches!(result, Err(ClientError::NotInitialized)));

        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn definition_accessor_works() {
        let def = echo_server_def();
        let client = McpClient::connect(&def).await.unwrap();

        assert_eq!(client.definition().command, "python3");

        client.shutdown().await.unwrap();
    }
}

mod error_cases {
    use super::*;
    use crate::mcp::config::McpServerDef;

    #[tokio::test]
    async fn connect_fails_for_missing_command() {
        let def = McpServerDef {
            command: "nonexistent_command_12345".into(),
            ..Default::default()
        };

        let result = McpClient::connect(&def).await;
        assert!(matches!(result, Err(ClientError::Transport(_))));
    }

    #[tokio::test]
    async fn handles_server_crash() {
        let def = McpServerDef {
            command: "sh".into(),
            args: vec!["-c".into(), "exit 1".into()],
            ..Default::default()
        };

        let mut client = McpClient::connect(&def).await.unwrap();

        // Server exited, so initialize should fail
        let result = client.initialize().await;
        assert!(result.is_err());
    }
}
