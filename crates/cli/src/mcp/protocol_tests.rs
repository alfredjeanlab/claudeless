#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use serde_json::json;

#[test]
fn serialize_initialize_params() {
    let params = InitializeParams::default();
    let json = serde_json::to_value(&params).unwrap();

    assert_eq!(json["protocolVersion"], "2024-11-05");
    assert_eq!(json["clientInfo"]["name"], "claudeless");
    assert!(json["capabilities"].is_object());
}

#[test]
fn serialize_initialize_params_omits_none_experimental() {
    let params = InitializeParams::default();
    let json = serde_json::to_value(&params).unwrap();

    // experimental should not be present when None
    assert!(!json["capabilities"]
        .as_object()
        .unwrap()
        .contains_key("experimental"));
}

#[test]
fn deserialize_initialize_result() {
    let json = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": { "listChanged": true }
        },
        "serverInfo": {
            "name": "test-server",
            "version": "1.0.0"
        }
    });

    let result: InitializeResult = serde_json::from_value(json).unwrap();
    assert_eq!(result.protocol_version, "2024-11-05");
    assert_eq!(result.server_info.name, "test-server");
    assert_eq!(result.server_info.version, Some("1.0.0".into()));
    assert!(result.capabilities.tools.is_some());
    assert!(result.capabilities.tools.unwrap().list_changed);
}

#[test]
fn deserialize_initialize_result_minimal() {
    let json = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "serverInfo": {
            "name": "minimal-server"
        }
    });

    let result: InitializeResult = serde_json::from_value(json).unwrap();
    assert_eq!(result.server_info.name, "minimal-server");
    assert!(result.server_info.version.is_none());
    assert!(result.capabilities.tools.is_none());
}

#[test]
fn deserialize_tools_list() {
    let json = json!({
        "tools": [
            {
                "name": "read_file",
                "description": "Read file contents",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } }
                }
            }
        ]
    });

    let result: ToolsListResult = serde_json::from_value(json).unwrap();
    assert_eq!(result.tools.len(), 1);
    assert_eq!(result.tools[0].name, "read_file");
    assert_eq!(
        result.tools[0].description,
        Some("Read file contents".into())
    );
}

#[test]
fn deserialize_tools_list_no_description() {
    let json = json!({
        "tools": [
            {
                "name": "minimal_tool",
                "inputSchema": { "type": "object" }
            }
        ]
    });

    let result: ToolsListResult = serde_json::from_value(json).unwrap();
    assert_eq!(result.tools.len(), 1);
    assert_eq!(result.tools[0].name, "minimal_tool");
    assert!(result.tools[0].description.is_none());
}

#[test]
fn deserialize_content_block_text() {
    let text_json = json!({"type": "text", "text": "hello"});
    let text: ContentBlock = serde_json::from_value(text_json).unwrap();
    assert_eq!(text.as_text(), Some("hello"));
}

#[test]
fn deserialize_content_block_image() {
    let img_json = json!({
        "type": "image",
        "data": "base64...",
        "mimeType": "image/png"
    });
    let img: ContentBlock = serde_json::from_value(img_json).unwrap();
    assert!(img.as_text().is_none());
    assert!(
        matches!(img, ContentBlock::Image { ref data, ref mime_type } if data == "base64..." && mime_type == "image/png")
    );
}

#[test]
fn deserialize_content_block_resource() {
    let res_json = json!({
        "type": "resource",
        "uri": "file:///path",
        "text": "content"
    });
    let res: ContentBlock = serde_json::from_value(res_json).unwrap();
    assert!(
        matches!(res, ContentBlock::Resource { uri, text: Some(t), .. } if uri == "file:///path" && t == "content")
    );
}

#[test]
fn deserialize_content_block_resource_minimal() {
    let res_json = json!({
        "type": "resource",
        "uri": "file:///path"
    });
    let res: ContentBlock = serde_json::from_value(res_json).unwrap();
    assert!(
        matches!(res, ContentBlock::Resource { uri, text: None, mime_type: None } if uri == "file:///path")
    );
}

