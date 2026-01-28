// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn test_hook_event_serialization() {
    let event = HookEvent::PreToolExecution;
    let json = serde_json::to_string(&event).unwrap();
    assert_eq!(json, "\"pre_tool_execution\"");

    let parsed: HookEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, event);
}

#[test]
fn test_hook_message_tool_execution() {
    let msg = HookMessage::tool_execution(
        "session_123",
        HookEvent::PreToolExecution,
        "Bash",
        serde_json::json!({"command": "ls -la"}),
        None,
    );

    assert_eq!(msg.event, HookEvent::PreToolExecution);
    assert_eq!(msg.session_id, "session_123");

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"tool_name\":\"Bash\""));
}

#[test]
fn test_hook_message_notification() {
    let msg = HookMessage::notification(
        "session_123",
        NotificationLevel::Warning,
        "Warning",
        "Something happened",
    );

    assert_eq!(msg.event, HookEvent::Notification);
    if let HookPayload::Notification {
        level,
        title,
        message,
    } = &msg.payload
    {
        assert_eq!(*level, NotificationLevel::Warning);
        assert_eq!(title, "Warning");
        assert_eq!(message, "Something happened");
    } else {
        unreachable!("Expected Notification payload");
    }
}

#[test]
fn test_hook_message_permission() {
    let msg = HookMessage::permission(
        "session_123",
        "Bash",
        "execute",
        serde_json::json!({"command": "rm -rf"}),
    );

    assert_eq!(msg.event, HookEvent::PermissionRequest);
}

#[test]
fn test_hook_message_session() {
    let msg = HookMessage::session(
        "session_123",
        HookEvent::SessionStart,
        Some("/project".to_string()),
    );

    assert_eq!(msg.event, HookEvent::SessionStart);
    if let HookPayload::Session { project_path } = &msg.payload {
        assert_eq!(*project_path, Some("/project".to_string()));
    } else {
        unreachable!("Expected Session payload");
    }
}

#[test]
fn test_hook_response_proceed() {
    let resp = HookResponse::proceed();
    assert!(resp.proceed);
    assert!(resp.error.is_none());

    let json = serde_json::to_string(&resp).unwrap();
    let parsed: HookResponse = serde_json::from_str(&json).unwrap();
    assert!(parsed.proceed);
}

#[test]
fn test_hook_response_block() {
    let resp = HookResponse::block("Not allowed");
    assert!(!resp.proceed);
    assert_eq!(resp.error, Some("Not allowed".to_string()));
}

#[test]
fn test_hook_response_with_modified() {
    let resp = HookResponse::proceed().with_modified(serde_json::json!({"modified": true}));
    assert!(resp.proceed);
    assert!(resp.modified_payload.is_some());
}

#[test]
fn test_hook_response_default_proceed() {
    // Test that deserialization defaults proceed to true
    let json = r#"{"error": null}"#;
    let resp: HookResponse = serde_json::from_str(json).unwrap();
    assert!(resp.proceed);
}

#[test]
fn test_notification_level_serialization() {
    let level = NotificationLevel::Error;
    let json = serde_json::to_string(&level).unwrap();
    assert_eq!(json, "\"error\"");
}

#[test]
fn test_hook_payload_serialization() {
    let payload = HookPayload::ToolExecution {
        tool_name: "Read".to_string(),
        tool_input: serde_json::json!({"file_path": "/test.txt"}),
        tool_output: Some("content".to_string()),
    };

    let json = serde_json::to_string(&payload).unwrap();
    assert!(json.contains("\"type\":\"tool_execution\""));
    assert!(json.contains("\"tool_name\":\"Read\""));
}

// =========================================================================
// Protocol Validation Tests
// =========================================================================
// These tests verify the hook protocol matches Claude Code documentation

#[test]
fn test_pre_tool_execution_payload_matches_spec() {
    let msg = HookMessage::tool_execution(
        "test-session",
        HookEvent::PreToolExecution,
        "Bash",
        serde_json::json!({"command": "ls -la"}),
        None,
    );

    let json = serde_json::to_value(&msg).unwrap();

    // Verify structure matches documentation
    assert_eq!(json["event"], "pre_tool_execution");
    assert!(json["session_id"].is_string());
    assert_eq!(json["payload"]["tool_name"], "Bash");
    assert!(json["payload"]["tool_input"].is_object());
    // tool_output should not be serialized when None
    assert!(json["payload"].get("tool_output").is_none());
}

