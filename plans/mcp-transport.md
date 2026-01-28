# Implementation Plan: JSON-RPC Stdio Transport Layer

## Overview

Build the low-level transport layer for communicating with MCP servers over stdio. This includes spawning child processes, buffered stdin/stdout management, JSON-RPC message serialization, timeout handling, and graceful shutdown. This is Phase 1 of the broader MCP implementation plan.

## Project Structure

```
crates/cli/src/mcp/
├── mod.rs              # Update exports
├── transport.rs        # NEW - JSON-RPC stdio transport
├── transport_tests.rs  # NEW - Unit tests
├── config.rs           # Existing - McpServerDef used for spawn config
├── config_tests.rs     # Existing
├── server.rs           # Existing (will use transport in future phase)
├── server_tests.rs     # Existing
├── tools.rs            # Existing
└── tools_tests.rs      # Existing
```

## Dependencies

No new dependencies required. Uses existing crates:

- `tokio` - async runtime with process spawning (`features = ["process", "io-util", "time", "sync"]`)
- `serde`, `serde_json` - JSON-RPC serialization
- `thiserror` - error types

## Implementation Phases

### Phase 1: JSON-RPC Message Types

**Goal:** Define the core JSON-RPC 2.0 message structures.

**Implementation:**

```rust
// transport.rs

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,  // Always "2.0"
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
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
    pub jsonrpc: String,
    pub id: u64,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Returns the result if successful, error otherwise.
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
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 notification (no id, no response expected).
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: &'static str,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            method: method.into(),
            params,
        }
    }
}
```

**Verification:**
- Test serialization produces valid JSON-RPC format
- Test deserialization handles both success and error responses
- Test edge cases: null result, missing optional fields

---

### Phase 2: Transport Error Type

**Goal:** Define comprehensive error handling for transport operations.

**Implementation:**

```rust
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
    IdMismatch { request: u64, response: u64 },

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
```

---

### Phase 3: StdioTransport Core Structure

**Goal:** Create the transport struct that manages the child process and I/O streams.

**Implementation:**

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

/// Stdio transport for JSON-RPC communication with a child process.
///
/// Manages spawning a child process, writing JSON-RPC requests to stdin,
/// and reading JSON-RPC responses from stdout. All messages are newline-delimited.
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
    shutdown: std::sync::atomic::AtomicBool,
}
```

**Key design decisions:**

1. **Mutex-wrapped Option for streams** - Allows taking ownership during shutdown while protecting concurrent access.

2. **AtomicU64 for request IDs** - Lock-free ID generation for concurrent requests.

3. **Newline-delimited JSON** - Standard format for stdio JSON-RPC (each message is one line).

4. **Buffered I/O** - BufReader/BufWriter for efficient reading line-by-line and batched writes.

---

### Phase 4: Process Spawning

**Goal:** Implement `StdioTransport::spawn()` to start the child process.

**Implementation:**

```rust
use super::config::McpServerDef;
use tokio::process::Command;

impl StdioTransport {
    /// Spawn a new child process and create a transport for communication.
    ///
    /// The process is spawned with stdin/stdout piped for JSON-RPC communication.
    /// Stderr is inherited from the parent (for debugging).
    pub async fn spawn(def: &McpServerDef) -> Result<Self, TransportError> {
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
        let mut child = cmd.spawn().map_err(|e| TransportError::Spawn(e.to_string()))?;

        // Take the stdio handles
        let stdin = child.stdin.take().ok_or(TransportError::StdinNotAvailable)?;
        let stdout = child.stdout.take().ok_or(TransportError::StdoutNotAvailable)?;

        Ok(Self {
            child: Mutex::new(Some(child)),
            stdin: Mutex::new(Some(BufWriter::new(stdin))),
            stdout: Mutex::new(Some(BufReader::new(stdout))),
            next_id: AtomicU64::new(1),
            shutdown: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// Generate the next request ID.
    fn next_request_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}
```

**Verification:**
- Test spawning a simple echo process
- Test spawning with environment variables
- Test spawning with working directory
- Test error when command doesn't exist

---

### Phase 5: Send and Receive Operations

**Goal:** Implement low-level send/receive for JSON-RPC messages.

**Implementation:**

```rust
impl StdioTransport {
    /// Send a JSON-RPC request to the child process.
    ///
    /// Writes the serialized request followed by a newline.
    pub async fn send(&self, request: &JsonRpcRequest) -> Result<(), TransportError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(TransportError::Shutdown);
        }

        let mut guard = self.stdin.lock().await;
        let stdin = guard.as_mut().ok_or(TransportError::StdinNotAvailable)?;

        // Serialize and write
        let json = serde_json::to_string(request)?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        Ok(())
    }

    /// Send a JSON-RPC notification (no response expected).
    pub async fn send_notification(&self, notification: &JsonRpcNotification) -> Result<(), TransportError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(TransportError::Shutdown);
        }

