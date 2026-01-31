# Implementation Plan: TUI JSONL Session Writing

## Overview

Add JSONL session file writing to TUI mode (interactive mode without `-p` flag). Currently, only print mode (`-p`) writes JSONL files via `StateWriter`. The TUI's `SessionManager` only tracks in-memory state and saves to `.json` files. This blocks otters integration tests that watch for JSONL state changes (e.g., `stop_reason: end_turn`).

**Goal:** When running in TUI mode, claudeless should write session JSONL to `CLAUDE_LOCAL_STATE_DIR` so external watchers can detect agent state changes.

## Project Structure

```
crates/cli/src/
├── state/
│   ├── mod.rs                    # Re-exports StateWriter
│   ├── session.rs                # SessionManager (existing)
│   └── session/
│       └── jsonl.rs              # JSONL write functions (existing)
├── tui/
│   └── app/
│       ├── state/
│       │   └── mod.rs            # TuiAppStateInner (add StateWriter)
│       ├── commands.rs           # process_prompt, start_streaming_inner
│       └── types.rs              # TuiConfig
└── main.rs                       # TUI initialization
```

## Dependencies

No new dependencies required. Uses existing:
- `StateWriter` from `crates/cli/src/state/mod.rs`
- JSONL functions from `crates/cli/src/state/session/jsonl.rs`
- `StateDirectory::resolve()` for finding the state directory

## Implementation Phases

### Phase 1: Add StateWriter to TuiAppState

**Files:** `crates/cli/src/tui/app/state/mod.rs`, `crates/cli/src/tui/app/types.rs`

Add an optional `StateWriter` to `TuiAppStateInner` and update `TuiConfig` to pass it through.

```rust
// In TuiConfig
pub struct TuiConfig {
    // ... existing fields ...
    pub state_writer: Option<Arc<RwLock<StateWriter>>>,
}

// In TuiAppStateInner
pub(super) struct TuiAppStateInner {
    // ... existing fields ...
    /// State writer for JSONL persistence
    pub state_writer: Option<Arc<RwLock<StateWriter>>>,
}
```

**Verification:** `cargo build` succeeds, no runtime changes yet.

### Phase 2: Initialize StateWriter in TUI Startup

**File:** `crates/cli/src/main.rs`

Create `StateWriter` for TUI mode similar to print mode, passing it through `TuiConfig`.

```rust
// In run_tui() function
let state_writer = if !cli.no_session_persistence {
    Some(Arc::new(RwLock::new(StateWriter::new(
        session_id.to_string(),
        &project_path,
        launch_timestamp,
        &model,
        &working_directory,
    )?)))
} else {
    None
};

let tui_config = TuiConfig {
    // ... existing fields ...
    state_writer: state_writer.clone(),
};
```

Update `TuiAppState::new()` to store the writer from config.

**Verification:** TUI starts without errors, StateWriter is initialized (but not yet used).

### Phase 3: Write JSONL on Message Submission

**File:** `crates/cli/src/tui/app/commands.rs`

Update `process_prompt()` and `start_streaming_inner()` to write JSONL entries when turns are recorded.

Key changes in `process_prompt()`:
```rust
pub(super) fn process_prompt(&self, prompt: String) {
    // ... existing prompt processing ...

    // Record user message to JSONL
    let user_uuid = if let Some(ref writer) = inner.state_writer {
        Some(writer.write().record_user_message(&prompt).ok())
    } else {
        None
    }.flatten();

    // Store user_uuid for later linking with assistant response
    inner.display.pending_user_uuid = user_uuid;

    // ... rest of processing ...
}
```

Key changes in `start_streaming_inner()`:
```rust
pub(super) fn start_streaming_inner(inner: &mut TuiAppStateInner, text: String) {
    // ... existing streaming logic ...

    // Record assistant response to JSONL
    if let (Some(ref writer), Some(ref user_uuid)) =
           (&inner.state_writer, &inner.display.pending_user_uuid) {
        let _ = writer.write().record_assistant_response(user_uuid, &text);
    }
    inner.display.pending_user_uuid = None;

    // ... rest of function ...
}
```

**Verification:** After sending a prompt in TUI mode, verify JSONL file is created at `CLAUDE_LOCAL_STATE_DIR/projects/{project}/session.jsonl`.

### Phase 4: Write stop_reason in Assistant Messages

**File:** `crates/cli/src/state/mod.rs`

Update `record_assistant_response()` to include `stop_reason: "end_turn"` in the JSONL output. This is critical for external watchers.

