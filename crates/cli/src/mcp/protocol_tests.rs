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

mod initialization_edge_cases {
    use super::*;

    #[test]
    fn deserialize_initialize_result_with_extra_fields() {
        // Servers may include additional fields we don't know about
        let json = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}, "unknownCap": true},
            "serverInfo": {"name": "test", "extraField": "ignored"},
            "extraTopLevel": 123
        });

        let result: InitializeResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.protocol_version, "2024-11-05");
        assert_eq!(result.server_info.name, "test");
    }

    #[test]
    fn client_capabilities_empty_by_default() {
        let caps = ClientCapabilities::default();
        let json = serde_json::to_value(&caps).unwrap();
        // Should be empty object, not null
        assert!(json.is_object());
    }
}

mod tools_list_edge_cases {
    use super::*;

    #[test]
    fn deserialize_empty_tools_list() {
        let json = json!({"tools": []});
        let result: ToolsListResult = serde_json::from_value(json).unwrap();
        assert!(result.tools.is_empty());
    }

    #[test]
    fn deserialize_tool_with_complex_schema() {
        let json = json!({
            "tools": [{
                "name": "complex",
                "description": "Complex tool",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "required_field": {"type": "string"},
                        "optional_array": {
                            "type": "array",
                            "items": {"type": "number"}
                        }
                    },
                    "required": ["required_field"]
                }
            }]
        });

        let result: ToolsListResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.tools[0].name, "complex");
        assert!(result.tools[0].input_schema["required"].is_array());
    }
}

mod tool_call_edge_cases {
    use super::*;

    #[test]
    fn tool_call_result_empty_content() {
        let result = ToolCallResult {
            content: vec![],
            is_error: false,
        };
        let mcp_result = result.into_tool_result();
        assert!(mcp_result.success);
        assert!(mcp_result.content.is_array());
    }

    #[test]
    fn deserialize_tool_result_with_mixed_content() {
        let json = json!({
            "content": [
                {"type": "text", "text": "Output line 1"},
                {"type": "image", "data": "base64data", "mimeType": "image/png"},
                {"type": "text", "text": "Output line 2"}
            ],
            "isError": false
        });

        let result: ToolCallResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.content.len(), 3);

        // Extract text content
        let text: Vec<_> = result.content.iter().filter_map(|c| c.as_text()).collect();
        assert_eq!(text, vec!["Output line 1", "Output line 2"]);
    }

    #[test]
    fn tool_call_params_empty_arguments() {
        let params = ToolCallParams {
            name: "no_args".into(),
            arguments: Some(json!({})),
        };
        let json = serde_json::to_value(&params).unwrap();
        assert!(json["arguments"].is_object());
        assert!(json["arguments"].as_object().unwrap().is_empty());
    }
}

mod content_block_edge_cases {
    use super::*;

    #[test]
    fn resource_with_all_fields() {
        let json = json!({
            "type": "resource",
            "uri": "file:///test",
            "text": "content",
            "mimeType": "application/json"
        });
        let block: ContentBlock = serde_json::from_value(json).unwrap();
        match block {
            ContentBlock::Resource {
                uri,
                text,
                mime_type,
            } => {
                assert_eq!(uri, "file:///test");
                assert_eq!(text, Some("content".into()));
                assert_eq!(mime_type, Some("application/json".into()));
            }
            _ => panic!("Expected Resource"),
        }
    }

    #[test]
    fn text_block_with_empty_string() {
        let block = ContentBlock::Text { text: "".into() };
        assert_eq!(block.as_text(), Some(""));
    }

    #[test]
    fn image_block_preserves_base64() {
        let data = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let block = ContentBlock::Image {
            data: data.into(),
            mime_type: "image/png".into(),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["data"], data);
    }
}
