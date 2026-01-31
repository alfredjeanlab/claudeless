// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

// NOTE(compat): Keep full API surface for future use
#![allow(dead_code)]

//! JSON-RPC stdio transport for MCP server communication.
//!
//! This module provides the low-level transport layer for communicating with MCP servers
//! over stdio. It handles process spawning, buffered I/O, JSON-RPC message serialization,
//! timeout handling, and graceful shutdown.
//!
//! # Example
//!
//! ```ignore
//! use crate::mcp::config::McpServerDef;
//! use crate::mcp::transport::StdioTransport;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let def = McpServerDef {
//!     command: "mcp-server".to_string(),
//!     ..Default::default()
//! };
//!
//! let transport = StdioTransport::spawn(&def, "server", false).await?;
//!
//! // Send a request and wait for response
//! let result = transport.request(
//!     "initialize",
//!     Some(serde_json::json!({"capabilities": {}})),
//!     30000,
//! ).await?;
//!
//! // Gracefully shut down
//! transport.shutdown().await?;
//! # Ok(())
//! # }
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

use super::config::McpServerDef;

// ============================================================================
// JSON-RPC Message Types
// ============================================================================

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: &'static str,
    /// Request identifier.
    pub id: u64,
    /// Method name to invoke.
    pub method: String,
    /// Optional parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request.
    pub fn new(id: u64, method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version.
    pub jsonrpc: String,
    /// Response identifier (matches request ID).
    pub id: u64,
    /// Result value on success.
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    /// Error object on failure.
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Convert response to Result, extracting either the result or error.
    pub fn into_result(self) -> Result<serde_json::Value, JsonRpcError> {
        if let Some(err) = self.error {
            Err(err)
        } else {
            Ok(self.result.unwrap_or(serde_json::Value::Null))
        }
    }
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Deserialize, thiserror::Error)]
#[error("JSON-RPC error {code}: {message}")]
pub struct JsonRpcError {
    /// Error code.
    pub code: i64,
    /// Human-readable error message.
    pub message: String,
    /// Additional error data.
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 notification (no id, no response expected).
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcNotification {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: &'static str,
    /// Method name.
    pub method: String,
    /// Optional parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcNotification {
    /// Create a new JSON-RPC notification.
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            method: method.into(),
            params,
        }
    }
}

// ============================================================================
// Transport Error Type
// ============================================================================

/// Errors that can occur during transport operations.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// Failed to spawn the child process.
    #[error("failed to spawn process: {0}")]
    Spawn(String),

    /// Process stdin not available (already taken or closed).
    #[error("stdin not available")]
    StdinNotAvailable,

    /// Process stdout not available (already taken or closed).
    #[error("stdout not available")]
    StdoutNotAvailable,

    /// IO error during read/write.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to serialize request to JSON.
    #[error("failed to serialize request: {0}")]
    Serialize(#[from] serde_json::Error),

    /// Failed to parse response as JSON.
    #[error("failed to parse response: {0}")]
    Parse(String),

    /// Response ID doesn't match request ID.
    #[error("response id {response} doesn't match request id {request}")]
    IdMismatch {
        /// The request ID that was sent.
        request: u64,
        /// The response ID that was received.
        response: u64,
    },

    /// JSON-RPC error response from server.
    #[error("{0}")]
    JsonRpc(#[from] JsonRpcError),

    /// Request timed out.
    #[error("request timed out after {0}ms")]
    Timeout(u64),

    /// Process exited unexpectedly.
    #[error("process exited unexpectedly")]
    ProcessExited,

    /// Transport is already shut down.
    #[error("transport is shut down")]
    Shutdown,
}

// ============================================================================
// StdioTransport
// ============================================================================

/// Stdio transport for JSON-RPC communication with a child process.
///
/// Manages spawning a child process, writing JSON-RPC requests to stdin,
/// and reading JSON-RPC responses from stdout. All messages are newline-delimited.
///
/// # Thread Safety
///
/// The transport is designed for safe concurrent access:
/// - `Mutex<Option<...>>` for stdin/stdout allows exclusive access during operations
/// - `AtomicU64` for request IDs avoids locking for ID generation
/// - `AtomicBool` for shutdown flag allows lock-free checks
pub struct StdioTransport {
    /// The child process.
    child: Mutex<Option<Child>>,

    /// Buffered writer for stdin.
    stdin: Mutex<Option<BufWriter<ChildStdin>>>,

    /// Buffered reader for stdout.
    stdout: Mutex<Option<BufReader<ChildStdout>>>,

    /// Next request ID (atomically incremented).
    next_id: AtomicU64,

    /// Whether the transport has been shut down.
    shutdown: AtomicBool,

    /// Enable debug logging of JSON-RPC messages.
    debug: bool,

    /// Server name for debug logging context.
    server_name: String,
}

impl std::fmt::Debug for StdioTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StdioTransport")
            .field("server_name", &self.server_name)
            .field("next_id", &self.next_id.load(Ordering::Relaxed))
            .field("shutdown", &self.shutdown.load(Ordering::Relaxed))
            .field("debug", &self.debug)
            .finish_non_exhaustive()
    }
}

