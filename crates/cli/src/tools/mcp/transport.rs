// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! MCP transport layer for stdio communication.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use super::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::mcp::McpServerDef;

/// MCP transport over stdio.
pub struct McpTransport {
    /// Child process handle.
    process: Child,

    /// Stdin writer.
    stdin: ChildStdin,

    /// Stdout reader.
    stdout: BufReader<ChildStdout>,

    /// Next request ID.
    next_id: u64,
}

impl McpTransport {
    /// Spawn a new MCP server process.
    pub fn spawn(def: &McpServerDef) -> Result<Self, McpTransportError> {
        let mut cmd = Command::new(&def.command);
        cmd.args(&def.args);

        // Set environment variables
        for (key, value) in &def.env {
            cmd.env(key, value);
        }

        // Set working directory if specified
        if let Some(ref cwd) = def.cwd {
            cmd.current_dir(cwd);
        }

        // Configure stdio for JSON-RPC communication
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut process = cmd.spawn().map_err(|e| McpTransportError::SpawnFailed {
            command: def.command.clone(),
            error: e.to_string(),
        })?;

        let stdin = process.stdin.take().ok_or(McpTransportError::NoStdin)?;
        let stdout = process.stdout.take().ok_or(McpTransportError::NoStdout)?;

        Ok(Self {
            process,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
        })
    }

    /// Send a JSON-RPC request and receive the response.
    pub fn request(
        &mut self,
        request: JsonRpcRequest,
    ) -> Result<JsonRpcResponse, McpTransportError> {
        // Serialize request to JSON (newline-delimited)
        let json = serde_json::to_string(&request).map_err(McpTransportError::SerializeFailed)?;

        // Write request
        writeln!(self.stdin, "{}", json).map_err(McpTransportError::WriteFailed)?;
        self.stdin.flush().map_err(McpTransportError::WriteFailed)?;

        // Read response
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .map_err(McpTransportError::ReadFailed)?;

        if line.is_empty() {
            return Err(McpTransportError::ConnectionClosed);
        }

        // Parse response
        let response: JsonRpcResponse =
            serde_json::from_str(&line).map_err(McpTransportError::DeserializeFailed)?;

        // Verify ID matches
        if response.id != request.id {
            return Err(McpTransportError::IdMismatch {
                expected: request.id,
                got: response.id,
            });
        }

        Ok(response)
    }

    /// Allocate the next request ID.
    pub fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Check if the process is still running.
    pub fn is_running(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }

    /// Kill the process.
    pub fn kill(&mut self) -> Result<(), McpTransportError> {
        self.process
            .kill()
            .map_err(|e| McpTransportError::KillFailed(e.to_string()))
    }
}

impl Drop for McpTransport {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

/// Transport errors.
#[derive(Debug)]
pub enum McpTransportError {
    /// Failed to spawn process.
    SpawnFailed { command: String, error: String },

    /// No stdin available.
    NoStdin,

    /// No stdout available.
    NoStdout,

    /// Failed to serialize request.
    SerializeFailed(serde_json::Error),

    /// Failed to deserialize response.
    DeserializeFailed(serde_json::Error),

    /// Failed to write to stdin.
    WriteFailed(std::io::Error),

    /// Failed to read from stdout.
    ReadFailed(std::io::Error),

    /// Connection closed unexpectedly.
    ConnectionClosed,

    /// Response ID doesn't match request.
    IdMismatch { expected: u64, got: u64 },

    /// Failed to kill process.
    KillFailed(String),
}

impl std::fmt::Display for McpTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpawnFailed { command, error } => {
                write!(f, "Failed to spawn '{}': {}", command, error)
            }
            Self::NoStdin => write!(f, "No stdin available"),
            Self::NoStdout => write!(f, "No stdout available"),
            Self::SerializeFailed(e) => write!(f, "Failed to serialize request: {}", e),
            Self::DeserializeFailed(e) => write!(f, "Failed to deserialize response: {}", e),
            Self::WriteFailed(e) => write!(f, "Failed to write to stdin: {}", e),
            Self::ReadFailed(e) => write!(f, "Failed to read from stdout: {}", e),
            Self::ConnectionClosed => write!(f, "Connection closed unexpectedly"),
            Self::IdMismatch { expected, got } => {
                write!(
                    f,
                    "Response ID mismatch: expected {}, got {}",
                    expected, got
                )
            }
            Self::KillFailed(e) => write!(f, "Failed to kill process: {}", e),
        }
    }
}

impl std::error::Error for McpTransportError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_spawn_nonexistent() {
        let def = McpServerDef {
            command: "nonexistent_command_12345".to_string(),
            args: vec![],
            env: Default::default(),
            cwd: None,
            timeout_ms: 30000,
        };

        let result = McpTransport::spawn(&def);
        assert!(result.is_err());
        if let Err(McpTransportError::SpawnFailed { command, .. }) = result {
            assert_eq!(command, "nonexistent_command_12345");
        } else {
            panic!("Expected SpawnFailed error");
        }
    }

    #[test]
    fn test_transport_error_display() {
        let err = McpTransportError::ConnectionClosed;
        assert_eq!(err.to_string(), "Connection closed unexpectedly");

        let err = McpTransportError::IdMismatch {
            expected: 1,
            got: 2,
        };
        assert!(err.to_string().contains("expected 1"));
    }
}
