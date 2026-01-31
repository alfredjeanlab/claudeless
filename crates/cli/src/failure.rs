// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Failure injection modes for testing error handling.

use crate::cli::FailureMode;
use crate::config::FailureSpec;
use crate::output::ResultOutput;
use crate::state::{to_io_json, StateWriter};
use parking_lot::RwLock;
use std::io::Write;
use std::sync::Arc;
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

    /// Execute failure with session recording.
    ///
    /// Writes error entry to JSONL before writing to stderr and exiting.
    /// This allows watchers to detect errors by parsing the session file.
    pub async fn execute_with_session<W: Write>(
        spec: &FailureSpec,
        writer: &mut W,
        state_writer: Option<&Arc<RwLock<StateWriter>>>,
    ) -> Result<(), std::io::Error> {
        // MalformedJson doesn't record to JSONL since it simulates corrupted output
        if matches!(spec, FailureSpec::MalformedJson { .. }) {
            return Self::execute(spec, writer).await;
        }

        // 1. Record to session JSONL if state_writer provided
        if let Some(sw) = state_writer {
            let (error, error_type, retry_after, duration) = match spec {
                FailureSpec::NetworkUnreachable => (
                    "Network error: Connection refused".to_string(),
                    Some("network_error"),
                    None,
                    5000u64,
                ),
                FailureSpec::ConnectionTimeout { after_ms } => (
                    format!("Network error: Connection timed out after {}ms", after_ms),
                    Some("timeout_error"),
                    None,
                    *after_ms,
                ),
                FailureSpec::AuthError { message } => {
                    (message.clone(), Some("authentication_error"), None, 100u64)
                }
                FailureSpec::RateLimit { retry_after } => (
                    format!("Rate limited. Retry after {} seconds.", retry_after),
                    Some("rate_limit_error"),
                    Some(*retry_after),
                    50u64,
                ),
                FailureSpec::OutOfCredits => (
                    "Billing error: No credits remaining".to_string(),
                    Some("billing_error"),
                    None,
                    100u64,
                ),
                FailureSpec::PartialResponse { partial_text } => (
                    format!("Partial response: {}", partial_text),
                    Some("partial_response"),
                    None,
                    1000u64,
                ),
                FailureSpec::MalformedJson { .. } => {
                    unreachable!("MalformedJson handled above")
                }
            };
            // Acquire lock only for the duration of the record_error call
            sw.read()
                .record_error(&error, error_type, retry_after, duration)?;
        }

        // 2. Execute original failure behavior
        Self::execute(spec, writer).await
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
        writeln!(writer, "{}", to_io_json(&result)?)
    }

    /// Write a rate limit error in real Claude's result wrapper format
    fn write_real_rate_limit<W: Write>(
        writer: &mut W,
        retry_after: u64,
        session_id: &str,
    ) -> Result<(), std::io::Error> {
        let result = ResultOutput::rate_limit(retry_after, session_id.to_string());
        writeln!(writer, "{}", to_io_json(&result)?)
    }
}

#[cfg(test)]
#[path = "failure_tests.rs"]
mod tests;
