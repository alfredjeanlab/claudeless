#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_transport_spawn_nonexistent() {
    let def = McpServerDef {
        command: "nonexistent_command_12345".to_string(),
        args: vec![],
        env: Default::default(),
        cwd: None,
        timeout_ms: 30000,
    };

    let result = McpTransport::spawn(&def);
    assert!(result.is_err());
    if let Err(McpTransportError::SpawnFailed { command, .. }) = result {
        assert_eq!(command, "nonexistent_command_12345");
    } else {
        panic!("Expected SpawnFailed error");
    }
}

#[test]
fn test_transport_error_display() {
    let err = McpTransportError::ConnectionClosed;
    assert_eq!(err.to_string(), "Connection closed unexpectedly");

    let err = McpTransportError::IdMismatch {
        expected: 1,
        got: 2,
    };
    assert!(err.to_string().contains("expected 1"));
}
