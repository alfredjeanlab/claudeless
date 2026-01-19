# Epic 12a: Fix CLI Output Format

**Status:** Ready for validation

## Overview

Fix CLI output format divergences identified by running comparison scripts against real Claude CLI.

## Prerequisites (COMPLETED)

1. ✅ `crates/cli/scripts/compare-cli.sh` implemented
2. ✅ Fixtures captured in `tests/fixtures/cli/v2.1.12/`
3. ✅ Failing tests added with `FIXME: epic-05x-fix-cli` comments in `tests/cli_fixtures.rs`

## Architecture

### Normalization Utility

Shared normalization logic for comparing outputs:

```
crates/cli/src/testing/
└── normalize.rs    # Normalize JSON for comparison (replace UUIDs, timestamps)
```

**Key functions:**
- `normalize_json(value: Value) -> Value` - Replace dynamic fields with placeholders
- `normalize_jsonl(lines: &str) -> String` - Normalize each line of JSONL

### Fixture Comparison Test Helper

```
crates/cli/tests/common/
└── fixtures.rs     # Load and compare against fixtures
```

**Key functions:**
- `load_fixture(version: &str, name: &str) -> String`
- `assert_matches_fixture(actual: &str, version: &str, name: &str)`

## Required Fixes

Discovered by running `cargo test --test cli_fixtures -- --ignored`:

### 1. JSON output uses wrong format

**File:** `src/main.rs:174`
**Issue:** Uses `write_response()` (raw API format) instead of `write_real_response()` (result wrapper format)
**Expected:** `{"type": "result", "subtype": "success", "cost_usd": ..., "is_error": false, ...}`
**Actual:** `{"type": "message", "role": "assistant", "content": [...], ...}`
**Fix:** Change `writer.write_response()` to `writer.write_real_response()` in main.rs

### 2. Stream-JSON missing system init event

**File:** `src/output.rs` (write_response path, not write_real_stream_json)
**Issue:** First event is `message_start` instead of `system` with `subtype: init`
**Expected first event:** `{"type": "system", "subtype": "init", "cwd": "...", "session_id": "...", "tools": [...]}`
**Actual first event:** `{"type": "message_start", ...}`
**Fix:** Use `write_real_response()` path which includes system init

### 3. Stream-JSON missing final result event

**File:** `src/output.rs` (write_response path)
**Issue:** Last event is `message_stop` instead of `result`
**Expected last event:** `{"type": "result", "subtype": "success", ...}`
**Actual last event:** `{"type": "message_stop"}`
**Fix:** Use `write_real_response()` path which includes result wrapper

### 4. Stream-JSON event types differ

**Expected sequence:**
```
system (init) -> assistant (message_start) -> content_block_start ->
content_block_delta (multiple) -> content_block_stop ->
assistant (message_delta) -> assistant (message_stop) -> result (success)
```

**Actual sequence:**
```
message_start -> content_block_start -> content_block_delta (multiple) ->
content_block_stop -> message_delta -> message_stop
```

**Issues:**
- Missing `system` init event at start
- Missing `assistant` subtype wrapper on events
- Missing `result` event at end
- Fewer content_block_delta events (chunking differs)

**Root cause:** All issues stem from using wrong output path in main.rs

## Verification

- [ ] All `FIXME: epic-05x-fix-cli` tests pass
- [ ] `scripts/compare-cli.sh` exits 0
- [ ] `make check` passes
