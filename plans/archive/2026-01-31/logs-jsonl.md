# Implementation Plan: Session JSONL Compatibility for Log Extraction

## Overview

Update claudeless session JSONL output to include fields required by otters' `log_entry::parse_entries_from`. Currently claudeless writes session entries that are missing:
1. `stop_reason: "end_turn"` for detecting turn completion
2. `type: "result"` records for extracting bash exit codes

This blocks the otters test: `agent_logs_written_to_pipeline_step_structure`.

## Project Structure

```
crates/cli/src/state/
├── session/
│   └── jsonl.rs      # JSONL types and writing functions (primary changes)
├── mod.rs            # StateWriter facade (minor changes)
└── ...

crates/cli/src/
├── main.rs           # Entry point, session orchestration
└── tools/            # Tool execution (exit codes originate here)
```

## Dependencies

No new dependencies required. Uses existing:
- `serde`, `serde_json` for serialization
- `chrono` for timestamps

## Implementation Phases

### Phase 1: Add `stop_reason` Support

**Goal:** Enable TurnComplete extraction by setting `stop_reason: "end_turn"` on final assistant messages.

**Changes in `jsonl.rs`:**

1. The `AssistantMessageParams` already has `stop_reason: Option<&'a str>` - no change needed there.

2. The writing functions already propagate `stop_reason` - no change needed.

**Changes in `mod.rs` (StateWriter):**

1. Add `record_assistant_response_final()` method that sets `stop_reason: Some("end_turn")`:

```rust
/// Record a final assistant response (end of turn).
pub fn record_assistant_response_final(
    &mut self,
    parent_user_uuid: &str,
    response: &str,
) -> std::io::Result<String> {
    // Same as record_assistant_response but with stop_reason: Some("end_turn")
}
```

2. Alternatively, add a `is_final: bool` parameter to existing methods.

**Verification:** Unit test that final assistant messages include `"stop_reason":"end_turn"`.

---

### Phase 2: Add `type: "result"` Record

**Goal:** Enable bash exit code extraction by writing a `type: "result"` record after tool execution.

**Changes in `jsonl.rs`:**

1. Add new struct for result records:

```rust
/// Tool result record for log extraction.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultLine {
    #[serde(rename = "type")]
    pub line_type: &'static str,  // "result"
    pub tool_use_id: String,
    pub content: String,          // Simple string content for exit code parsing
    pub timestamp: String,
}
```

2. Add writing function:

```rust
/// Append a result record to a JSONL file.
pub fn append_result_jsonl(
    path: &Path,
    tool_use_id: &str,
    content: &str,
    timestamp: DateTime<Utc>,
) -> std::io::Result<()> {
    let mut file = open_append(path)?;
    let line = ResultLine {
        line_type: "result",
        tool_use_id: tool_use_id.to_string(),
        content: content.to_string(),
        timestamp: timestamp.to_rfc3339(),
    };
    writeln!(file, "{}", serde_json::to_string(&line)?)?;
    Ok(())
}
```

**Changes in `mod.rs` (StateWriter):**

1. Update `record_tool_result()` to also write a result record:

```rust
pub fn record_tool_result(
    &mut self,
    tool_use_id: &str,
    result_content: &str,
    assistant_uuid: &str,
    tool_use_result: serde_json::Value,
) -> std::io::Result<String> {
    // ... existing code to write user-type message ...

    // Also write result-type record for log extraction
    append_result_jsonl(&jsonl_path, tool_use_id, result_content, Utc::now())?;

    // ... rest of existing code ...
}
```

**Verification:** Unit test that tool results produce both user and result records.

---

### Phase 3: Exit Code Formatting

**Goal:** Ensure bash exit codes are in a format otters can parse.

Otters extracts exit codes via `extract_exit_code()` which looks for patterns like:
- `"exit code: 0"`
- `"Exit code: 1"`

**Changes in tools/bash.rs (or equivalent):**

1. Ensure bash tool results include the exit code pattern:

```rust
// Format: "output...\n\nExit code: 0"
format!("{}\n\nExit code: {}", output, exit_code)
```

**Verification:** Unit test that bash results contain parseable exit code pattern.

---

### Phase 4: Integration and End-to-End Test

**Goal:** Verify the complete flow works with otters.

**Changes:**

1. Add integration test in claudeless that verifies JSONL output matches expected format.

2. Remove `#[ignore]` from otters test `agent_logs_written_to_pipeline_step_structure`.

**Test scenario:**
```toml
[[responses]]
pattern = { type = "any" }
[responses.response]
text = "Running command."
[[responses.response.tool_calls]]
tool = "Bash"
input = { command = "echo hello" }
```

**Expected JSONL output:**
```jsonl
{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"echo hello"}}],"stop_reason":"tool_use"},...}
{"type":"user",...}
{"type":"result","toolUseId":"...","content":"hello\n\nExit code: 0","timestamp":"..."}
{"type":"assistant","message":{"content":[{"type":"text","text":"Done."}],"stop_reason":"end_turn"},...}
```

---

### Phase 5: Update Existing Fixtures

**Goal:** Update test fixtures to match new format.

**Files to update:**
- `crates/cli/tests/fixtures/dotclaude/v2.1.12/*/session.jsonl`

Add `stop_reason` and result records where appropriate.

---

## Key Implementation Details

### Field Requirements Summary

| Record Type | Required Fields | Purpose |
|-------------|-----------------|---------|
| `assistant` | `message.content[].{type,name,input}` | Tool call extraction |
| `assistant` | `message.stop_reason: "end_turn"` | Turn complete detection |
| `assistant` | `message.usage.output_tokens` | Token counting |
| `result` | `type: "result"`, `content` | Exit code extraction |
| `user` | `timestamp` | Duration calculation |

### Exit Code Pattern

Otters uses this regex-like pattern:
```
"exit code:" followed by optional whitespace and integer
```

The content string should include this pattern for bash commands.

### Dual Record Strategy

Tool results will produce two JSONL records:
1. `type: "user"` with nested content (for Claude API compatibility)
2. `type: "result"` with simple content (for log extraction)

This maintains backward compatibility while enabling log extraction.

## Verification Plan

1. **Unit tests in `jsonl_tests.rs`:**
   - `result_line_serialization` - verify result record format
   - `assistant_stop_reason_end_turn` - verify stop_reason field
   - `exit_code_in_content` - verify exit code pattern

2. **Unit tests in `mod_tests.rs`:**
   - `record_tool_result_writes_result_record` - verify dual records
   - `record_assistant_response_final_sets_stop_reason` - verify end_turn

3. **Integration test:**
   - Run claudeless with bash scenario
   - Parse resulting JSONL with otters' `parse_entries_from`
   - Verify BashCommand with exit_code, TurnComplete extracted

4. **Otters test:**
   - Remove `#[ignore]` from `agent_logs_written_to_pipeline_step_structure`
   - Run full test suite
