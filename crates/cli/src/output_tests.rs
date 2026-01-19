#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_text_output() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(&mut buf, OutputFormat::Text, "claude-test".to_string());

    let response = ResponseSpec::Simple("Hello, world!".to_string());
    writer.write_response(&response, &[]).unwrap();

    let output = String::from_utf8(buf).unwrap();
    assert_eq!(output.trim(), "Hello, world!");
}

#[test]
fn test_text_output_detailed() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(&mut buf, OutputFormat::Text, "claude-test".to_string());

    let response = ResponseSpec::Detailed {
        text: "Detailed response".to_string(),
        tool_calls: vec![],
        usage: None,
        delay_ms: None,
    };
    writer.write_response(&response, &[]).unwrap();

    let output = String::from_utf8(buf).unwrap();
    assert_eq!(output.trim(), "Detailed response");
}

#[test]
fn test_json_output() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(&mut buf, OutputFormat::Json, "claude-test".to_string());

    let response = ResponseSpec::Simple("Test response".to_string());
    writer.write_response(&response, &[]).unwrap();

    let output = String::from_utf8(buf).unwrap();
    let json: JsonResponse = serde_json::from_str(&output).unwrap();

    assert_eq!(json.model, "claude-test");
    assert_eq!(json.response_type, "message");
    assert_eq!(json.role, "assistant");
    assert_eq!(json.stop_reason, "end_turn");
    assert_eq!(json.content.len(), 1);
    assert!(matches!(&json.content[0], ContentBlock::Text { text } if text == "Test response"));
}

#[test]
fn test_json_output_with_tool_calls() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(&mut buf, OutputFormat::Json, "claude-test".to_string());

    let response = ResponseSpec::Simple("Running command".to_string());
    let tool_calls = vec![ToolCallSpec {
        tool: "Bash".to_string(),
        input: serde_json::json!({ "command": "ls -la" }),
        result: None,
    }];
    writer.write_response(&response, &tool_calls).unwrap();

    let output = String::from_utf8(buf).unwrap();
    let json: JsonResponse = serde_json::from_str(&output).unwrap();

    assert_eq!(json.content.len(), 2);
    assert!(matches!(&json.content[0], ContentBlock::Text { text } if text == "Running command"));
    assert!(matches!(&json.content[1], ContentBlock::ToolUse { name, .. } if name == "Bash"));
}

#[test]
fn test_json_output_with_usage() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(&mut buf, OutputFormat::Json, "claude-test".to_string());

    let response = ResponseSpec::Detailed {
        text: "Response".to_string(),
        tool_calls: vec![],
        usage: Some(UsageSpec {
            input_tokens: 50,
            output_tokens: 25,
        }),
        delay_ms: None,
    };
    writer.write_response(&response, &[]).unwrap();

    let output = String::from_utf8(buf).unwrap();
    let json: JsonResponse = serde_json::from_str(&output).unwrap();

    assert_eq!(json.usage.input_tokens, 50);
    assert_eq!(json.usage.output_tokens, 25);
}

#[test]
fn test_stream_json_output() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(
        &mut buf,
        OutputFormat::StreamJson,
        "claude-test".to_string(),
    );

    let response = ResponseSpec::Simple("Hello!".to_string());
    writer.write_response(&response, &[]).unwrap();

    let output = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    // Should have: message_start, content_block_start, content_block_delta(s),
    // content_block_stop, message_delta, message_stop
    assert!(lines.len() >= 5);

    // First line is message_start
    let start: StreamEvent = serde_json::from_str(lines[0]).unwrap();
    assert!(matches!(start, StreamEvent::MessageStart { .. }));

    // Last line is message_stop
    let stop: StreamEvent = serde_json::from_str(lines[lines.len() - 1]).unwrap();
    assert!(matches!(stop, StreamEvent::MessageStop));
}

