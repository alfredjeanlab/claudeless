# Epic 12c: Wire Up Tool Result Messages and Improve Fixture Tests

**Goal:** Complete tool result message recording and make fixture-based tests more robust

## Overview

Currently, `ToolResultMessageLine` structs are defined but never written to session JSONL. The fixture tests only check the first message of each type, missing systematic format errors. This plan addresses both gaps.

## Part 1: Wire Up Tool Result Messages

### Problem

When claudeless executes tools, it writes results to stdout but doesn't record them to the ~/.claude session JSONL file. Real Claude writes:

```
Line 1: queue-operation (if -p mode)
Line 2: user (prompt)
Line 3: assistant (with tool_use content blocks)
Line 4: user (tool_result) ← MISSING
Line 5: assistant (final response)
```

### Files to Modify

1. **`crates/cli/src/state/session.rs`**
   - Add `ToolResultParams` struct
   - Add `append_tool_result_jsonl()` function
   - Add `append_assistant_with_tool_use()` for assistant messages with tool calls

2. **`crates/cli/src/state/mod.rs`**
   - Add `StateWriter::record_tool_result()` method
   - Add `StateWriter::record_assistant_tool_use()` method
   - Track `last_assistant_uuid` for tool result parent linking

3. **`crates/cli/src/main.rs`**
   - After tool execution, call `state_writer.record_tool_result()`
   - Before tool execution, record assistant message with tool_use blocks
   - Pass tool_use_id and result content to state writer

### Implementation Details

#### A. New structs in session.rs

```rust
/// Parameters for writing tool result JSONL lines.
pub struct ToolResultParams<'a> {
    pub session_id: &'a str,
    pub result_uuid: &'a str,
    pub parent_uuid: &'a str,      // The assistant message UUID
    pub tool_use_id: &'a str,
    pub result_content: &'a str,
    pub cwd: &'a str,
    pub version: &'a str,
    pub git_branch: &'a str,
    pub tool_use_result: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}
```

#### B. StateWriter additions in mod.rs

```rust
impl StateWriter {
    /// Record an assistant message with tool_use blocks.
    /// Returns the assistant UUID for linking tool results.
    pub fn record_assistant_tool_use(
        &mut self,
        parent_user_uuid: &str,
        tool_calls: &[ToolCall],
        model: &str,
    ) -> std::io::Result<String>;

    /// Record a tool result message.
    pub fn record_tool_result(
        &self,
        tool_use_id: &str,
        result_content: &str,
        assistant_uuid: &str,
        tool_use_result: serde_json::Value,
    ) -> std::io::Result<()>;
}
```

#### C. Main.rs flow changes

Current flow (lines 196-246 in main.rs):
```rust
// Record turn BEFORE tools (writes user + assistant pair)
state_writer.write().record_turn(&prompt, &response_text)?;

// Execute tools (writes to stdout only, not JSONL)
for (i, call) in tool_calls.iter().enumerate() {
    let tool_use_id = format!("toolu_{:08x}", i);
    let result = executor.execute(call, &tool_use_id, &ctx);
    writer.write_tool_result(&result)?;  // stdout only
}
```

New flow:
```rust
// 1. Record user message (returns user_uuid)
let user_uuid = state_writer.write().record_user_message(&prompt)?;

// 2. If no tool calls, just record simple assistant response
if tool_calls.is_empty() {
    state_writer.write().record_assistant_response(&user_uuid, &response_text)?;
} else {
    // 3. Record assistant message with tool_use blocks (returns assistant_uuid)
    let assistant_uuid = state_writer.write()
        .record_assistant_tool_use(&user_uuid, &tool_calls)?;

    // 4. Execute tools and record results
    for (i, call) in tool_calls.iter().enumerate() {
        let tool_use_id = format!("toolu_{:08x}", i);
        let result = executor.execute(call, &tool_use_id, &ctx);
        writer.write_tool_result(&result)?;  // stdout

        // Record tool result to JSONL
        state_writer.read().record_tool_result(
            &tool_use_id,
            result.text().unwrap_or(""),
            &assistant_uuid,
            serde_json::json!({}),  // tool_use_result (empty for most tools)
        )?;
    }

    // 5. Record final assistant response
    state_writer.write().record_assistant_response(&user_uuid, &response_text)?;
}
```

Note: The fixture shows Real Claude writes multiple assistant messages when tools are involved:
- Line 3: assistant (text: "I'll create a todo list...")
- Line 4: assistant (tool_use: TodoWrite)
- Line 5: user (tool_result)
- Line 6: assistant (final text)

The simulator's response structure differs - it combines text and tool_use in one response.
For now, we'll write: user → assistant (with tool_use) → tool_result → assistant (final).
This matches the semantic flow even if message count differs slightly.

## Part 2: Improve Fixture-Based Tests

### Current Test Gaps

1. Only checks first message of each type
2. Doesn't validate nested field normalization (message.id, requestId, tool_use_id)
3. No message count validation
4. No message order validation
5. Extra fields in actual output not caught
6. Content array structure not validated

### Files to Modify

1. **`crates/cli/tests/dot_claude_projects.rs`**
   - Enhance `normalize_json()` to handle nested IDs
   - Add `validate_message_sequence()` helper
   - Add `validate_all_messages_of_type()` helper
   - Update `test_session_jsonl_matches_fixture`

2. **`crates/cli/tests/fixtures/dotclaude/v2.1.12/session.jsonl`**
   - Update fixture if needed after implementation