```rust
pub fn record_assistant_response(
    &mut self,
    parent_user_uuid: &str,
    response: &str,
) -> std::io::Result<String> {
    // ... existing code ...

    let params = AssistantMessageParams {
        // ... existing fields ...
        stop_reason: Some("end_turn"),  // Always set for completed responses
        // ...
    };
    // ...
}
```

**Verification:** Inspect JSONL output and confirm `stop_reason: "end_turn"` appears in assistant messages.

### Phase 5: Handle Tool Calls in TUI

**File:** `crates/cli/src/tui/app/commands.rs`

When tool permissions are granted and tools execute, record tool_use and tool_result messages to JSONL.

```rust
// In confirm_permission() or after tool execution
if let Some(ref writer) = inner.state_writer {
    // Record assistant tool_use message
    let content_blocks = vec![ContentBlock::ToolUse {
        id: tool_use_id.clone(),
        name: tool_name.clone(),
        input: tool_input.clone(),
    }];
    let assistant_uuid = writer.write().record_assistant_tool_use(
        &user_uuid,
        content_blocks,
    )?;

    // Record tool result
    writer.write().record_tool_result(
        &tool_use_id,
        &result_content,
        &assistant_uuid,
        serde_json::json!({"success": true}),
    )?;
}
```

**Verification:** After tool execution in TUI, JSONL contains tool_use and tool_result messages.

### Phase 6: Integration Test Verification

**File:** Create `crates/cli/tests/tui_jsonl.rs`

Add integration tests that verify JSONL writing in TUI mode works correctly for the otters integration tests.

```rust
#[test]
fn test_tui_writes_jsonl_on_prompt() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_var("CLAUDE_LOCAL_STATE_DIR", temp_dir.path());

    // Simulate TUI interaction
    // ...

    // Verify JSONL file exists and has correct structure
    let jsonl_path = temp_dir.path()
        .join("projects")
        .join("-tmp-test-project")
        .join("session.jsonl");
    assert!(jsonl_path.exists());

    // Verify content has stop_reason: end_turn
    let content = std::fs::read_to_string(&jsonl_path).unwrap();
    assert!(content.contains("\"stop_reason\":\"end_turn\""));
}

#[test]
fn test_tui_jsonl_matches_print_mode_format() {
    // Verify TUI JSONL output matches the format expected by otters
    // Same fields, same structure as print mode
}
```

**Verification:** `cargo test --all` passes, including new integration tests.

## Key Implementation Details

### JSONL vs JSON Persistence

The existing `SessionManager` saves sessions as `.json` files for TUI state recovery. The new `StateWriter` integration writes `.jsonl` files for external watchers. Both can coexist:

- `.json` files: Used by TUI for session resume/recovery (existing)
- `.jsonl` files: Used by external tools (otters) to watch for state changes (new)

### Message UUID Linking

JSONL messages are linked via `parentUuid` fields:
1. User message: `parentUuid: null` (or previous assistant uuid for multi-turn)
2. Assistant message: `parentUuid: <user-uuid>`
3. Tool result: `parentUuid: <assistant-uuid>`

The `pending_user_uuid` field in `DisplayState` tracks the current user message UUID for linking.

### stop_reason Values

External watchers look for `stop_reason` in assistant messages:
- `"end_turn"` - Normal completion, agent is idle
- `"tool_use"` - Agent is waiting for tool results (not idle)
- `null` - Response still streaming

For TUI mode, completed responses should always have `stop_reason: "end_turn"`.

### Error Handling

JSONL write errors should be logged but not fail the TUI operation. Users should still be able to interact even if persistence fails:

```rust
if let Err(e) = writer.write().record_user_message(&prompt) {
    tracing::warn!("Failed to write JSONL: {}", e);
}
```

## Verification Plan

1. **Unit Tests**
   - `StateWriter` methods produce valid JSONL
   - `stop_reason` is correctly set for completed responses

2. **Integration Tests**
   - TUI mode creates JSONL file on first prompt
   - JSONL format matches print mode output
   - Multiple turns produce multiple JSONL entries
   - Tool calls produce tool_use and tool_result entries

3. **Manual Testing**
   ```bash
   # Start TUI with explicit state dir
   export CLAUDE_LOCAL_STATE_DIR=/tmp/claudeless-test
   cargo run

   # In another terminal, watch for JSONL changes
   tail -f /tmp/claudeless-test/projects/*/*.jsonl

   # Send a prompt in TUI, observe JSONL output
   ```

4. **Otters Integration**
   - Run `agent_spawn_interactive_idle_completes` test
   - Run `runbook_with_agent_config_idle_completes` test
   - Both should pass with TUI JSONL support

5. **Pre-commit Checks**
   - `make check` passes
   - `cargo clippy --all-targets --all-features -- -D warnings` clean
   - `cargo test --all` passes
