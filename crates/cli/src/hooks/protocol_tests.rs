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
        "idle_prompt",
        "Idle",
        "Claude is waiting for input",
    );

    assert_eq!(msg.event, HookEvent::Notification);
    if let HookPayload::Notification {
        notification_type,
        title,
        message,
    } = &msg.payload
    {
        assert_eq!(notification_type, "idle_prompt");
        assert_eq!(title, "Idle");
        assert_eq!(message, "Claude is waiting for input");
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
fn test_notification_type_constants() {
    assert_eq!(NOTIFICATION_IDLE_PROMPT, "idle_prompt");
    assert_eq!(NOTIFICATION_PERMISSION_PROMPT, "permission_prompt");
    assert_eq!(NOTIFICATION_ELICITATION_DIALOG, "elicitation_dialog");
    assert_eq!(NOTIFICATION_AUTH_SUCCESS, "auth_success");
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
        NOTIFICATION_IDLE_PROMPT,
        "Task Complete",
        "Your code has been generated.",
    );

    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["event"], "notification");
    assert_eq!(json["payload"]["notification_type"], "idle_prompt");
    assert_eq!(json["payload"]["title"], "Task Complete");
    assert_eq!(json["payload"]["message"], "Your code has been generated.");
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

#[test]
fn test_pre_compact_event_serialization() {
    let event = HookEvent::PreCompact;
    let json = serde_json::to_string(&event).unwrap();
    assert_eq!(json, "\"pre_compact\"");

    let parsed: HookEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, event);
}

#[test]
fn test_stop_event_serialization() {
    let event = HookEvent::Stop;
    let json = serde_json::to_string(&event).unwrap();
    assert_eq!(json, "\"stop\"");

    let parsed: HookEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, event);
}

#[test]
fn test_compaction_trigger_serialization() {
    let manual = CompactionTrigger::Manual;
    let json = serde_json::to_value(&manual).unwrap();
    assert_eq!(json, "manual");

    let auto = CompactionTrigger::Auto;
    let json = serde_json::to_value(&auto).unwrap();
    assert_eq!(json, "auto");
}

#[test]
fn test_hook_message_compaction_manual() {
    let msg = HookMessage::compaction(
        "test-session",
        CompactionTrigger::Manual,
        Some("Focus on core functionality".to_string()),
    );

    assert_eq!(msg.event, HookEvent::PreCompact);
    assert_eq!(msg.session_id, "test-session");

    if let HookPayload::Compaction {
        trigger,
        custom_instructions,
    } = &msg.payload
    {
        assert_eq!(*trigger, CompactionTrigger::Manual);
        assert_eq!(
            *custom_instructions,
            Some("Focus on core functionality".to_string())
        );
    } else {
        unreachable!("Expected Compaction payload");
    }
}

#[test]
fn test_hook_message_stop() {
    let msg = HookMessage::stop("test-session", false);

    assert_eq!(msg.event, HookEvent::Stop);
    assert_eq!(msg.session_id, "test-session");
    if let HookPayload::Stop { stop_hook_active } = &msg.payload {
        assert!(!stop_hook_active);
    } else {
        unreachable!("Expected Stop payload");
    }
}

#[test]
fn test_hook_message_compaction_auto() {
    let msg = HookMessage::compaction("test-session", CompactionTrigger::Auto, None);

    assert_eq!(msg.event, HookEvent::PreCompact);

    if let HookPayload::Compaction {
        trigger,
        custom_instructions,
    } = &msg.payload
    {
        assert_eq!(*trigger, CompactionTrigger::Auto);
        assert!(custom_instructions.is_none());
    } else {
        unreachable!("Expected Compaction payload");
    }
}

#[test]
fn test_pre_compact_payload_matches_spec() {
    let msg = HookMessage::compaction(
        "session-abc123",
        CompactionTrigger::Manual,
        Some("Summarize the conversation".to_string()),
    );

    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["event"], "pre_compact");
    assert_eq!(json["session_id"], "session-abc123");
    assert_eq!(json["payload"]["type"], "compaction");
    assert_eq!(json["payload"]["trigger"], "manual");
    assert_eq!(
        json["payload"]["custom_instructions"],
        "Summarize the conversation"
    );
}

#[test]
fn test_pre_compact_auto_omits_custom_instructions() {
    let msg = HookMessage::compaction("test-session", CompactionTrigger::Auto, None);

    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["event"], "pre_compact");
    assert_eq!(json["payload"]["trigger"], "auto");
    // custom_instructions should not be serialized when None
    assert!(json["payload"].get("custom_instructions").is_none());
}

#[test]
fn test_stop_payload_matches_spec() {
    let msg = HookMessage::stop("test-session", true);

    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["event"], "stop");
    assert_eq!(json["session_id"], "test-session");
    assert_eq!(json["payload"]["stop_hook_active"], true);
}

#[test]
fn test_stop_hook_active_prevents_loops() {
    // Test with stop_hook_active = false (initial stop)
    let msg_initial = HookMessage::stop("session-1", false);
    let json_initial = serde_json::to_value(&msg_initial).unwrap();
    assert_eq!(json_initial["payload"]["stop_hook_active"], false);

    // Test with stop_hook_active = true (already continuing from hook)
    let msg_active = HookMessage::stop("session-1", true);
    let json_active = serde_json::to_value(&msg_active).unwrap();
    assert_eq!(json_active["payload"]["stop_hook_active"], true);
}

// =========================================================================
// StopHookResponse Tests
// =========================================================================

#[test]
fn test_stop_hook_response_allow() {
    let resp = StopHookResponse::allow();
    assert_eq!(resp.decision, "allow");
    assert!(resp.reason.is_none());
    assert!(!resp.is_blocked());
}

#[test]
fn test_stop_hook_response_block() {
    let resp = StopHookResponse::block("Please verify");
    assert_eq!(resp.decision, "block");
    assert_eq!(resp.reason, Some("Please verify".to_string()));
    assert!(resp.is_blocked());
}

#[test]
fn test_stop_hook_response_parse_allow() {
    let json = r#"{"decision": "allow"}"#;
    let resp: StopHookResponse = serde_json::from_str(json).unwrap();
    assert!(!resp.is_blocked());
    assert!(resp.reason.is_none());
}

#[test]
fn test_stop_hook_response_parse_block() {
    let json = r#"{"decision": "block", "reason": "continue with verification"}"#;
    let resp: StopHookResponse = serde_json::from_str(json).unwrap();
    assert!(resp.is_blocked());
    assert_eq!(resp.reason, Some("continue with verification".to_string()));
}

#[test]
fn test_stop_hook_response_default_decision() {
    // Empty object should default to "allow"
    let json = r#"{}"#;
    let resp: StopHookResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.decision, "allow");
    assert!(!resp.is_blocked());
}

#[test]
fn test_stop_hook_response_block_without_reason() {
    let json = r#"{"decision": "block"}"#;
    let resp: StopHookResponse = serde_json::from_str(json).unwrap();
    assert!(resp.is_blocked());
    assert!(resp.reason.is_none());
}

#[test]
fn test_stop_hook_response_serialization() {
    let resp = StopHookResponse::block("test reason");
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["decision"], "block");
    assert_eq!(json["reason"], "test reason");
}
