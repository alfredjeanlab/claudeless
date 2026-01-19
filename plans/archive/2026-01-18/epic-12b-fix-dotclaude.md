# Epic 12b: Fix ~/.claude Directory Format

## Overview

Fix ~/.claude state directory format divergences from real Claude CLI.

## Prerequisites

1. `crates/cli/scripts/capture-state.sh` implemented [DONE]
2. `crates/cli/scripts/compare-state.sh` implemented [DONE]
3. Fixtures captured in `tests/fixtures/dotclaude/2.1.12/` [DONE]
   - session.jsonl, session.normalized.jsonl
   - todo.json, todo.normalized.json
   - plan.md
   - sessions-index.json, sessions-index.normalized.json
4. Failing tests added with `FIXME: epic-05x-fix-dotclaude` comments [DONE]

### Implemented Tests (Ignored Until Fixtures Captured)

- `test_sessions_index_matches_fixture` - Compare sessions-index.json structure
- `test_session_jsonl_matches_fixture` - Compare session.jsonl message types
- `test_todo_json_matches_fixture` - Compare todo.json structure
- `test_plan_md_matches_fixture_structure` - Compare plan.md structure

## Architecture

### State Normalization Utility

```
crates/cli/src/testing/
└── normalize_state.rs    # Normalize state files for comparison
```

**Key functions:**
- `normalize_sessions_index(json: Value) -> Value` - Normalize sessions-index.json
- `normalize_session_jsonl(content: &str) -> String` - Normalize session JSONL
- `normalize_todo_json(json: Value) -> Value` - Normalize todo files
- Handles: UUIDs, timestamps, mtime values, absolute paths

### Fixture Comparison for State

```
crates/cli/tests/common/
└── state_fixtures.rs     # Load and compare state fixtures
```

**Key functions:**
- `load_state_fixture(version: &str, name: &str) -> String`
- `assert_state_matches_fixture(actual: &str, version: &str, name: &str)`

## Required Fixes

_Identified from captured fixtures (v2.1.12) vs current simulator implementation._

### 1. `queue-operation` message type (conditional)

**File:** `src/state/session.rs`
**Fixture:** `tests/fixtures/dotclaude/2.1.12/session.jsonl` (line 1)

**Analysis:** `queue-operation` has multiple uses in real Claude:

1. **Session start with `-p` flag**: First line is `{"type":"queue-operation","operation":"dequeue",...}`
   - Only for non-interactive `-p` (print) mode
   - Contains: `type`, `operation`, `timestamp`, `sessionId`

2. **Interactive input queuing** (mid-session): `enqueue`, `popAll`, `remove` operations
   - Contains additional `content` field with queued message
   - Used when user types while Claude is still responding

3. **Interactive sessions** start with `file-history-snapshot` or `summary` instead

**Fix for simulator:**
- For `-p` mode: emit `queue-operation` with `operation: "dequeue"` as first line
- For interactive mode: start with `file-history-snapshot` or `user` message
- Input queuing operations are optional (nice-to-have for TUI fidelity)

### 2. User message missing fields

**File:** `src/state/session.rs:206-221` (`UserMessageLine`)
**Fixture:** `tests/fixtures/dotclaude/2.1.12/session.normalized.jsonl` (line 2)
**Issue:** Real Claude user messages include additional fields not present in simulator
**Fix:** Add missing fields to `UserMessageLine`:
- `parentUuid: Option<String>` - null for first message, UUID for tool results
- `isSidechain: bool` - always false for normal sessions
- `userType: String` - e.g., "external"
- `version: String` - CLI version, e.g., "2.1.12"
- `gitBranch: String` - current git branch or empty

### 3. Assistant message missing fields

**File:** `src/state/session.rs:257-275` (`AssistantMessageLine`)
**Fixture:** `tests/fixtures/dotclaude/2.1.12/session.normalized.jsonl` (line 3)
**Issue:** Real Claude assistant messages include fields not present in simulator
**Fix:** Add missing fields to `AssistantMessageLine`:
- `isSidechain: bool`
- `userType: String`
- `cwd: String`
- `version: String`
- `gitBranch: String`

### 4. Assistant message.* envelope fields

**File:** `src/state/session.rs:246-254` (`AssistantMessage`)
**Fixture:** `tests/fixtures/dotclaude/2.1.12/session.normalized.jsonl`
**Issue:** Real Claude includes full API response envelope in `message`
**Fix:** Add missing fields to `AssistantMessage`:
- `id: String` - message ID, e.g., "msg_..."
- `type: String` - always "message"
- `stop_reason: Option<String>`
- `stop_sequence: Option<String>`
- `usage: Usage` - object with token counts

### 5. Tool result messages have special fields

**File:** `src/state/session.rs`
**Fixture:** `tests/fixtures/dotclaude/2.1.12/session.normalized.jsonl` (line 5)
**Issue:** Tool result user messages include metadata about the tool execution
**Fix:** Create `ToolResultMessageLine` with:
- `toolUseResult: serde_json::Value` - tool-specific result data
- `sourceToolAssistantUUID: String` - UUID of assistant message containing tool_use
- `message.content: Vec<ContentBlock>` - array format (not string)

### 6. Sessions index not captured [DONE]

**File:** `scripts/capture-state.sh`
**Issue:** Capture script doesn't copy sessions-index.json
**Fix:** Updated capture script with:
- Debug output showing project directory contents
- Fallback search for sessions-index.json in ~/.claude/projects/
- Note that sessions-index.json may only be created on session resume or after multiple sessions

### 7. Todo JSON format matches (no fix needed)

**File:** `src/state/todos.rs:43-52` (`ClaudeTodoItem`)
**Fixture:** `tests/fixtures/dotclaude/2.1.12/todo.json`
**Status:** Already correct - `save_claude_format()` produces matching output

## Verification

- [ ] All `FIXME: epic-05x-fix-dotclaude` tests pass
- [ ] `scripts/compare-state.sh` exits 0
- [ ] `make check` passes
