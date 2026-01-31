// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

mod json_rpc_types {
    use super::*;

    #[test]
    fn request_serializes_correctly() {
        let req = JsonRpcRequest::new(1, "test", Some(serde_json::json!({"key": "value"})));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""id":1"#));
        assert!(json.contains(r#""method":"test""#));
    }

    #[test]
    fn request_without_params_omits_field() {
        let req = JsonRpcRequest::new(1, "test", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("params"));
    }

    #[test]
    fn response_deserializes_success() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, 1);
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn response_deserializes_error() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32600);
        assert_eq!(err.message, "Invalid Request");
    }

    #[test]
    fn response_deserializes_error_with_data() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request","data":{"extra":"info"}}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32600);
        assert!(err.data.is_some());
    }

    #[test]
    fn response_into_result_success() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let result = resp.into_result().unwrap();
        assert_eq!(result["ok"], true);
    }

    #[test]
    fn response_into_result_error() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let err = resp.into_result().unwrap_err();
        assert_eq!(err.code, -32600);
    }

    #[test]
    fn response_into_result_null() {
        let json = r#"{"jsonrpc":"2.0","id":1}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let result = resp.into_result().unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn notification_serializes_correctly() {
        let notif = JsonRpcNotification::new("notify", None);
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""method":"notify""#));
        assert!(!json.contains("id"));
    }

    #[test]
    fn notification_with_params() {
        let notif = JsonRpcNotification::new("notify", Some(serde_json::json!({"data": 123})));
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains(r#""params""#));
        assert!(json.contains(r#""data":123"#));
    }
}

mod spawn {
    use super::*;

    #[tokio::test]
    async fn spawns_echo_process() {
        let def = McpServerDef {
            command: "cat".to_string(),
            ..Default::default()
        };
        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();
        assert!(transport.is_running().await);
        transport.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn spawn_fails_for_nonexistent_command() {
        let def = McpServerDef {
            command: "nonexistent_command_12345".to_string(),
            ..Default::default()
        };
        let result = StdioTransport::spawn(&def, "test", false).await;
        assert!(matches!(result, Err(TransportError::Spawn(_))));
    }

    #[tokio::test]
    async fn spawn_with_args() {
        let def = McpServerDef {
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            ..Default::default()
        };
        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();
        // Process will exit immediately after echo, that's OK
        transport.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn spawn_with_env_vars() {
        let mut def = McpServerDef {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "echo $TEST_VAR".to_string()],
            ..Default::default()
        };
        def.env.insert("TEST_VAR".to_string(), "hello".to_string());

        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();
        transport.shutdown().await.unwrap();
    }
}

mod send_receive {
    use super::*;

    #[tokio::test]
    async fn send_and_receive_roundtrip() {
        // Use a simple JSON echo server (Python one-liner)
        let def = McpServerDef {
            command: "python3".to_string(),
            args: vec![
                "-c".to_string(),
                r#"
import sys, json
for line in sys.stdin:
    req = json.loads(line)
    resp = {"jsonrpc": "2.0", "id": req["id"], "result": req.get("params", {})}
    print(json.dumps(resp), flush=True)
"#
                .to_string(),
            ],
            ..Default::default()
        };

        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();

        let result = transport
            .request("echo", Some(serde_json::json!({"test": "value"})), 5000)
            .await
            .unwrap();

        assert_eq!(result["test"], "value");

        transport.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn multiple_requests() {
        let def = McpServerDef {
            command: "python3".to_string(),
            args: vec![
                "-c".to_string(),
                r#"
import sys, json
for line in sys.stdin:
    req = json.loads(line)
    resp = {"jsonrpc": "2.0", "id": req["id"], "result": {"method": req["method"]}}
    print(json.dumps(resp), flush=True)
"#
                .to_string(),
            ],
            ..Default::default()
        };

        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();

        let result1 = transport.request("method1", None, 5000).await.unwrap();
        assert_eq!(result1["method"], "method1");

        let result2 = transport.request("method2", None, 5000).await.unwrap();
        assert_eq!(result2["method"], "method2");

        let result3 = transport.request("method3", None, 5000).await.unwrap();
        assert_eq!(result3["method"], "method3");

        transport.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn receive_returns_process_exited_on_eof() {
        let def = McpServerDef {
            command: "true".to_string(), // Exits immediately
            ..Default::default()
        };

        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();

        // Give process time to exit
        tokio::time::sleep(Duration::from_millis(100)).await;

        let result = transport.receive().await;
        assert!(matches!(result, Err(TransportError::ProcessExited)));
    }

    #[tokio::test]
    async fn request_handles_json_rpc_error() {
        let def = McpServerDef {
            command: "python3".to_string(),
            args: vec![
                "-c".to_string(),
                r#"
import sys, json
for line in sys.stdin:
    req = json.loads(line)
    resp = {"jsonrpc": "2.0", "id": req["id"], "error": {"code": -32601, "message": "Method not found"}}
    print(json.dumps(resp), flush=True)
"#
                .to_string(),
            ],
            ..Default::default()
        };

        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();

        let result = transport.request("unknown", None, 5000).await;
        match result {
            Err(TransportError::JsonRpc(err)) => {
                assert_eq!(err.code, -32601);
                assert_eq!(err.message, "Method not found");
            }
            other => panic!("Expected JsonRpc error, got {:?}", other),
        }

        transport.shutdown().await.unwrap();
    }
}

mod timeout_tests {
    use super::*;

    #[tokio::test]
    async fn request_times_out() {
        // Process that doesn't respond
        let def = McpServerDef {
            command: "sleep".to_string(),
            args: vec!["10".to_string()],
            ..Default::default()
        };

        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();

        let result = transport.request("test", None, 100).await;
        assert!(matches!(result, Err(TransportError::Timeout(100))));

        transport.shutdown().await.unwrap();
    }
}

mod shutdown_tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_marks_transport_closed() {
        let def = McpServerDef {
            command: "cat".to_string(),
            ..Default::default()
        };
        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();

        transport.shutdown().await.unwrap();

        assert!(transport.is_shutdown());
        assert!(!transport.is_running().await);
    }

    #[tokio::test]
    async fn operations_fail_after_shutdown() {
        let def = McpServerDef {
            command: "cat".to_string(),
            ..Default::default()
        };
        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();
        transport.shutdown().await.unwrap();

        let result = transport.request("test", None, 1000).await;
        assert!(matches!(result, Err(TransportError::Shutdown)));
    }

    #[tokio::test]
    async fn send_fails_after_shutdown() {
        let def = McpServerDef {
            command: "cat".to_string(),
            ..Default::default()
        };
        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();
        transport.shutdown().await.unwrap();

        let req = JsonRpcRequest::new(1, "test", None);
        let result = transport.send(&req).await;
        assert!(matches!(result, Err(TransportError::Shutdown)));
    }

    #[tokio::test]
    async fn receive_fails_after_shutdown() {
        let def = McpServerDef {
            command: "cat".to_string(),
            ..Default::default()
        };
        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();
        transport.shutdown().await.unwrap();

        let result = transport.receive().await;
        assert!(matches!(result, Err(TransportError::Shutdown)));
    }

    #[tokio::test]
    async fn shutdown_force_kills_unresponsive_process() {
        // Process that ignores stdin close
        let def = McpServerDef {
            command: "sleep".to_string(),
            args: vec!["60".to_string()],
            ..Default::default()
        };

        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();

        // Shutdown should still succeed (will force kill after timeout)
        let start = std::time::Instant::now();
        transport.shutdown().await.unwrap();
        let elapsed = start.elapsed();

        // Should take ~1 second (the graceful timeout) plus a bit
        assert!(elapsed < Duration::from_secs(3));
        assert!(transport.is_shutdown());
    }

    #[tokio::test]
    async fn notification_fails_after_shutdown() {
        let def = McpServerDef {
            command: "cat".to_string(),
            ..Default::default()
        };
        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();
        transport.shutdown().await.unwrap();

        let notif = JsonRpcNotification::new("test", None);
        let result = transport.send_notification(&notif).await;
        assert!(matches!(result, Err(TransportError::Shutdown)));
    }
}

mod id_generation {
    use super::*;

    #[tokio::test]
    async fn ids_increment() {
        let def = McpServerDef {
            command: "python3".to_string(),
            args: vec![
                "-c".to_string(),
                r#"
import sys, json
for line in sys.stdin:
    req = json.loads(line)
    resp = {"jsonrpc": "2.0", "id": req["id"], "result": {"received_id": req["id"]}}
    print(json.dumps(resp), flush=True)
"#
                .to_string(),
            ],
            ..Default::default()
        };

        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();

        let result1 = transport.request("test", None, 5000).await.unwrap();
        let result2 = transport.request("test", None, 5000).await.unwrap();
        let result3 = transport.request("test", None, 5000).await.unwrap();

        assert_eq!(result1["received_id"], 1);
        assert_eq!(result2["received_id"], 2);
        assert_eq!(result3["received_id"], 3);

        transport.shutdown().await.unwrap();
    }
}

mod error_display {
    use super::*;

    #[test]
    fn transport_error_display() {
        let err = TransportError::Spawn("command not found".to_string());
        assert_eq!(
            format!("{}", err),
            "failed to spawn process: command not found"
        );

        let err = TransportError::Timeout(5000);
        assert_eq!(format!("{}", err), "request timed out after 5000ms");

        let err = TransportError::IdMismatch {
            request: 1,
            response: 2,
        };
        assert_eq!(
            format!("{}", err),
            "response id 2 doesn't match request id 1"
        );
    }

    #[test]
    fn json_rpc_error_display() {
        let err = JsonRpcError {
            code: -32600,
            message: "Invalid Request".to_string(),
            data: None,
        };
        assert_eq!(format!("{}", err), "JSON-RPC error -32600: Invalid Request");
    }
}

mod json_rpc_edge_cases {
    use super::*;

    #[test]
    fn response_with_null_result() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":null}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let result = resp.into_result().unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn request_with_complex_params() {
        let params = serde_json::json!({
            "nested": {"array": [1, 2, 3], "bool": true},
            "unicode": "こんにちは"
        });
        let req = JsonRpcRequest::new(99, "complex", Some(params.clone()));
        let json = serde_json::to_string(&req).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["params"]["nested"]["array"][0], 1);
        assert_eq!(parsed["params"]["unicode"], "こんにちは");
    }

    #[test]
    fn error_with_numeric_data() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"Custom","data":42}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.data.unwrap(), 42);
    }
}

