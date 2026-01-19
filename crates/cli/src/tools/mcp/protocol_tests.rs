#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_json_rpc_request_serialization() {
    let req = JsonRpcRequest::new(1, "test", Some(serde_json::json!({ "key": "value" })));
    let json = serde_json::to_string(&req).unwrap();

    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"id\":1"));
    assert!(json.contains("\"method\":\"test\""));
}

#[test]
fn test_initialize_request() {
    let req = JsonRpcRequest::initialize(1, McpInitializeParams::default()).unwrap();
    assert_eq!(req.method, "initialize");
    assert!(req.params.is_some());
}

#[test]
fn test_tools_list_request() {
    let req = JsonRpcRequest::tools_list(2);
    assert_eq!(req.method, "tools/list");
}

#[test]
fn test_tools_call_request() {
    let params = McpToolCallParams::new("read_file", Some(serde_json::json!({ "path": "/tmp" })));
    let req = JsonRpcRequest::tools_call(3, params).unwrap();
    assert_eq!(req.method, "tools/call");
}

#[test]
fn test_json_rpc_response_success() {
    let json = r#"{"jsonrpc":"2.0","id":1,"result":{"key":"value"}}"#;
    let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();

    assert!(!resp.is_error());
    assert!(resp.result.is_some());
}

#[test]
fn test_json_rpc_response_error() {
    let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
    let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();

    assert!(resp.is_error());
    assert_eq!(resp.error.as_ref().unwrap().code, -32600);
}

#[test]
fn test_mcp_content_text() {
    let content = McpContent::Text {
        text: "Hello".to_string(),
    };
    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains("\"type\":\"text\""));
    assert!(json.contains("\"text\":\"Hello\""));
}

#[test]
fn test_mcp_tool_call_result() {
    let json = r#"{"content":[{"type":"text","text":"output"}],"isError":false}"#;
    let result: McpToolCallResult = serde_json::from_str(json).unwrap();

    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}
