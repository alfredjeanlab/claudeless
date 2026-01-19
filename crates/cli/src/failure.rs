// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Failure injection modes for testing error handling.

use crate::cli::FailureMode;
use crate::config::FailureSpec;
use crate::output::ResultOutput;
use std::io::Write;
use std::time::Duration;
use tokio::time::sleep;

/// Exit codes matching real Claude CLI behavior
pub mod exit_codes {
    /// Successful execution
    pub const SUCCESS: i32 = 0;
    /// General error (auth, network, etc.)
    pub const ERROR: i32 = 1;
    /// Partial response / interrupted
    pub const PARTIAL: i32 = 2;
    /// Interrupted by signal (Ctrl+C)
    pub const INTERRUPTED: i32 = 130;
}

/// Failure executor that simulates error conditions
pub struct FailureExecutor;

impl FailureExecutor {
    /// Execute a failure mode, writing appropriate error output
    pub async fn execute<W: Write>(
        spec: &FailureSpec,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        match spec {
            FailureSpec::NetworkUnreachable => Self::network_unreachable(writer),
            FailureSpec::ConnectionTimeout { after_ms } => {
                Self::connection_timeout(*after_ms, writer).await
            }
            FailureSpec::AuthError { message } => Self::auth_error(message, writer),
            FailureSpec::RateLimit { retry_after } => Self::rate_limit(*retry_after, writer),
            FailureSpec::OutOfCredits => Self::out_of_credits(writer),
            FailureSpec::PartialResponse { partial_text } => {
                Self::partial_response(partial_text, writer)
            }
            FailureSpec::MalformedJson { raw } => Self::malformed_json(raw, writer),
        }
    }

    /// Convert a CLI failure mode to a failure spec
    pub fn from_mode(mode: &FailureMode) -> FailureSpec {
        match mode {
            FailureMode::NetworkUnreachable => FailureSpec::NetworkUnreachable,
            FailureMode::ConnectionTimeout => FailureSpec::ConnectionTimeout { after_ms: 5000 },
            FailureMode::AuthError => FailureSpec::AuthError {
                message: "Invalid API key".to_string(),
            },
            FailureMode::RateLimit => FailureSpec::RateLimit { retry_after: 60 },
            FailureMode::OutOfCredits => FailureSpec::OutOfCredits,
            FailureMode::PartialResponse => FailureSpec::PartialResponse {
                partial_text: "I was going to say...".to_string(),
            },
            FailureMode::MalformedJson => FailureSpec::MalformedJson {
                raw: r#"{"type":"message","content":[{"#.to_string(),
            },
        }
    }

    fn network_unreachable<W: Write>(writer: &mut W) -> Result<(), std::io::Error> {
        writeln!(
            writer,
            "Error: Failed to connect to Claude API: Network is unreachable"
        )?;
        std::process::exit(1);
    }

    async fn connection_timeout<W: Write>(
        after_ms: u64,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        sleep(Duration::from_millis(after_ms)).await;
        writeln!(
            writer,
            "Error: Connection to Claude API timed out after {}ms",
            after_ms
        )?;
        std::process::exit(1);
    }

    fn auth_error<W: Write>(message: &str, writer: &mut W) -> Result<(), std::io::Error> {
        let error = serde_json::json!({
            "type": "error",
            "error": {
                "type": "authentication_error",
                "message": message
            }
        });
        writeln!(writer, "{}", error)?;
        std::process::exit(1);
    }

    fn rate_limit<W: Write>(retry_after: u64, writer: &mut W) -> Result<(), std::io::Error> {
        let error = serde_json::json!({
            "type": "error",
            "error": {
                "type": "rate_limit_error",
                "message": "Rate limit exceeded",
                "retry_after": retry_after
            }
        });
        writeln!(writer, "{}", error)?;
        std::process::exit(1);
    }

