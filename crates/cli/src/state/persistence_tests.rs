// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use std::io::BufRead;
use tempfile::tempdir;

#[test]
fn test_result_line_serialization() {
    let line = ResultLine {
        line_type: "result".to_string(),
        tool_use_id: "toolu_123".to_string(),
        content: "hello\n\nExit code: 0".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&line).unwrap();

    // Verify structure
    assert!(json.contains("\"type\":\"result\""));
    assert!(json.contains("\"toolUseId\":\"toolu_123\""));
    assert!(json.contains("Exit code: 0"));
}

#[test]
fn test_result_line_deserialization() {
    let json = r#"{"type":"result","toolUseId":"toolu_456","content":"output\n\nExit code: 1","timestamp":"2025-01-01T00:00:00Z"}"#;
    let line: ResultLine = serde_json::from_str(json).unwrap();

    assert_eq!(line.line_type, "result");
    assert_eq!(line.tool_use_id, "toolu_456");
    assert!(line.content.contains("Exit code: 1"));
}

#[test]
fn test_append_result_jsonl() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.jsonl");
    let timestamp = chrono::Utc::now();

    append_result_jsonl(&path, "toolu_abc", "hello world\n\nExit code: 0", timestamp).unwrap();

    // Read and verify
    let content = std::fs::read_to_string(&path).unwrap();
    let parsed: ResultLine = serde_json::from_str(&content).unwrap();

    assert_eq!(parsed.line_type, "result");
    assert_eq!(parsed.tool_use_id, "toolu_abc");
    assert!(parsed.content.contains("hello world"));
    assert!(parsed.content.contains("Exit code: 0"));
}

#[test]
fn test_assistant_message_with_stop_reason_end_turn() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.jsonl");
    let timestamp = chrono::Utc::now();

    let params = AssistantMessageParams {
        session_id: "session_123",
        assistant_uuid: "assistant_456",
        parent_uuid: "user_789",
        request_id: "req_abc",
        message_id: "msg_def",
        content: vec![ContentBlock::Text {
            text: "Done.".to_string(),
        }],
        model: "claude-sonnet-4-20250514",
        stop_reason: Some("end_turn"),
        cwd: "/tmp",
        version: "0.1.0",
        git_branch: "main",
        timestamp,
    };

    append_assistant_message_jsonl(&path, &params).unwrap();

    // Read and verify
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("\"stop_reason\":\"end_turn\""));
}

#[test]
fn test_assistant_message_with_stop_reason_tool_use() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.jsonl");
    let timestamp = chrono::Utc::now();

    let params = AssistantMessageParams {
        session_id: "session_123",
        assistant_uuid: "assistant_456",
        parent_uuid: "user_789",
        request_id: "req_abc",
        message_id: "msg_def",
        content: vec![ContentBlock::ToolUse {
            id: "toolu_123".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
        }],
        model: "claude-sonnet-4-20250514",
        stop_reason: Some("tool_use"),
        cwd: "/tmp",
        version: "0.1.0",
        git_branch: "main",
        timestamp,
    };

    append_assistant_message_jsonl(&path, &params).unwrap();

    // Read and verify
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("\"stop_reason\":\"tool_use\""));
}

#[test]
fn test_dual_record_strategy() {
    // Verify that a tool result should produce both user and result records
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.jsonl");
    let timestamp = chrono::Utc::now();

    // Write user message with tool result
    let user_params = UserMessageParams {
        session_id: "session_123",
        user_uuid: "user_456",
        parent_uuid: Some("assistant_789"),
        content: UserMessageContent::ToolResult {
            tool_use_id: "toolu_abc",
            content: "hello\n\nExit code: 0",
            tool_use_result: serde_json::json!({}),
            source_tool_assistant_uuid: "assistant_789",
        },
        cwd: "/tmp",
        version: "0.1.0",
        git_branch: "main",
        timestamp,
    };
    append_user_message_jsonl(&path, &user_params).unwrap();

    // Write result record
    append_result_jsonl(&path, "toolu_abc", "hello\n\nExit code: 0", timestamp).unwrap();

    // Verify both records exist
    let file = std::fs::File::open(&path).unwrap();
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"type\":\"user\""));
    assert!(lines[1].contains("\"type\":\"result\""));
}
