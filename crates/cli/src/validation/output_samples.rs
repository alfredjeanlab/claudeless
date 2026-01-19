// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Golden output samples for validation testing.
//!
//! These samples capture the expected output format from real Claude Code CLI
//! and are used to verify the simulator produces compatible output.

/// Sample JSON output from real Claude `--output-format json`
///
/// This is the result wrapper format used by Claude Code for non-streaming responses.
pub const REAL_JSON_RESULT: &str = r#"{
  "type": "result",
  "subtype": "success",
  "cost_usd": 0.003,
  "is_error": false,
  "duration_ms": 1234,
  "duration_api_ms": 1100,
  "num_turns": 1,
  "result": "Hello! How can I help you today?",
  "session_id": "01234567-89ab-cdef-0123-456789abcdef"
}"#;

/// Sample error result from real Claude
pub const REAL_ERROR_RESULT: &str = r#"{
  "type": "result",
  "subtype": "error",
  "cost_usd": 0,
  "is_error": true,
  "duration_ms": 123,
  "duration_api_ms": 100,
  "num_turns": 0,
  "error": "Invalid API key"
}"#;

/// Sample rate limit error result
pub const REAL_RATE_LIMIT_RESULT: &str = r#"{
  "type": "result",
  "subtype": "error",
  "cost_usd": 0,
  "is_error": true,
  "duration_ms": 50,
  "duration_api_ms": 50,
  "num_turns": 0,
  "error": "Rate limited. Retry after 60 seconds.",
  "retry_after": 60
}"#;

/// Sample stream-json init event
pub const REAL_STREAM_INIT: &str = r#"{"type":"system","subtype":"init","session_id":"01234567-89ab-cdef-0123-456789abcdef","tools":["Read","Write","Edit","Bash"],"mcp_servers":[]}"#;

/// Sample stream-json message start
pub const REAL_STREAM_MESSAGE_START: &str = r#"{"type":"assistant","subtype":"message_start","message":{"id":"msg_01234","model":"claude-sonnet-4-20250514","role":"assistant","content":[],"stop_reason":null}}"#;

/// Sample stream-json content block start
pub const REAL_STREAM_CONTENT_START: &str =
    r#"{"type":"content_block_start","subtype":"text","index":0}"#;

/// Sample stream-json content delta
pub const REAL_STREAM_CONTENT_DELTA: &str =
    r#"{"type":"content_block_delta","subtype":"text_delta","index":0,"delta":"Hello!"}"#;

/// Sample stream-json content stop
pub const REAL_STREAM_CONTENT_STOP: &str = r#"{"type":"content_block_stop","index":0}"#;

/// Sample stream-json message delta with usage
pub const REAL_STREAM_MESSAGE_DELTA: &str = r#"{"type":"assistant","subtype":"message_delta","usage":{"input_tokens":10,"output_tokens":8,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}"#;

/// Sample stream-json message stop
pub const REAL_STREAM_MESSAGE_STOP: &str = r#"{"type":"assistant","subtype":"message_stop"}"#;

/// Complete stream-json output sequence
pub const REAL_STREAM_JSON_SEQUENCE: &str = r#"{"type":"system","subtype":"init","session_id":"01234567-89ab-cdef-0123-456789abcdef","tools":["Read","Write","Edit","Bash"],"mcp_servers":[]}
{"type":"assistant","subtype":"message_start","message":{"id":"msg_01234","model":"claude-sonnet-4-20250514","role":"assistant","content":[],"stop_reason":null}}
{"type":"content_block_start","subtype":"text","index":0}
{"type":"content_block_delta","subtype":"text_delta","index":0,"delta":"Hello! "}
{"type":"content_block_delta","subtype":"text_delta","index":0,"delta":"How can I help you today?"}
{"type":"content_block_stop","index":0}
{"type":"assistant","subtype":"message_delta","usage":{"input_tokens":10,"output_tokens":8,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}
{"type":"assistant","subtype":"message_stop"}
{"type":"result","subtype":"success","cost_usd":0.003,"is_error":false,"duration_ms":1234,"duration_api_ms":1100,"num_turns":1,"result":"Hello! How can I help you today?","session_id":"01234567-89ab-cdef-0123-456789abcdef"}"#;

/// Required fields in JSON result output
pub const REQUIRED_JSON_RESULT_FIELDS: &[&str] = &[
    "type",
    "subtype",
    "cost_usd",
    "is_error",
    "duration_ms",
    "duration_api_ms",
    "num_turns",
    "session_id",
];

/// Required fields in error result output
pub const REQUIRED_ERROR_RESULT_FIELDS: &[&str] = &[
    "type",
    "subtype",
    "cost_usd",
    "is_error",
    "duration_ms",
    "duration_api_ms",
    "num_turns",
    "error",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_result_is_valid_json() {
        let parsed: serde_json::Value = serde_json::from_str(REAL_JSON_RESULT).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn test_error_result_is_valid_json() {
        let parsed: serde_json::Value = serde_json::from_str(REAL_ERROR_RESULT).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn test_stream_events_are_valid_json() {
        let samples = [
            REAL_STREAM_INIT,
            REAL_STREAM_MESSAGE_START,
            REAL_STREAM_CONTENT_START,
            REAL_STREAM_CONTENT_DELTA,
            REAL_STREAM_CONTENT_STOP,
            REAL_STREAM_MESSAGE_DELTA,
            REAL_STREAM_MESSAGE_STOP,
        ];

        for sample in samples {
            let parsed: serde_json::Value = serde_json::from_str(sample).unwrap();
            assert!(parsed.is_object(), "Failed to parse: {}", sample);
        }
    }

    #[test]
    fn test_stream_sequence_is_ndjson() {
        for line in REAL_STREAM_JSON_SEQUENCE.lines() {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(parsed.is_object(), "Failed to parse line: {}", line);
        }
    }

    #[test]
    fn test_json_result_has_required_fields() {
        let parsed: serde_json::Value = serde_json::from_str(REAL_JSON_RESULT).unwrap();

        for field in REQUIRED_JSON_RESULT_FIELDS {
            assert!(
                parsed.get(field).is_some(),
                "Missing required field: {}",
                field
            );
        }
    }

    #[test]
    fn test_json_result_types() {
        let parsed: serde_json::Value = serde_json::from_str(REAL_JSON_RESULT).unwrap();

        assert_eq!(parsed["type"], "result");
        assert_eq!(parsed["subtype"], "success");
        assert!(parsed["cost_usd"].is_number());
        assert!(parsed["is_error"].is_boolean());
        assert!(parsed["duration_ms"].is_number());
        assert!(parsed["session_id"].is_string());
    }

    #[test]
    fn test_error_result_has_error_field() {
        let parsed: serde_json::Value = serde_json::from_str(REAL_ERROR_RESULT).unwrap();

        assert!(parsed["is_error"].as_bool().unwrap());
        assert!(parsed["error"].is_string());
    }

    #[test]
    fn test_rate_limit_has_retry_after() {
        let parsed: serde_json::Value = serde_json::from_str(REAL_RATE_LIMIT_RESULT).unwrap();

        assert!(parsed["retry_after"].is_number());
        assert_eq!(parsed["retry_after"], 60);
    }
}
