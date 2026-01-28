# Implementation Plan: MCP Tech Debt Cleanup (Phases 1-3)

## Overview

Reduce duplication (DRY), improve conciseness, establish consistent error handling patterns, remove dead code, and ensure idiomatic Rust in the MCP transport/protocol/client modules. This cleanup follows the initial implementation of Phases 1-3.

**Target Files:**
- `crates/cli/src/mcp/transport.rs` (470 lines)
- `crates/cli/src/mcp/protocol.rs` (261 lines)
- `crates/cli/src/mcp/client.rs` (310 lines)

## Project Structure

```
crates/cli/src/mcp/
├── transport.rs        # JSON-RPC stdio transport (Phase 1)
├── transport_tests.rs  # Transport unit tests
├── protocol.rs         # MCP protocol types (Phase 2)
├── protocol_tests.rs   # Protocol unit tests
├── client.rs           # MCP client lifecycle (Phase 3)
├── client_tests.rs     # Client unit tests
└── mod.rs              # Module exports
```

## Dependencies

No new dependencies. Cleanup only.

## Implementation Phases

### Phase 1: Client DRY Cleanup

**Goal:** Eliminate duplication between `call_tool()` and `call_tool_with_timeout()`.

**File:** `crates/cli/src/mcp/client.rs`

**Issue:** Lines 207-263 contain nearly identical implementations differing only in the timeout source.

**Before:**
```rust
pub async fn call_tool(
    &self,
    name: &str,
    arguments: serde_json::Value,
) -> Result<ToolCallResult, ClientError> {
    if !self.initialized {
        return Err(ClientError::NotInitialized);
    }
    let params = ToolCallParams { name: name.to_string(), arguments: Some(arguments) };
    let params_json = serde_json::to_value(&params)
        .map_err(|e| ClientError::InvalidResponse(e.to_string()))?;
    let result = self.transport.request("tools/call", Some(params_json), self.timeout_ms).await?;
    let tool_result: ToolCallResult = serde_json::from_value(result)
        .map_err(|e| ClientError::InvalidResponse(e.to_string()))?;
    Ok(tool_result)
}

pub async fn call_tool_with_timeout(
    &self,
    name: &str,
    arguments: serde_json::Value,
    timeout_ms: u64,
) -> Result<ToolCallResult, ClientError> {
    // ... identical except for timeout_ms parameter
}
```

**After:**
```rust
/// Execute a tool call using the default client timeout.
pub async fn call_tool(
    &self,
    name: &str,
    arguments: serde_json::Value,
) -> Result<ToolCallResult, ClientError> {
    self.call_tool_with_timeout(name, arguments, self.timeout_ms).await
}

/// Execute a tool call with a custom timeout.
pub async fn call_tool_with_timeout(
    &self,
    name: &str,
    arguments: serde_json::Value,
    timeout_ms: u64,
) -> Result<ToolCallResult, ClientError> {
    self.require_initialized()?;

    let params = ToolCallParams {
        name: name.to_string(),
        arguments: Some(arguments),
    };
    let params_json = self.serialize_params(&params)?;
    let result = self.transport.request("tools/call", Some(params_json), timeout_ms).await?;
    self.deserialize_response(result)
}
```

**Verification:**
```bash
cargo test -p claudeless mcp::client
```

---

### Phase 2: Client Helper Methods

**Goal:** Extract repeated serialization/deserialization patterns into helpers.

**File:** `crates/cli/src/mcp/client.rs`

**Issue:** The pattern `serde_json::to_value(&x).map_err(...)` appears 3 times; `serde_json::from_value(x).map_err(...)` appears 4 times.

**Add private helpers:**
```rust
impl McpClient {
    /// Check that the client is initialized, returning an error if not.
    fn require_initialized(&self) -> Result<(), ClientError> {
        if self.initialized {
            Ok(())
        } else {
            Err(ClientError::NotInitialized)
        }
    }

    /// Serialize params to JSON, mapping errors consistently.
    fn serialize_params<T: Serialize>(&self, params: &T) -> Result<serde_json::Value, ClientError> {
        serde_json::to_value(params).map_err(|e| ClientError::InvalidResponse(e.to_string()))
    }

    /// Deserialize a JSON response, mapping errors consistently.
    fn deserialize_response<T: DeserializeOwned>(&self, value: serde_json::Value) -> Result<T, ClientError> {
        serde_json::from_value(value).map_err(|e| ClientError::InvalidResponse(e.to_string()))
    }
}
```

**Update call sites in:**
- `initialize()` (lines 131-141)
- `list_tools()` (lines 181-182)
- `call_tool_with_timeout()` (from Phase 1)

**Note:** The `serialize_params` helper takes `&self` for consistency with other methods even though it doesn't use self. This allows future extension if needed (e.g., logging). Alternatively, make it a standalone function if the team prefers.

**Verification:**
```bash
cargo test -p claudeless mcp::client
```

---

### Phase 3: Transport Shutdown Check Extraction

**Goal:** Extract repeated shutdown check pattern.

**File:** `crates/cli/src/mcp/transport.rs`

**Issue:** Lines 288-290, 309-311, and 328-330 repeat the same shutdown check:
```rust
if self.shutdown.load(Ordering::Acquire) {
    return Err(TransportError::Shutdown);
}
```

**Add helper:**
```rust
impl StdioTransport {
    /// Check if transport is shut down, returning an error if so.
    fn require_not_shutdown(&self) -> Result<(), TransportError> {
        if self.shutdown.load(Ordering::Acquire) {
            Err(TransportError::Shutdown)
        } else {
            Ok(())
        }
    }
}
```

