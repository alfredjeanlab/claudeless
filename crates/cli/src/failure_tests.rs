// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn test_from_mode_network_unreachable() {
    let spec = FailureExecutor::from_mode(&FailureMode::NetworkUnreachable);
    assert!(matches!(spec, FailureSpec::NetworkUnreachable));
}

#[test]
fn test_from_mode_connection_timeout() {
    let spec = FailureExecutor::from_mode(&FailureMode::ConnectionTimeout);
    assert!(matches!(
        spec,
        FailureSpec::ConnectionTimeout { after_ms: 5000 }
    ));
}

#[test]
fn test_from_mode_auth_error() {
    let spec = FailureExecutor::from_mode(&FailureMode::AuthError);
    match spec {
        FailureSpec::AuthError { message } => {
            assert_eq!(message, "Invalid API key");
        }
        _ => unreachable!("Expected AuthError"),
    }
}

#[test]
fn test_from_mode_rate_limit() {
    let spec = FailureExecutor::from_mode(&FailureMode::RateLimit);
    match spec {
        FailureSpec::RateLimit { retry_after } => {
            assert_eq!(retry_after, 60);
        }
        _ => unreachable!("Expected RateLimit"),
    }
}

#[test]
fn test_from_mode_out_of_credits() {
    let spec = FailureExecutor::from_mode(&FailureMode::OutOfCredits);
    assert!(matches!(spec, FailureSpec::OutOfCredits));
}

#[test]
fn test_from_mode_partial_response() {
    let spec = FailureExecutor::from_mode(&FailureMode::PartialResponse);
    match spec {
        FailureSpec::PartialResponse { partial_text } => {
            assert!(!partial_text.is_empty());
        }
        _ => unreachable!("Expected PartialResponse"),
    }
}

#[test]
fn test_from_mode_malformed_json() {
    let spec = FailureExecutor::from_mode(&FailureMode::MalformedJson);
    match spec {
        FailureSpec::MalformedJson { raw } => {
            // Verify it's actually malformed JSON
            assert!(serde_json::from_str::<serde_json::Value>(&raw).is_err());
        }
        _ => unreachable!("Expected MalformedJson"),
    }
}

#[test]
fn test_malformed_json_output() {
    let mut buf = Vec::new();
    let _spec = FailureSpec::MalformedJson {
        raw: r#"{"incomplete"#.to_string(),
    };

    // This should not exit, just write malformed output
    let result = FailureExecutor::malformed_json(r#"{"incomplete"#, &mut buf);
    assert!(result.is_ok());

    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains(r#"{"incomplete"#));

    // Verify it's actually unparseable
    assert!(serde_json::from_str::<serde_json::Value>(output.trim()).is_err());
}

// Note: Tests for network_unreachable, connection_timeout, auth_error,
// rate_limit, out_of_credits, and partial_response call std::process::exit()
// which cannot be tested directly. These are tested via integration tests.

// =========================================================================
// Real Claude Format Validation Tests
// =========================================================================

#[test]
fn test_exit_codes_match_real_claude() {
    // Verify exit codes match documented Claude CLI behavior
    assert_eq!(exit_codes::SUCCESS, 0);
    assert_eq!(exit_codes::ERROR, 1);
    assert_eq!(exit_codes::PARTIAL, 2);
    assert_eq!(exit_codes::INTERRUPTED, 130);
}

#[test]
fn test_write_real_error_format() {
    let mut buf = Vec::new();
    FailureExecutor::write_real_error(&mut buf, "Test error", "session-123", 100).unwrap();

    let output = String::from_utf8(buf).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Verify structure matches real Claude error format
    assert_eq!(parsed["type"], "result");
    assert_eq!(parsed["subtype"], "error");
    assert_eq!(parsed["is_error"], true);
    assert_eq!(parsed["error"], "Test error");
    assert_eq!(parsed["session_id"], "session-123");
    assert!(parsed["duration_ms"].is_number());
}

#[test]
fn test_write_real_rate_limit_format() {
    let mut buf = Vec::new();
    FailureExecutor::write_real_rate_limit(&mut buf, 60, "session-123").unwrap();

    let output = String::from_utf8(buf).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Verify structure matches real Claude rate limit format
    assert_eq!(parsed["type"], "result");
    assert_eq!(parsed["subtype"], "error");
    assert_eq!(parsed["is_error"], true);
    assert_eq!(parsed["retry_after"], 60);
    assert!(parsed["error"].as_str().unwrap().contains("Rate limited"));
}

#[tokio::test]
async fn test_execute_real_format_auth_error() {
    let mut buf = Vec::new();
    let spec = FailureSpec::AuthError {
        message: "Invalid API key".to_string(),
    };

    let exit_code = FailureExecutor::execute_real_format(&spec, &mut buf, "session-123")
        .await
        .unwrap();

    assert_eq!(exit_code, exit_codes::ERROR);

    let output = String::from_utf8(buf).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["type"], "result");
    assert_eq!(parsed["is_error"], true);
    assert_eq!(parsed["error"], "Invalid API key");
}

#[tokio::test]
async fn test_execute_real_format_rate_limit() {
    let mut buf = Vec::new();
    let spec = FailureSpec::RateLimit { retry_after: 30 };

    let exit_code = FailureExecutor::execute_real_format(&spec, &mut buf, "session-123")
        .await
        .unwrap();

    assert_eq!(exit_code, exit_codes::ERROR);

    let output = String::from_utf8(buf).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["retry_after"], 30);
}

#[tokio::test]
async fn test_execute_real_format_partial_response() {
    let mut buf = Vec::new();
    let spec = FailureSpec::PartialResponse {
        partial_text: "Hello, I was going to say...".to_string(),
    };

    let exit_code = FailureExecutor::execute_real_format(&spec, &mut buf, "session-123")
        .await
        .unwrap();

    // Partial response should exit with code 2
    assert_eq!(exit_code, exit_codes::PARTIAL);

    let output = String::from_utf8(buf).unwrap();
    assert_eq!(output, "Hello, I was going to say...");
}

#[tokio::test]
async fn test_execute_real_format_network_error() {
    let mut buf = Vec::new();
    let spec = FailureSpec::NetworkUnreachable;

    let exit_code = FailureExecutor::execute_real_format(&spec, &mut buf, "session-123")
        .await
        .unwrap();

    assert_eq!(exit_code, exit_codes::ERROR);

    let output = String::from_utf8(buf).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert!(parsed["error"].as_str().unwrap().contains("Network error"));
}
