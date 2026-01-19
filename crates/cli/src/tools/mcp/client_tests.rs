#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_client_error_display() {
    let err = McpClientError::NotInitialized;
    assert_eq!(err.to_string(), "Client not initialized");

    let err = McpClientError::ToolCallFailed {
        tool: "read_file".to_string(),
        message: "file not found".to_string(),
    };
    assert!(err.to_string().contains("read_file"));
}

#[test]
fn test_spawn_nonexistent_server() {
    let def = McpServerDef {
        command: "nonexistent_mcp_server_12345".to_string(),
        args: vec![],
        env: Default::default(),
        cwd: None,
        timeout_ms: 30000,
    };

    let result = McpClient::spawn(&def);
    assert!(result.is_err());
}