impl StdioTransport {
    /// Spawn a new child process and create a transport for communication.
    ///
    /// The process is spawned with stdin/stdout piped for JSON-RPC communication.
    /// Stderr is inherited from the parent (for debugging).
    ///
    /// # Arguments
    ///
    /// * `def` - Server definition with command and arguments
    /// * `server_name` - Server name for debug logging context
    /// * `debug` - Enable JSON-RPC debug logging to stderr
    pub async fn spawn(
        def: &McpServerDef,
        server_name: impl Into<String>,
        debug: bool,
    ) -> Result<Self, TransportError> {
        let mut cmd = Command::new(&def.command);

        // Add arguments
        cmd.args(&def.args);

        // Set environment variables
        for (key, value) in &def.env {
            cmd.env(key, value);
        }

        // Set working directory if specified
        if let Some(ref cwd) = def.cwd {
            cmd.current_dir(cwd);
        }

        // Configure stdio
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());

        // Spawn the process
        let mut child = cmd
            .spawn()
            .map_err(|e| TransportError::Spawn(e.to_string()))?;

        // Take the stdio handles
        let stdin = child
            .stdin
            .take()
            .ok_or(TransportError::StdinNotAvailable)?;
        let stdout = child
            .stdout
            .take()
            .ok_or(TransportError::StdoutNotAvailable)?;

        Ok(Self {
            child: Mutex::new(Some(child)),
            stdin: Mutex::new(Some(BufWriter::new(stdin))),
            stdout: Mutex::new(Some(BufReader::new(stdout))),
            next_id: AtomicU64::new(1),
            shutdown: AtomicBool::new(false),
            debug,
            server_name: server_name.into(),
        })
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    /// Generate the next request ID.
    fn next_request_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Check if transport is shut down, returning an error if so.
    fn require_not_shutdown(&self) -> Result<(), TransportError> {
        if self.shutdown.load(Ordering::Acquire) {
            Err(TransportError::Shutdown)
        } else {
            Ok(())
        }
    }