        let mut guard = self.stdin.lock().await;
        let stdin = guard.as_mut().ok_or(TransportError::StdinNotAvailable)?;

        let json = serde_json::to_string(notification)?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        Ok(())
    }

    /// Receive a JSON-RPC response from the child process.
    ///
    /// Reads one line and parses it as a JSON-RPC response.
    pub async fn receive(&self) -> Result<JsonRpcResponse, TransportError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(TransportError::Shutdown);
        }

        let mut guard = self.stdout.lock().await;
        let stdout = guard.as_mut().ok_or(TransportError::StdoutNotAvailable)?;

        let mut line = String::new();
        let bytes_read = stdout.read_line(&mut line).await?;

        if bytes_read == 0 {
            return Err(TransportError::ProcessExited);
        }

        serde_json::from_str(&line).map_err(|e| TransportError::Parse(e.to_string()))
    }
}
```

**Verification:**
- Test send serializes correctly with newline
- Test receive parses valid JSON-RPC response
- Test receive returns ProcessExited on EOF
- Test operations fail after shutdown

---

### Phase 6: Request-Response with Timeout

**Goal:** Implement `request()` method that sends a request and waits for response with timeout.

**Implementation:**

```rust
use std::time::Duration;
use tokio::time::timeout;

impl StdioTransport {
    /// Send a JSON-RPC request and wait for the response.
    ///
    /// This is the primary method for request-response communication.
    /// Returns an error if the response doesn't arrive within the timeout.
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
        let response = timeout(
            Duration::from_millis(timeout_ms),
            self.receive()
        ).await.map_err(|_| TransportError::Timeout(timeout_ms))??;

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