#[test]
fn test_stream_json_with_tool_calls() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(
        &mut buf,
        OutputFormat::StreamJson,
        "claude-test".to_string(),
    );

    let response = ResponseSpec::Simple("Running".to_string());
    let tool_calls = vec![ToolCallSpec {
        tool: "Bash".to_string(),
        input: serde_json::json!({ "command": "pwd" }),
        result: None,
    }];
    writer.write_response(&response, &tool_calls).unwrap();

    let output = String::from_utf8(buf).unwrap();

    // Should contain tool use content block
    assert!(output.contains("tool_use"));
    assert!(output.contains("Bash"));
}

#[test]
fn test_estimate_tokens() {
    assert_eq!(estimate_tokens(""), 1); // Min 1
    assert_eq!(estimate_tokens("test"), 1);
    assert_eq!(estimate_tokens("hello world"), 2); // 11 chars / 4 = 2
    assert_eq!(
        estimate_tokens("this is a longer string with more tokens"),
        10
    );
}

#[test]
fn test_json_response_serialization() {
    let response = JsonResponse {
        id: "msg_123".to_string(),
        model: "claude-test".to_string(),
        response_type: "message".to_string(),
        role: "assistant".to_string(),
        content: vec![ContentBlock::Text {
            text: "Hello".to_string(),
        }],
        stop_reason: "end_turn".to_string(),
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
        },
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: JsonResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.id, response.id);
    assert_eq!(parsed.model, response.model);
}

#[test]
fn test_stream_event_serialization() {
    let event = StreamEvent::ContentBlockDelta {
        index: 0,
        delta: Delta::TextDelta {
            text: "chunk".to_string(),
        },
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("content_block_delta"));
    assert!(json.contains("text_delta"));
    assert!(json.contains("chunk"));
}

// =========================================================================
// Real Claude Format Tests
// =========================================================================

#[test]
fn test_result_output_success() {
    let result = ResultOutput::success("Hello!".to_string(), "session-123".to_string(), 1000);

    let json = serde_json::to_string(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "result");
    assert_eq!(parsed["subtype"], "success");
    assert_eq!(parsed["is_error"], false);
    assert!(parsed["result"].is_string());
    assert!(parsed["session_id"].is_string());
    assert!(parsed["duration_ms"].is_number());
}

#[test]
fn test_result_output_error() {
    let result = ResultOutput::error(
        "Something went wrong".to_string(),
        "session-123".to_string(),
        100,
    );

    let json = serde_json::to_string(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "result");
    assert_eq!(parsed["subtype"], "error");
    assert_eq!(parsed["is_error"], true);
    assert!(parsed["error"].is_string());
    assert!(parsed.get("result").is_none());
}

#[test]
fn test_result_output_rate_limit() {
    let result = ResultOutput::rate_limit(60, "session-123".to_string());

    let json = serde_json::to_string(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "result");
    assert_eq!(parsed["subtype"], "error");
    assert_eq!(parsed["retry_after"], 60);
}

#[test]
fn test_system_init_event() {
    let init = SystemInitEvent::new("session-123", vec!["Bash".to_string(), "Read".to_string()]);

    let json = serde_json::to_string(&init).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "system");
    assert_eq!(parsed["subtype"], "init");
    assert_eq!(parsed["session_id"], "session-123");
    assert_eq!(parsed["tools"], serde_json::json!(["Bash", "Read"]));
    assert_eq!(parsed["mcp_servers"], serde_json::json!([]));
}

#[test]
fn test_system_init_event_with_mcp_servers() {
    let init = SystemInitEvent::with_mcp_servers(
        "session-123",
        vec!["Bash".to_string(), "Read".to_string()],
        vec!["filesystem".to_string(), "github".to_string()],
    );

    let json = serde_json::to_string(&init).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "system");
    assert_eq!(parsed["subtype"], "init");
    assert_eq!(parsed["session_id"], "session-123");
    assert_eq!(parsed["tools"], serde_json::json!(["Bash", "Read"]));
    assert_eq!(
        parsed["mcp_servers"],
        serde_json::json!(["filesystem", "github"])
    );
}

#[test]
fn test_assistant_event_message_start() {
    let message = AssistantMessageContent {
        id: "msg_123".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        role: "assistant".to_string(),
        content: vec![],
        stop_reason: None,
    };
    let event = AssistantEvent::message_start(message);

    let json = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "assistant");
    assert_eq!(parsed["subtype"], "message_start");
    assert!(parsed["message"].is_object());
}