    /// Write a serializable message to stdin with newline delimiter.
    async fn write_message<T: Serialize>(&self, message: &T) -> Result<(), TransportError> {
        self.require_not_shutdown()?;

        let mut guard = self.stdin.lock().await;
        let stdin = guard.as_mut().ok_or(TransportError::StdinNotAvailable)?;

        let json = serde_json::to_string(message)?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        Ok(())
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /// Send a JSON-RPC request to the child process.
    pub async fn send(&self, request: &JsonRpcRequest) -> Result<(), TransportError> {
        if self.debug {
            eprintln!(
                "MCP JSON-RPC [{}] -> {}: {}",
                self.server_name,
                request.method,
                serde_json::to_string(request).unwrap_or_default()
            );
        }
        self.write_message(request).await
    }

    /// Send a JSON-RPC notification (no response expected).
    pub async fn send_notification(
        &self,
        notification: &JsonRpcNotification,
    ) -> Result<(), TransportError> {
        if self.debug {
            eprintln!(
                "MCP JSON-RPC [{}] -> {} (notification): {}",
                self.server_name,
                notification.method,
                serde_json::to_string(notification).unwrap_or_default()
            );
        }
        self.write_message(notification).await
    }

    /// Receive a JSON-RPC response from the child process.
    ///
    /// Reads one line and parses it as a JSON-RPC response.
    pub async fn receive(&self) -> Result<JsonRpcResponse, TransportError> {
        self.require_not_shutdown()?;

        let mut guard = self.stdout.lock().await;
        let stdout = guard.as_mut().ok_or(TransportError::StdoutNotAvailable)?;

        let mut line = String::new();
        let bytes_read = stdout.read_line(&mut line).await?;

        if bytes_read == 0 {
            return Err(TransportError::ProcessExited);
        }

        if self.debug {
            eprintln!("MCP JSON-RPC [{}] <- {}", self.server_name, line.trim());
        }

        serde_json::from_str(&line).map_err(|e| TransportError::Parse(e.to_string()))
    }

    /// Send a JSON-RPC request and wait for the response.
    ///
    /// This is the primary method for request-response communication.
    /// Returns an error if the response doesn't arrive within the timeout.
    ///
    /// # Arguments
    ///
    /// * `method` - The JSON-RPC method name
    /// * `params` - Optional parameters for the method
    /// * `timeout_ms` - Timeout in milliseconds
    ///
    /// # Note
    ///
    /// The current implementation assumes responses arrive in order. For more complex
    /// scenarios with out-of-order responses, a pending requests map would be needed.
    pub async fn request(
        &self,
        method: impl Into<String>,
        params: Option<serde_json::Value>,
        timeout_ms: u64,
    ) -> Result<serde_json::Value, TransportError> {
        let id = self.next_request_id();
        let request = JsonRpcRequest::new(id, method, params);

        // Send the request
        self.send(&request).await?;

        // Wait for response with timeout
        let response = timeout(Duration::from_millis(timeout_ms), self.receive())
            .await
            .map_err(|_| TransportError::Timeout(timeout_ms))??;

        // Verify response ID matches
        if response.id != id {
            return Err(TransportError::IdMismatch {
                request: id,
                response: response.id,
            });
        }

        // Extract result or error
        response.into_result().map_err(TransportError::from)
    }

    /// Gracefully shut down the transport and terminate the child process.
    ///
    /// 1. Closes stdin (signals EOF to child)
    /// 2. Waits briefly for process to exit
    /// 3. Forcefully kills if still running
    pub async fn shutdown(&self) -> Result<(), TransportError> {
        // Mark as shut down
        self.shutdown.store(true, Ordering::Release);

        // Close stdin to signal EOF
        {
            let mut guard = self.stdin.lock().await;
            if let Some(mut stdin) = guard.take() {
                // Flush any pending writes
                let _ = stdin.flush().await;
                // Drop closes the handle
            }
        }

        // Close stdout
        {
            let mut guard = self.stdout.lock().await;
            guard.take();
        }

        // Wait for process to exit, then kill if necessary
        let mut guard = self.child.lock().await;
        if let Some(mut child) = guard.take() {
            // Give process a chance to exit gracefully
            match timeout(Duration::from_millis(1000), child.wait()).await {
                Ok(Ok(_status)) => {
                    // Process exited normally
                }
                Ok(Err(e)) => {
                    // Error waiting for process
                    return Err(TransportError::Io(e));
                }
                Err(_) => {
                    // Timeout - force kill
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
            }
        }

        Ok(())
    }

    /// Check if the transport has been shut down.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }

    /// Check if the child process is still running.
    pub async fn is_running(&self) -> bool {
        if self.shutdown.load(Ordering::Acquire) {
            return false;
        }

        let mut guard = self.child.lock().await;
        if let Some(ref mut child) = *guard {
            // try_wait returns Ok(Some(_)) if exited, Ok(None) if still running
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        }
    }
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