    /// Send a JSON-RPC request and wait for the response, using the default timeout.
    pub async fn request_with_default_timeout(
        &self,
        method: impl Into<String>,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, TransportError> {
        self.request(method, params, 30000).await
    }
}
```

**Note on ID matching:** The current implementation assumes responses arrive in order. For more complex scenarios with out-of-order responses, a pending requests map would be needed. This is deferred to a future phase if needed.

**Verification:**
- Test successful request-response flow
- Test timeout triggers after specified duration
- Test ID mismatch detection
- Test error response handling

---

### Phase 7: Graceful Shutdown

**Goal:** Implement clean shutdown of the transport and child process.

**Implementation:**

```rust
impl StdioTransport {
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
```

**Verification:**
- Test shutdown closes streams
- Test shutdown waits for graceful exit
- Test shutdown force-kills after timeout
- Test operations fail after shutdown

---

## Key Implementation Details

### Thread Safety

The transport is designed for safe concurrent access:

- `Mutex<Option<...>>` for stdin/stdout allows exclusive access during operations and clean shutdown
- `AtomicU64` for request IDs avoids locking for ID generation
- `AtomicBool` for shutdown flag allows lock-free checks

### Error Recovery

The transport doesn't attempt automatic recovery. If an error occurs:

1. The caller should inspect the error type
2. For recoverable errors (timeout), retry may be appropriate
3. For fatal errors (ProcessExited, Shutdown), create a new transport

### Message Format

Uses newline-delimited JSON (NDJSON):
- Each message is a single line of JSON
- Lines are terminated with `\n`
- This is the standard format for stdio JSON-RPC

### Future Considerations

For Phase 2 (MCP Protocol), the transport will be wrapped by an `McpClient` that:
- Manages the MCP protocol lifecycle (initialize, etc.)
- Handles MCP-specific message types
- Provides higher-level error handling

## Verification Plan

### Unit Tests (transport_tests.rs)

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

mod json_rpc_types {
    use super::*;

    #[test]
    fn request_serializes_correctly() {
        let req = JsonRpcRequest::new(1, "test", Some(serde_json::json!({"key": "value"})));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""id":1"#));
        assert!(json.contains(r#""method":"test""#));
    }

    #[test]
    fn request_without_params_omits_field() {
        let req = JsonRpcRequest::new(1, "test", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("params"));
    }

    #[test]
    fn response_deserializes_success() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, 1);
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn response_deserializes_error() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32600);
        assert_eq!(err.message, "Invalid Request");
    }

    #[test]
    fn notification_serializes_correctly() {
        let notif = JsonRpcNotification::new("notify", None);
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""method":"notify""#));
        assert!(!json.contains("id"));
    }
}

mod spawn {
    use super::*;

    #[tokio::test]
    async fn spawns_echo_process() {
        let def = McpServerDef {
            command: "cat".to_string(),
            ..Default::default()
        };
        let transport = StdioTransport::spawn(&def).await.unwrap();
        assert!(transport.is_running().await);
        transport.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn spawn_fails_for_nonexistent_command() {
        let def = McpServerDef {
            command: "nonexistent_command_12345".to_string(),
            ..Default::default()
        };
        let result = StdioTransport::spawn(&def).await;
        assert!(matches!(result, Err(TransportError::Spawn(_))));
    }

    #[tokio::test]
    async fn spawn_with_env_vars() {
        let mut def = McpServerDef {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "echo $TEST_VAR".to_string()],
            ..Default::default()
        };
        def.env.insert("TEST_VAR".to_string(), "hello".to_string());

        let transport = StdioTransport::spawn(&def).await.unwrap();
        // Process should run and we can verify env var was set
        transport.shutdown().await.unwrap();
    }
}

mod send_receive {
    use super::*;

    #[tokio::test]
    async fn send_and_receive_roundtrip() {
        // Use a simple JSON echo server (Python one-liner)
        let def = McpServerDef {
            command: "python3".to_string(),
            args: vec![
                "-c".to_string(),
                r#"
import sys, json
for line in sys.stdin:
    req = json.loads(line)
    resp = {"jsonrpc": "2.0", "id": req["id"], "result": req.get("params", {})}
    print(json.dumps(resp), flush=True)
"#.to_string(),
            ],
            ..Default::default()
        };

        let transport = StdioTransport::spawn(&def).await.unwrap();

        let result = transport.request(
            "echo",
            Some(serde_json::json!({"test": "value"})),
            5000,
        ).await.unwrap();

        assert_eq!(result["test"], "value");

        transport.shutdown().await.unwrap();
    }
}

mod timeout {
    use super::*;

    #[tokio::test]
    async fn request_times_out() {
        // Process that doesn't respond
        let def = McpServerDef {
            command: "sleep".to_string(),
            args: vec!["10".to_string()],
            ..Default::default()
        };

        let transport = StdioTransport::spawn(&def).await.unwrap();

        let result = transport.request("test", None, 100).await;
        assert!(matches!(result, Err(TransportError::Timeout(100))));

        transport.shutdown().await.unwrap();
    }
}

mod shutdown {
    use super::*;

    #[tokio::test]
    async fn shutdown_marks_transport_closed() {
        let def = McpServerDef {
            command: "cat".to_string(),
            ..Default::default()
        };
        let transport = StdioTransport::spawn(&def).await.unwrap();

        transport.shutdown().await.unwrap();

        assert!(transport.is_shutdown());
        assert!(!transport.is_running().await);
    }

    #[tokio::test]
    async fn operations_fail_after_shutdown() {
        let def = McpServerDef {
            command: "cat".to_string(),
            ..Default::default()
        };
        let transport = StdioTransport::spawn(&def).await.unwrap();
        transport.shutdown().await.unwrap();

        let result = transport.request("test", None, 1000).await;
        assert!(matches!(result, Err(TransportError::Shutdown)));
    }
}
```

### Integration Testing

Create a simple echo MCP server fixture for integration tests:

```python
#!/usr/bin/env python3
# tests/fixtures/echo_jsonrpc.py
"""Simple JSON-RPC echo server for transport testing."""
import json
import sys

def main():
    for line in sys.stdin:
        try:
            req = json.loads(line.strip())
            resp = {
                "jsonrpc": "2.0",
                "id": req.get("id"),
                "result": req.get("params", {})
            }
            print(json.dumps(resp), flush=True)
        except json.JSONDecodeError:
            err = {
                "jsonrpc": "2.0",
                "id": None,
                "error": {"code": -32700, "message": "Parse error"}
            }
            print(json.dumps(err), flush=True)

if __name__ == "__main__":
    main()
```

### Final Verification

```bash
# Run all tests
cargo test --package claudeless mcp::transport

# Run with output to verify
cargo test --package claudeless mcp::transport -- --nocapture

# Full project check
make check
```
