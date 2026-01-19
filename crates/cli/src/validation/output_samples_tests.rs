#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