#[test]
fn test_post_tool_execution_includes_output() {
    let msg = HookMessage::tool_execution(
        "test-session",
        HookEvent::PostToolExecution,
        "Bash",
        serde_json::json!({"command": "ls"}),
        Some("file1\nfile2".to_string()),
    );

    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["event"], "post_tool_execution");
    assert_eq!(json["payload"]["tool_output"], "file1\nfile2");
}

#[test]
fn test_hook_response_minimal_parsing() {
    // Real Claude accepts minimal response: just {"proceed": true}
    let response_json = r#"{"proceed": true}"#;
    let response: HookResponse = serde_json::from_str(response_json).unwrap();
    assert!(response.proceed);
    assert!(response.error.is_none());
    assert!(response.modified_payload.is_none());
}

#[test]
fn test_hook_response_block_with_error() {
    let response_json = r#"{"proceed": false, "error": "Permission denied"}"#;
    let response: HookResponse = serde_json::from_str(response_json).unwrap();
    assert!(!response.proceed);
    assert_eq!(response.error.as_deref(), Some("Permission denied"));
}

#[test]
fn test_notification_payload_has_required_fields() {
    let msg = HookMessage::notification(
        "test-session",
        NotificationLevel::Info,
        "Task Complete",
        "Your code has been generated.",
    );

    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["event"], "notification");
    assert_eq!(json["payload"]["level"], "info");
    assert_eq!(json["payload"]["title"], "Task Complete");
    assert_eq!(json["payload"]["message"], "Your code has been generated.");
}

#[test]
fn test_notification_levels() {
    // Verify all notification levels serialize correctly
    let levels = [
        (NotificationLevel::Info, "info"),
        (NotificationLevel::Warning, "warning"),
        (NotificationLevel::Error, "error"),
    ];

    for (level, expected) in levels {
        let json = serde_json::to_value(&level).unwrap();
        assert_eq!(json, expected);
    }
}

#[test]
fn test_session_start_payload() {
    let msg = HookMessage::session(
        "test-session",
        HookEvent::SessionStart,
        Some("/path/to/project".to_string()),
    );

    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["event"], "session_start");
    assert_eq!(json["session_id"], "test-session");
    assert_eq!(json["payload"]["project_path"], "/path/to/project");
}

#[test]
fn test_session_end_payload() {
    let msg = HookMessage::session("test-session", HookEvent::SessionEnd, None);

    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["event"], "session_end");
    // project_path should not be serialized when None
    assert!(json["payload"].get("project_path").is_none());
}

#[test]
fn test_prompt_submit_payload() {
    let msg = HookMessage::prompt_submit("test-session", "Hello, Claude!");

    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["event"], "prompt_submit");
    assert_eq!(json["payload"]["prompt"], "Hello, Claude!");
}

#[test]
fn test_hook_message_full_serialization() {
    // Test that a full hook message can round-trip
    let msg = HookMessage::tool_execution(
        "session-abc123",
        HookEvent::PreToolExecution,
        "Edit",
        serde_json::json!({
            "file_path": "/src/main.rs",
            "old_string": "foo",
            "new_string": "bar"
        }),
        None,
    );

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: HookMessage = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.event, msg.event);
    assert_eq!(parsed.session_id, msg.session_id);
}

#[test]
fn test_hook_response_with_modified_payload() {
    let response_json = r#"{
        "proceed": true,
        "modified_payload": {"command": "ls -la /safe/path"}
    }"#;
    let response: HookResponse = serde_json::from_str(response_json).unwrap();

    assert!(response.proceed);
    assert!(response.modified_payload.is_some());
    assert_eq!(
        response.modified_payload.unwrap()["command"],
        "ls -la /safe/path"
    );
}

#[test]
fn test_hook_response_with_data() {
    let response_json = r#"{
        "proceed": true,
        "data": {"custom_field": "custom_value"}
    }"#;
    let response: HookResponse = serde_json::from_str(response_json).unwrap();

    assert!(response.proceed);
    assert!(response.data.is_some());
    assert_eq!(response.data.unwrap()["custom_field"], "custom_value");
}