#[test]
fn serialize_content_block_roundtrip() {
    let blocks = vec![
        ContentBlock::Text {
            text: "hello".into(),
        },
        ContentBlock::Image {
            data: "abc123".into(),
            mime_type: "image/png".into(),
        },
        ContentBlock::Resource {
            uri: "file:///test".into(),
            text: Some("content".into()),
            mime_type: Some("text/plain".into()),
        },
    ];

    for block in blocks {
        let json = serde_json::to_value(&block).unwrap();
        let roundtrip: ContentBlock = serde_json::from_value(json).unwrap();
        assert_eq!(format!("{:?}", block), format!("{:?}", roundtrip));
    }
}

#[test]
fn tool_call_result_conversion_success() {
    let result = ToolCallResult {
        content: vec![ContentBlock::Text {
            text: "output".into(),
        }],
        is_error: false,
    };

    let mcp_result = result.into_tool_result();
    assert!(mcp_result.success);
    assert!(mcp_result.error.is_none());
}

#[test]
fn tool_call_result_conversion_error() {
    let error_result = ToolCallResult {
        content: vec![ContentBlock::Text {
            text: "error message".into(),
        }],
        is_error: true,
    };

    let mcp_error = error_result.into_tool_result();
    assert!(!mcp_error.success);
    assert_eq!(mcp_error.error, Some("error message".into()));
}

#[test]
fn tool_call_result_conversion_multiline_error() {
    let error_result = ToolCallResult {
        content: vec![
            ContentBlock::Text {
                text: "line 1".into(),
            },
            ContentBlock::Text {
                text: "line 2".into(),
            },
        ],
        is_error: true,
    };

    let mcp_error = error_result.into_tool_result();
    assert!(!mcp_error.success);
    assert_eq!(mcp_error.error, Some("line 1\nline 2".into()));
}

#[test]
fn tool_call_result_conversion_error_ignores_non_text() {
    let error_result = ToolCallResult {
        content: vec![
            ContentBlock::Text {
                text: "error text".into(),
            },
            ContentBlock::Image {
                data: "ignored".into(),
                mime_type: "image/png".into(),
            },
        ],
        is_error: true,
    };

    let mcp_error = error_result.into_tool_result();
    assert_eq!(mcp_error.error, Some("error text".into()));
}

#[test]
fn tool_info_to_def_conversion() {
    let info = ToolInfo {
        name: "test_tool".into(),
        description: Some("A test tool".into()),
        input_schema: json!({"type": "object"}),
    };

    let def = info.into_tool_def("my-server");
    assert_eq!(def.name, "test_tool");
    assert_eq!(def.description, "A test tool");
    assert_eq!(def.server_name, "my-server");
    assert_eq!(def.input_schema, json!({"type": "object"}));
}

#[test]
fn tool_info_to_def_conversion_no_description() {
    let info = ToolInfo {
        name: "no_desc_tool".into(),
        description: None,
        input_schema: json!({}),
    };

    let def = info.into_tool_def("server");
    assert_eq!(def.name, "no_desc_tool");
    assert_eq!(def.description, ""); // None becomes empty string
}

#[test]
fn serialize_tool_call_params() {
    let params = ToolCallParams {
        name: "test_tool".into(),
        arguments: Some(json!({"key": "value"})),
    };

    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["name"], "test_tool");
    assert_eq!(json["arguments"]["key"], "value");
}

#[test]
fn serialize_tool_call_params_no_arguments() {
    let params = ToolCallParams {
        name: "no_args_tool".into(),
        arguments: None,
    };

    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["name"], "no_args_tool");
    // arguments should not be present when None
    assert!(!json.as_object().unwrap().contains_key("arguments"));
}

#[test]
fn protocol_version_constant() {
    assert_eq!(PROTOCOL_VERSION, "2024-11-05");
}

#[test]
fn client_info_default() {
    let info = ClientInfo::default();
    assert_eq!(info.name, "claudeless");
    // Version should be the crate version
    assert!(!info.version.is_empty());
}
