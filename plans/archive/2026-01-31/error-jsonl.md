# Error JSONL Implementation Plan

## Overview

Add error entries to the session JSONL file when failures occur in print mode (`-p`). Currently, failures (rate_limit, network_unreachable, etc.) write error messages to stderr and exit without recording anything to the session JSONL. Watchers in otters parse the JSONL for error fields to trigger `on_error` actions, but cannot detect errors that only appear in stderr.

This blocks otters tests: `on_error_escalate_on_network_failure`, `on_error_recover_retries_after_rate_limit`.

## Project Structure

```
crates/cli/src/
├── main.rs              # Modify: record errors before exiting
├── failure.rs           # Modify: add session-recording variant
├── output.rs            # Existing: ResultOutput::error(), ::rate_limit()
├── state/
│   ├── mod.rs           # Modify: add record_error() to StateWriter
│   └── session/
│       ├── jsonl.rs     # Add: ErrorLine type
│       └── mod.rs       # Export: error writing functions
```

## Dependencies

No new dependencies. Uses existing:
- `serde` / `serde_json` for JSONL serialization
- `chrono` for timestamps
- `uuid` for message IDs

## Implementation Phases

### Phase 1: Define Error Entry Type in JSONL

**Goal**: Add `ErrorLine` type matching the result wrapper format.

**Files**: `crates/cli/src/state/session/jsonl.rs`

```rust
/// Error entry in JSONL format for failure events.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorLine {
    #[serde(rename = "type")]
    pub line_type: &'static str,  // "result"
    pub subtype: String,           // "error"
    pub is_error: bool,            // true
    pub session_id: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>, // "rate_limit_error", "network_error", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,   // seconds for rate limit
    pub duration_ms: u64,
    pub timestamp: String,
}
```

Add writer function:
```rust
pub fn append_error_jsonl(
    path: &Path,
    session_id: &str,
    error: &str,
    error_type: Option<&str>,
    retry_after: Option<u64>,
    duration_ms: u64,
    timestamp: DateTime<Utc>,
) -> std::io::Result<()>;
```

**Verification**: Add unit tests in `jsonl_tests.rs` for ErrorLine serialization.

---

### Phase 2: Add StateWriter Error Recording

**Goal**: Add `record_error()` method to `StateWriter`.

**Files**: `crates/cli/src/state/mod.rs`

```rust
impl StateWriter {
    /// Record an error to the session JSONL file.
    ///
    /// # Arguments
    /// * `error` - Error message
    /// * `error_type` - Optional error type (e.g., "rate_limit_error")
    /// * `retry_after` - Optional retry delay in seconds (for rate limits)
    /// * `duration_ms` - Time elapsed before error
    pub fn record_error(
        &mut self,
        error: &str,
        error_type: Option<&str>,
        retry_after: Option<u64>,
        duration_ms: u64,
    ) -> std::io::Result<()> {
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let jsonl_path = self.session_jsonl_path();
        append_error_jsonl(
            &jsonl_path,
            &self.session_id,
            error,
            error_type,
            retry_after,
            duration_ms,
            Utc::now(),
        )
    }
}
```

Export from `state/session/mod.rs`: `append_error_jsonl`, `ErrorLine`.

**Verification**: Add unit tests for `record_error()` in `state/mod_tests.rs`.

---

### Phase 3: Add Failure Executor Session Recording

**Goal**: Add variant that records to session JSONL before executing failure.

**Files**: `crates/cli/src/failure.rs`

Add new method to `FailureExecutor`:
```rust
/// Execute failure with session recording.
///
/// Writes error entry to JSONL before writing to stderr and exiting.
pub async fn execute_with_session<W: Write>(
    spec: &FailureSpec,
    writer: &mut W,
    state_writer: Option<&mut StateWriter>,
) -> Result<(), std::io::Error> {
    // 1. Record to session JSONL if state_writer provided
    if let Some(sw) = state_writer {
        let (error, error_type, retry_after, duration) = match spec {
            FailureSpec::NetworkUnreachable => (
                "Network error: Connection refused".to_string(),
                Some("network_error"),
                None,
                5000u64,
            ),
            FailureSpec::RateLimit { retry_after } => (
                format!("Rate limited. Retry after {} seconds.", retry_after),
                Some("rate_limit_error"),
                Some(*retry_after),
                50u64,
            ),
            // ... other variants
        };
        sw.record_error(&error, error_type, retry_after, duration)?;
    }

    // 2. Execute original failure behavior
    Self::execute(spec, writer).await
}
```

**Verification**: Add tests in `failure_tests.rs` for session recording behavior.

---

### Phase 4: Integrate in Main.rs

**Goal**: Use session-recording failure execution in print mode.

**Files**: `crates/cli/src/main.rs`