### Test Improvements

#### A. Enhanced normalization

```rust
fn normalize_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        String(s) => {
            // Existing: UUID, timestamp, temp path
            // Add: message ID pattern (msg_*)
            if s.starts_with("msg_") { return "<MESSAGE_ID>".into(); }
            // Add: request ID pattern (req_*)
            if s.starts_with("req_") { return "<REQUEST_ID>".into(); }
            // Add: tool use ID pattern (toolu_*)
            if s.starts_with("toolu_") { return "<TOOL_USE_ID>".into(); }
            // ... existing patterns
        }
        // ... rest unchanged
    }
}
```

#### B. New validation helpers

```rust
/// Validate all messages of a given type have the expected structure.
fn validate_all_messages_of_type(
    actual: &[Value],
    expected: &[Value],
    msg_type: &str,
) {
    let actual_msgs: Vec<_> = actual.iter()
        .filter(|m| m["type"].as_str() == Some(msg_type))
        .collect();
    let expected_msgs: Vec<_> = expected.iter()
        .filter(|m| m["type"].as_str() == Some(msg_type))
        .collect();

    // Check count matches
    assert_eq!(actual_msgs.len(), expected_msgs.len(),
        "Message type '{}' count mismatch", msg_type);

    // Check each message structure
    for (i, (actual, expected)) in actual_msgs.iter().zip(&expected_msgs).enumerate() {
        compare_message_structure(actual, expected, &format!("{}[{}]", msg_type, i));
    }
}

/// Validate message sequence order matches fixture.
fn validate_message_sequence(actual: &[Value], expected: &[Value]) {
    let actual_types: Vec<_> = actual.iter()
        .filter_map(|m| m["type"].as_str())
        .collect();
    let expected_types: Vec<_> = expected.iter()
        .filter_map(|m| m["type"].as_str())
        .collect();

    assert_eq!(actual_types, expected_types,
        "Message sequence mismatch");
}
```

#### C. Updated test

```rust
#[test]
fn test_session_jsonl_matches_fixture() {
    // ... setup ...

    // Normalize both
    let actual_normalized: Vec<_> = actual_lines.iter()
        .map(normalize_json)
        .collect();
    let expected_normalized: Vec<_> = expected_lines.iter()
        .map(normalize_json)
        .collect();

    // 1. Validate message sequence order
    validate_message_sequence(&actual_normalized, &expected_normalized);

    // 2. Validate all messages of each type
    for msg_type in ["queue-operation", "user", "assistant"] {
        validate_all_messages_of_type(&actual_normalized, &expected_normalized, msg_type);
    }

    // 3. Validate no extra fields (bidirectional check)
    for (actual, expected) in actual_normalized.iter().zip(&expected_normalized) {
        compare_keys_bidirectional(actual, expected);
    }
}
```

## Part 3: Handle Edge Cases

### Tool-specific result data

Different tools need different `toolUseResult` structures:

| Tool | toolUseResult Structure |
|------|------------------------|
| TodoWrite | `{ oldTodos: [], newTodos: [...] }` |
| Read | `{ path: "...", lines: N }` |
| Bash | `{ command: "...", exitCode: N }` |
| Other | `{}` or tool-specific |

The `ToolExecutionResult` needs a new field or method to provide this:

```rust
impl ToolExecutionResult {
    /// Get tool-specific result data for JSONL recording.
    pub fn tool_use_result(&self) -> serde_json::Value {
        // Default empty object, tools can override
        serde_json::json!({})
    }
}
```

### Stateful tools (TodoWrite, ExitPlanMode)

These already have access to StateWriter. They can populate `toolUseResult` with before/after state.

## Verification

1. Run `cargo test --all` - all tests pass
2. Run `make check` - full CI passes
3. Manual verification:
   ```bash
   # Run claudeless with tools
   echo '{"prompt":"Create a todo"}' | cargo run -p claudeless -- \
     --scenario tests/fixtures/scenarios/todo-write.yaml \
     -p --output-format json

   # Check generated JSONL
   cat ~/.claude/projects/*/SESSION_ID.jsonl | jq -c '.type'
   # Should show: queue-operation, user, assistant (with tool_use), user (tool_result), assistant
   ```
4. Run `scripts/compare-state.sh` against real Claude output

## Task Breakdown

### Part 1: Tool Result Recording

1. [ ] Split `record_turn()` into `record_user_message()` and `record_assistant_response()`
2. [ ] Add `record_assistant_tool_use()` for assistant messages with tool_use blocks
3. [ ] Add `ToolResultParams` and `append_tool_result_jsonl()` to session.rs
4. [ ] Add `record_tool_result()` to StateWriter
5. [ ] Update main.rs flow: user → assistant (tool_use) → tool_result → assistant (final)
6. [ ] Ensure backward compatibility for simple turns (no tools)

### Part 2: Test Improvements

7. [ ] Enhance `normalize_json()` with msg_*, req_*, toolu_* patterns
8. [ ] Add `validate_message_sequence()` helper for order checking
9. [ ] Add `validate_all_messages_of_type()` for comprehensive structure checks
10. [ ] Update `test_session_jsonl_matches_fixture` with new validations
11. [ ] Update fixture to match new simulator output (if needed)

### Part 3: Verification

12. [ ] Run `cargo check` after each major change
13. [ ] Run `make check` at the end
14. [ ] Manual test with a scenario that uses tools