mod transport_error_coverage {
    use super::*;

    #[test]
    fn all_error_variants_display() {
        let errors: Vec<TransportError> = vec![
            TransportError::Spawn("test".into()),
            TransportError::StdinNotAvailable,
            TransportError::StdoutNotAvailable,
            TransportError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "file")),
            TransportError::Serialize(serde_json::from_str::<()>("invalid").unwrap_err()),
            TransportError::Parse("parse".into()),
            TransportError::ProcessExited,
            TransportError::Timeout(1000),
            TransportError::IdMismatch {
                request: 1,
                response: 2,
            },
            TransportError::JsonRpc(JsonRpcError {
                code: -32600,
                message: "test".into(),
                data: None,
            }),
            TransportError::Shutdown,
        ];

        for err in errors {
            let msg = format!("{}", err);
            assert!(!msg.is_empty(), "Error should have display: {:?}", err);
        }
    }
}

mod concurrent_operations {
    use super::*;

    #[tokio::test]
    async fn sequential_requests_maintain_order() {
        // Verifies ID generation is sequential
        let def = McpServerDef {
            command: "python3".to_string(),
            args: vec![
                "-c".to_string(),
                r#"
import sys, json
for line in sys.stdin:
    req = json.loads(line)
    print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"id": req["id"]}}), flush=True)
"#
                .to_string(),
            ],
            ..Default::default()
        };

        let transport = StdioTransport::spawn(&def, "test", false).await.unwrap();

        for expected_id in 1..=5 {
            let result = transport.request("test", None, 5000).await.unwrap();
            assert_eq!(result["id"], expected_id);
        }

        transport.shutdown().await.unwrap();
    }
}