**Update:**
- `send()` line 288
- `send_notification()` line 309
- `receive()` line 328

**Verification:**
```bash
cargo test -p claudeless mcp::transport
```

---

### Phase 4: Transport Send Consolidation

**Goal:** Reduce duplication between `send()` and `send_notification()`.

**File:** `crates/cli/src/mcp/transport.rs`

**Issue:** Both methods (lines 284-322) share identical logic for acquiring stdin lock, writing JSON, and flushing.

**Before:**
```rust
pub async fn send(&self, request: &JsonRpcRequest) -> Result<(), TransportError> {
    if self.shutdown.load(Ordering::Acquire) { return Err(TransportError::Shutdown); }
    let mut guard = self.stdin.lock().await;
    let stdin = guard.as_mut().ok_or(TransportError::StdinNotAvailable)?;
    let json = serde_json::to_string(request)?;
    stdin.write_all(json.as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;
    Ok(())
}

pub async fn send_notification(&self, notification: &JsonRpcNotification) -> Result<(), TransportError> {
    // ... identical structure
}
```

**After:**
```rust
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

/// Send a JSON-RPC request to the child process.
pub async fn send(&self, request: &JsonRpcRequest) -> Result<(), TransportError> {
    self.write_message(request).await
}

/// Send a JSON-RPC notification (no response expected).
pub async fn send_notification(&self, notification: &JsonRpcNotification) -> Result<(), TransportError> {
    self.write_message(notification).await
}
```

**Verification:**
```bash
cargo test -p claudeless mcp::transport
```

---

### Phase 5: Dead Code Audit

**Goal:** Identify and remove unused code.

**File:** `crates/cli/src/mcp/transport.rs`

**Candidate:** `request_with_default_timeout()` (lines 390-396) - verify if used anywhere.

```rust
/// Send a JSON-RPC request and wait for the response, using the default timeout.
pub async fn request_with_default_timeout(
    &self,
    method: impl Into<String>,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, TransportError> {
    self.request(method, params, 30000).await
}
```

**Action:** Run grep to check usage:
```bash
rg "request_with_default_timeout" crates/
```

If only used in tests, consider:
1. Moving to test module, or
2. Marking with `#[cfg(test)]`, or
3. Removing if tests don't need it

**Also check:**
- `ClientError::ToolNotFound` - verify if used (appears unused in client.rs)
- `ClientError::ToolError` - verify if used

**Verification:**
```bash
cargo clippy --all-targets --all-features -- -D warnings -W dead_code
```

---

### Phase 6: Naming and Final Polish

**Goal:** Address naming clarity and minor idiom improvements.

**Items:**

1. **Field/method name collision in transport.rs:**
   The `shutdown` field shadows the `shutdown()` method name. While valid Rust, it's slightly confusing.

   **Option A (rename field):**
   ```rust
   pub struct StdioTransport {
       // ...
       is_shutdown: AtomicBool,  // was: shutdown
   }
   ```

   **Option B (keep as-is):** The collision is common in Rust and doesn't cause bugs. Document the pattern.

   **Recommendation:** Keep as-is unless team prefers renaming. The field is private and usage is clear.

2. **Remove unnecessary `.clone()` in client.rs line 146:**
   ```rust
   // Before
   init_result.protocol_version.clone()
   // After - move instead of clone since we're done with init_result
   init_result.protocol_version
   ```

3. **Simplify server_info access in `initialize()`:**
   ```rust
   // Line 157 - current (defensive but awkward)
   self.server_info.as_ref().ok_or(ClientError::NotInitialized)

   // Better - we just set it, unwrap is safe. Add comment explaining why.
   // SAFETY: server_info was set to Some on line 150
   Ok(self.server_info.as_ref().expect("set above"))
   ```

   Or even cleaner, restructure to return the value directly:
   ```rust
   self.server_info = Some(init_result.server_info);
   self.initialized = true;
   self.send_initialized_notification().await?;
   Ok(self.server_info.as_ref().expect("just set"))
   ```

**Verification:**
```bash
make check
```

---

## Key Implementation Details

### Error Handling Pattern

All modules use `thiserror` with consistent patterns:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    /// Wrap lower-level errors with #[from]
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    /// Context-rich errors with named fields
    #[error("response id {response} doesn't match request id {request}")]
    IdMismatch { request: u64, response: u64 },

    /// Simple errors with String context
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}
```

### Helper Method Pattern

Private helpers that check preconditions return `Result<(), Error>`:

```rust
fn require_initialized(&self) -> Result<(), ClientError> {
    if self.initialized { Ok(()) } else { Err(ClientError::NotInitialized) }
}

// Usage
self.require_initialized()?;
```

### Summary of Changes

| File | Lines Before | Est. Lines After | Change |
|------|-------------|-----------------|--------|
| client.rs | 310 | ~280 | -30 (~10%) |
| transport.rs | 470 | ~450 | -20 (~4%) |
| protocol.rs | 261 | 261 | 0 |

**Total reduction:** ~50 lines while improving clarity.

---

## Verification Plan

### After Each Phase

```bash
cargo test -p claudeless mcp::$MODULE
```

### Final Verification

```bash
# Full test suite
cargo test --all

# Lint checks
cargo clippy --all-targets --all-features -- -D warnings

# Format check
cargo fmt --all -- --check

# Full project check
make check
```

### Regression Checklist

- [ ] All existing tests pass
- [ ] No new clippy warnings
- [ ] No dead code warnings
- [ ] Integration tests with Python echo server pass
- [ ] `cargo doc` builds without warnings