#[test]
fn test_assistant_event_message_delta() {
    let usage = ExtendedUsage::new(10, 5);
    let event = AssistantEvent::message_delta(usage);

    let json = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "assistant");
    assert_eq!(parsed["subtype"], "message_delta");
    assert!(parsed["usage"].is_object());
    assert_eq!(parsed["usage"]["input_tokens"], 10);
    assert_eq!(parsed["usage"]["output_tokens"], 5);
    assert_eq!(parsed["usage"]["cache_creation_input_tokens"], 0);
}

#[test]
fn test_assistant_event_message_stop() {
    let event = AssistantEvent::message_stop();

    let json = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "assistant");
    assert_eq!(parsed["subtype"], "message_stop");
}

#[test]
fn test_content_block_start_event() {
    let event = ContentBlockStartEvent::text(0);

    let json = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "content_block_start");
    assert_eq!(parsed["subtype"], "text");
    assert_eq!(parsed["index"], 0);
}

#[test]
fn test_content_block_delta_event() {
    let event = ContentBlockDeltaEvent::text(0, "Hello!");

    let json = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "content_block_delta");
    assert_eq!(parsed["subtype"], "text_delta");
    assert_eq!(parsed["index"], 0);
    assert_eq!(parsed["delta"], "Hello!");
}

#[test]
fn test_content_block_stop_event() {
    let event = ContentBlockStopEvent::new(0);

    let json = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "content_block_stop");
    assert_eq!(parsed["index"], 0);
}

#[test]
fn test_write_real_json_response() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(&mut buf, OutputFormat::Json, "claude-test".to_string());

    let response = ResponseSpec::Simple("Hello!".to_string());
    writer
        .write_real_response(&response, "session-123", vec![])
        .unwrap();

    let output = String::from_utf8(buf).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Should be result wrapper format
    assert_eq!(parsed["type"], "result");
    assert_eq!(parsed["subtype"], "success");
    assert_eq!(parsed["result"], "Hello!");
    assert_eq!(parsed["session_id"], "session-123");
}

#[test]
fn test_write_real_stream_json_response() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(
        &mut buf,
        OutputFormat::StreamJson,
        "claude-test".to_string(),
    );

    let response = ResponseSpec::Simple("Hello!".to_string());
    let tools = vec!["Bash".to_string(), "Read".to_string()];
    writer
        .write_real_response(&response, "session-123", tools)
        .unwrap();

    let output = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    // Should have system init as first line
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["type"], "system");
    assert_eq!(first["subtype"], "init");

    // Should have result as last line
    let last: serde_json::Value = serde_json::from_str(lines[lines.len() - 1]).unwrap();
    assert_eq!(last["type"], "result");
    assert_eq!(last["subtype"], "success");
}

#[test]
fn test_real_stream_json_event_sequence() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(
        &mut buf,
        OutputFormat::StreamJson,
        "claude-test".to_string(),
    );

    let response = ResponseSpec::Simple("Hi".to_string());
    writer
        .write_real_response(&response, "session-123", vec!["Bash".to_string()])
        .unwrap();

    let output = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    // Verify condensed event sequence (matches real Claude CLI):
    // 1. system (init)
    // 2. assistant (with full message, no subtype)
    // 3. result (success)
    assert_eq!(lines.len(), 3, "Should have exactly 3 events");

    let types: Vec<String> = lines
        .iter()
        .map(|line| {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            parsed["type"].as_str().unwrap().to_string()
        })
        .collect();

    assert_eq!(types[0], "system");
    assert_eq!(types[1], "assistant");
    assert_eq!(types[2], "result");

    // Verify system event has subtype "init"
    let system_event: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(system_event["subtype"], "init");

    // Verify assistant event has no subtype (condensed format)
    let assistant_event: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert!(assistant_event.get("subtype").is_none());
    assert!(assistant_event["message"].is_object());

    // Verify result event
    let result_event: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(result_event["subtype"], "success");
}