    fn out_of_credits<W: Write>(writer: &mut W) -> Result<(), std::io::Error> {
        let error = serde_json::json!({
            "type": "error",
            "error": {
                "type": "billing_error",
                "message": "Your account has no credits remaining"
            }
        });
        writeln!(writer, "{}", error)?;
        std::process::exit(1);
    }

    fn partial_response<W: Write>(partial: &str, writer: &mut W) -> Result<(), std::io::Error> {
        // Write start of stream, then abruptly stop
        write!(writer, "{}", partial)?;
        writer.flush()?;
        // Simulate stream interruption with non-zero exit
        std::process::exit(2);
    }

    fn malformed_json<W: Write>(raw: &str, writer: &mut W) -> Result<(), std::io::Error> {
        // Write malformed JSON that won't parse
        writeln!(writer, "{}", raw)
    }

    // =========================================================================
    // Real Claude Format Error Methods
    // =========================================================================
    // These methods write errors in the result wrapper format matching real Claude

    /// Execute a failure mode using real Claude's result wrapper format
    pub async fn execute_real_format<W: Write>(
        spec: &FailureSpec,
        writer: &mut W,
        session_id: &str,
    ) -> Result<i32, std::io::Error> {
        match spec {
            FailureSpec::NetworkUnreachable => {
                Self::write_real_error(
                    writer,
                    "Network error: Connection refused",
                    session_id,
                    5000,
                )?;
                Ok(exit_codes::ERROR)
            }
            FailureSpec::ConnectionTimeout { after_ms } => {
                sleep(Duration::from_millis(*after_ms)).await;
                Self::write_real_error(
                    writer,
                    &format!("Network error: Connection timed out after {}ms", after_ms),
                    session_id,
                    *after_ms,
                )?;
                Ok(exit_codes::ERROR)
            }
            FailureSpec::AuthError { message } => {
                Self::write_real_error(writer, message, session_id, 100)?;
                Ok(exit_codes::ERROR)
            }
            FailureSpec::RateLimit { retry_after } => {
                Self::write_real_rate_limit(writer, *retry_after, session_id)?;
                Ok(exit_codes::ERROR)
            }
            FailureSpec::OutOfCredits => {
                Self::write_real_error(
                    writer,
                    "Billing error: No credits remaining",
                    session_id,
                    100,
                )?;
                Ok(exit_codes::ERROR)
            }
            FailureSpec::PartialResponse { partial_text } => {
                write!(writer, "{}", partial_text)?;
                writer.flush()?;
                Ok(exit_codes::PARTIAL)
            }
            FailureSpec::MalformedJson { raw } => {
                writeln!(writer, "{}", raw)?;
                Ok(exit_codes::SUCCESS) // Malformed JSON is still written, exit 0
            }
        }
    }

    /// Write an error in real Claude's result wrapper format
    fn write_real_error<W: Write>(
        writer: &mut W,
        message: &str,
        session_id: &str,
        duration_ms: u64,
    ) -> Result<(), std::io::Error> {
        let result = ResultOutput::error(message.to_string(), session_id.to_string(), duration_ms);
        let json = serde_json::to_string(&result)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(writer, "{}", json)
    }

    /// Write a rate limit error in real Claude's result wrapper format
    fn write_real_rate_limit<W: Write>(
        writer: &mut W,
        retry_after: u64,
        session_id: &str,
    ) -> Result<(), std::io::Error> {
        let result = ResultOutput::rate_limit(retry_after, session_id.to_string());
        let json = serde_json::to_string(&result)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(writer, "{}", json)
    }
}

#[cfg(test)]
mod tests {
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
            _ => panic!("Expected AuthError"),
        }
    }

    #[test]
    fn test_from_mode_rate_limit() {
        let spec = FailureExecutor::from_mode(&FailureMode::RateLimit);
        match spec {
            FailureSpec::RateLimit { retry_after } => {
                assert_eq!(retry_after, 60);
            }
            _ => panic!("Expected RateLimit"),
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
            _ => panic!("Expected PartialResponse"),
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
            _ => panic!("Expected MalformedJson"),
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
}