Modify CLI failure handling (~line 104):
```rust
if let Some(ref mode) = cli.failure {
    let spec = FailureExecutor::from_mode(mode);
    let mut stderr = io::stderr();

    if let Some(ref log) = capture {
        log.record(captured_args, CapturedOutcome::Failure { ... });
    }

    // Record error to session JSONL before exiting
    let mut state_writer_guard = state_writer.as_ref().map(|w| w.write());
    FailureExecutor::execute_with_session(
        &spec,
        &mut stderr,
        state_writer_guard.as_deref_mut(),
    ).await?;
    return Ok(());
}
```

Modify scenario failure handling (~line 140):
```rust
if let Some(failure_spec) = s.get_failure(&result) {
    let mut stderr = io::stderr();

    if let Some(ref log) = capture {
        log.record(captured_args, CapturedOutcome::Failure { ... });
    }

    // Record error to session JSONL before exiting
    let mut state_writer_guard = state_writer.as_ref().map(|w| w.write());
    FailureExecutor::execute_with_session(
        failure_spec,
        &mut stderr,
        state_writer_guard.as_deref_mut(),
    ).await?;
    return Ok(());
}
```

**Note**: Move `state_writer` creation before failure checks to ensure it's available.

**Verification**: Manual testing with `--failure rate_limit` and checking JSONL output.

---

### Phase 5: Integration Tests

**Goal**: Add CLI integration tests verifying error JSONL entries.

**Files**: `crates/cli/tests/error_jsonl.rs`

```rust
#[test]
fn error_jsonl_rate_limit() {
    // Run: claudeless -p --failure rate_limit "test"
    // Assert: JSONL contains error entry with:
    //   - type: "result"
    //   - subtype: "error"
    //   - is_error: true
    //   - error_type: "rate_limit_error"
    //   - retry_after: 60
}

#[test]
fn error_jsonl_network_unreachable() {
    // Run: claudeless -p --failure network_unreachable "test"
    // Assert: JSONL contains error entry with error_type: "network_error"
}

#[test]
fn error_jsonl_scenario_failure() {
    // Run: claudeless -p --scenario <with failure rule> "matching prompt"
    // Assert: JSONL contains error entry
}
```

**Verification**: `cargo test --test error_jsonl`.

---

## Key Implementation Details

### Error Entry Format

The error entry uses the same `result` type as success responses but with `subtype: "error"`:

```json
{
  "type": "result",
  "subtype": "error",
  "isError": true,
  "sessionId": "uuid",
  "error": "Rate limited. Retry after 60 seconds.",
  "errorType": "rate_limit_error",
  "retryAfter": 60,
  "durationMs": 50,
  "timestamp": "2026-01-31T12:00:00Z"
}
```

This matches the `ResultOutput::error()` format in `output.rs` but adapted for JSONL with camelCase field names.

### StateWriter Availability

Currently `state_writer` is created after failure checks in `main.rs`. It needs to be moved earlier:

```rust
// Create state writer BEFORE failure checks
let state_writer = if !cli.no_session_persistence {
    Some(Arc::new(RwLock::new(StateWriter::new(...))?))
} else {
    None
};

// Now failures can record to session
if let Some(ref mode) = cli.failure {
    // ... can use state_writer here
}
```

### Exit Code Preservation

The `FailureExecutor::execute()` method calls `std::process::exit()` directly. The new `execute_with_session()` must:
1. Record to JSONL first
2. Then call the original `execute()` which exits

### Field Mapping

| FailureSpec | error_type | retry_after |
|-------------|------------|-------------|
| NetworkUnreachable | `network_error` | None |
| ConnectionTimeout | `timeout_error` | None |
| AuthError | `authentication_error` | None |
| RateLimit | `rate_limit_error` | Some(seconds) |
| OutOfCredits | `billing_error` | None |
| PartialResponse | `partial_response` | None |
| MalformedJson | N/A (no JSONL entry) | N/A |

Note: `MalformedJson` may not need a JSONL entry since it simulates corrupted output.

---

## Verification Plan

### Unit Tests
- [ ] `ErrorLine` serializes to expected JSON format
- [ ] `append_error_jsonl()` writes valid JSONL line
- [ ] `StateWriter::record_error()` creates file and appends entry
- [ ] `FailureExecutor::execute_with_session()` records before exiting

### Integration Tests
- [ ] `--failure rate_limit` produces JSONL with error entry
- [ ] `--failure network_unreachable` produces JSONL with error entry
- [ ] Scenario-based failures produce JSONL entries
- [ ] `--no-session-persistence` skips JSONL recording

### Manual Verification
```bash
# Test rate limit error
./target/debug/claudeless -p --failure rate_limit "test"
cat ~/.claude/projects/*/SESSION_ID.jsonl | jq .

# Expected: queue-operation line + error result line
```

### Otters Integration
- [ ] `on_error_escalate_on_network_failure` test passes
- [ ] `on_error_recover_retries_after_rate_limit` test passes
