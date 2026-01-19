# Fix JSON Output Format

## Overview

Fix the JSON output format to match real Claude CLI v2.1.12 behavior. The main issues are:

1. **Field name mismatch**: `total_cost_usd` should be `cost_usd`
2. **Empty `usage` and `modelUsage` fields**: Need simulated values
3. **Tests are ignored**: Un-ignore tests once format is fixed

Real Claude CLI uses a result wrapper format for `--output-format json`:
```json
{
  "type": "result",
  "subtype": "success",
  "cost_usd": 0.003,
  "is_error": false,
  "duration_ms": 1234,
  "result": "Hello!",
  "session_id": "...",
  "usage": { "input_tokens": 100, ... },
  "modelUsage": { ... }
}
```

## Project Structure

```
crates/cli/
├── src/
│   ├── output.rs               # UPDATE: Fix ResultOutput fields
│   ├── output_tests.rs         # UPDATE: Add tests for new fields
│   ├── config.rs               # UPDATE: Extend UsageSpec if needed
│   └── validation/
│       └── output_samples.rs   # REFERENCE: Golden samples
├── tests/
│   └── smoke_test.rs           # UPDATE: Un-ignore passing tests
└── docs/
    └── LIMITATIONS.md          # UPDATE: Mark issues resolved
```

## Dependencies

No new dependencies required. Uses existing `serde` and `serde_json`.

## Implementation Phases

### Phase 1: Fix ResultOutput Field Names

**Goal**: Rename `total_cost_usd` to `cost_usd` to match real Claude.

**Changes to `src/output.rs`**:

```rust
// Line 17 - rename field
#[serde(rename = "cost_usd")]  // ADD this attribute
pub total_cost_usd: f64,       // OR rename to cost_usd
```

The cleaner approach is to rename the field entirely:

```rust
/// Result wrapper for JSON output matching real Claude's `--output-format json`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultOutput {
    #[serde(rename = "type")]
    pub output_type: String,
    pub subtype: String,
    pub cost_usd: f64,           // RENAME from total_cost_usd
    pub is_error: bool,
    pub duration_ms: u64,
    pub duration_api_ms: u64,
    pub num_turns: u32,
    // ... rest unchanged
}
```

**Update all constructors**: `success()`, `error()`, `rate_limit()` to use `cost_usd`.

**Verification**:
- `cargo test -p claudeless output`
- Output JSON contains `cost_usd` not `total_cost_usd`

---

### Phase 2: Add Extended Usage Types

**Goal**: Create proper types for `usage` and `modelUsage` fields instead of empty JSON objects.

**Changes to `src/output.rs`**:

```rust
/// Detailed usage statistics for result output
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
    /// Cost breakdown for this request
    pub cost_usd: f64,
}

impl ResultUsage {
    /// Create usage from basic token counts
    pub fn from_tokens(input: u32, output: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cost_usd: estimate_cost(input, output),
        }
    }
}

/// Per-model usage breakdown
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelUsage {
    #[serde(flatten)]
    pub models: std::collections::HashMap<String, ResultUsage>,
}

impl Default for ModelUsage {
    fn default() -> Self {
        Self {
            models: std::collections::HashMap::new(),
        }
    }
}

/// Estimate cost based on token counts (simulated)
fn estimate_cost(input_tokens: u32, output_tokens: u32) -> f64 {
    // Approximate Claude Sonnet pricing: $3/M input, $15/M output
    let input_cost = (input_tokens as f64) * 0.000003;
    let output_cost = (output_tokens as f64) * 0.000015;
    input_cost + output_cost
}
```

**Update `ResultOutput`**:

```rust
pub struct ResultOutput {
    // ... existing fields ...

    #[serde(rename = "modelUsage")]
    pub model_usage: ModelUsage,  // CHANGE from serde_json::Value
    pub usage: ResultUsage,       // CHANGE from serde_json::Value
    // ...
}
```

**Verification**:
- `cargo build -p claudeless`
- Types compile without errors

---

### Phase 3: Populate Usage in Constructors

**Goal**: Generate realistic simulated usage values in `ResultOutput` constructors.

**Changes to `src/output.rs`**:

```rust
impl ResultOutput {
    /// Create a success result with usage based on response
    pub fn success(result: String, session_id: String, duration_ms: u64) -> Self {
        let output_tokens = estimate_tokens(&result);
        let input_tokens = 100; // Default simulated input tokens
        let usage = ResultUsage::from_tokens(input_tokens, output_tokens);

        let mut model_usage = ModelUsage::default();
        // Add usage for the default model
        model_usage.models.insert(
            "claude-sonnet-4-20250514".to_string(),
            usage.clone(),
        );

        Self {
            output_type: "result".to_string(),
            subtype: "success".to_string(),
            cost_usd: usage.cost_usd,
            is_error: false,
            duration_ms,
            duration_api_ms: duration_ms.saturating_sub(50),
            num_turns: 1,
            result: Some(result),
            error: None,
            session_id,
            uuid: uuid_stub(),
            retry_after: None,
            model_usage,
            usage,
            permission_denials: vec![],
        }
    }

    /// Create a success result with custom usage
    pub fn success_with_usage(
        result: String,
        session_id: String,
        duration_ms: u64,
        input_tokens: u32,
        output_tokens: u32,
        model: &str,
    ) -> Self {
        let usage = ResultUsage::from_tokens(input_tokens, output_tokens);

        let mut model_usage = ModelUsage::default();
        model_usage.models.insert(model.to_string(), usage.clone());

        Self {
            output_type: "result".to_string(),
            subtype: "success".to_string(),
            cost_usd: usage.cost_usd,
            is_error: false,
            duration_ms,
            duration_api_ms: duration_ms.saturating_sub(50),
            num_turns: 1,
            result: Some(result),
            error: None,
            session_id,
            uuid: uuid_stub(),
            retry_after: None,
            model_usage,
            usage,
            permission_denials: vec![],
        }
    }

    /// Create an error result
    pub fn error(error: String, session_id: String, duration_ms: u64) -> Self {
        Self {
            output_type: "result".to_string(),
            subtype: "error".to_string(),
            cost_usd: 0.0,
            is_error: true,
            duration_ms,
            duration_api_ms: duration_ms.saturating_sub(10),
            num_turns: 0,
            result: None,
            error: Some(error),
            session_id,
            uuid: uuid_stub(),
            retry_after: None,
            model_usage: ModelUsage::default(),
            usage: ResultUsage::from_tokens(0, 0),
            permission_denials: vec![],
        }
    }

    /// Create a rate limit error result
    pub fn rate_limit(retry_after: u64, session_id: String) -> Self {
        Self {
            output_type: "result".to_string(),
            subtype: "error".to_string(),
            cost_usd: 0.0,
            is_error: true,
            duration_ms: 50,
            duration_api_ms: 50,
            num_turns: 0,
            result: None,
            error: Some(format!(
                "Rate limited. Retry after {} seconds.",
                retry_after
            )),
            session_id,
            uuid: uuid_stub(),
            retry_after: Some(retry_after),
            model_usage: ModelUsage::default(),
            usage: ResultUsage::from_tokens(0, 0),
            permission_denials: vec![],
        }
    }
}
```

**Verification**:
- `cargo test -p claudeless output`
- ResultOutput serializes with non-empty `usage` and `modelUsage`

---

### Phase 4: Update write_real_json to Use Response Usage

**Goal**: When a `ResponseSpec::Detailed` includes usage info, use it in the result.

**Changes to `src/output.rs`**:

```rust
/// Write JSON in real Claude's result wrapper format
fn write_real_json(
    &mut self,
    response: &ResponseSpec,
    session_id: &str,
) -> std::io::Result<()> {
    let (text, usage_spec) = match response {
        ResponseSpec::Simple(s) => (s.clone(), None),
        ResponseSpec::Detailed { text, usage, .. } => (text.clone(), usage.clone()),
    };

    let result = if let Some(usage) = usage_spec {
        ResultOutput::success_with_usage(
            text,
            session_id.to_string(),
            1000,
            usage.input_tokens,
            usage.output_tokens,
            &self.model,
        )
    } else {
        ResultOutput::success(text, session_id.to_string(), 1000)
    };

    self.write_result(&result)
}
```

**Verification**:
- Scenario with custom usage reflects in JSON output

---

### Phase 5: Update Unit Tests

**Goal**: Add/update tests to verify the new format.

**Changes to `src/output_tests.rs`**:

```rust
#[test]
fn test_result_output_has_usage_fields() {
    let result = ResultOutput::success(
        "Hello!".to_string(),
        "session-123".to_string(),
        1000,
    );

    let json = serde_json::to_string(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Verify usage is populated (not empty object)
    assert!(parsed["usage"].is_object());
    assert!(parsed["usage"]["input_tokens"].is_number());
    assert!(parsed["usage"]["output_tokens"].is_number());
    assert!(parsed["usage"]["cost_usd"].is_number());

    // Verify modelUsage is populated
    assert!(parsed["modelUsage"].is_object());
}

#[test]
fn test_result_output_uses_cost_usd() {
    let result = ResultOutput::success(
        "Hello!".to_string(),
        "session-123".to_string(),
        1000,
    );

    let json = serde_json::to_string(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Field should be cost_usd not total_cost_usd
    assert!(parsed["cost_usd"].is_number());
    assert!(parsed.get("total_cost_usd").is_none());
}

#[test]
fn test_result_output_with_custom_usage() {
    let result = ResultOutput::success_with_usage(
        "Test".to_string(),
        "session-123".to_string(),
        500,
        50,  // input tokens
        25,  // output tokens
        "claude-test",
    );

    let json = serde_json::to_string(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["usage"]["input_tokens"], 50);
    assert_eq!(parsed["usage"]["output_tokens"], 25);
    assert!(parsed["modelUsage"]["claude-test"].is_object());
}
```

**Verification**:
- `cargo test -p claudeless output` passes

---

### Phase 6: Un-ignore Smoke Tests

**Goal**: Remove `#[ignore]` from tests that now pass.

**Changes to `tests/smoke_test.rs`**:

```rust
// Line 141-143: Remove #[ignore]
/// Behavior observed with: claude --version 2.1.12 (Claude Code)
#[test]
// #[ignore]  <-- REMOVE THIS LINE
fn test_json_output_uses_result_wrapper_format() {
    // ...
}

// Line 197-199: Remove #[ignore]
#[test]
// #[ignore]  <-- REMOVE THIS LINE
fn test_json_output_result_contains_response_text() {
    // ...
}
```

**Update test assertions** if needed (e.g., if test checks for `total_cost_usd`):

```rust
// Line 189-191: Update to check cost_usd
assert!(
    parsed["cost_usd"].is_number(),  // CHANGE from total_cost_usd
    "Real Claude includes cost_usd"
);
```

**Verification**:
- `cargo test -p claudeless smoke` passes all tests
- `cargo test -- --ignored` shows fewer ignored tests

---

### Phase 7: Update Documentation

**Goal**: Mark issues as resolved in LIMITATIONS.md.

**Changes to `docs/LIMITATIONS.md`**:

Remove from "Known TODOs" section (lines 58-61):
```markdown
- [ ] **JSON output format**: `usage` and `modelUsage` fields empty
  - `test_json_output_uses_result_wrapper_format`
  - `test_json_output_result_contains_response_text`
```

Update "Output Format Divergences" table (lines 113-123):
```markdown
### JSON Output (`--output-format json`)

| Field | Real Claude | Claudeless |
|-------|-------------|------------|
| `usage` | Rich cache/server metrics | Simulated token-based usage |
| `modelUsage` | Per-model detailed metrics | Simulated per-model usage |
| `cost_usd` | Actual API cost | Simulated (~$3/M in, $15/M out) |
| `duration_ms` | Actual timing | Simulated |
```

**Verification**:
- Documentation reflects actual behavior

---

## Key Implementation Details

### Cost Estimation Formula

Claude Sonnet 3.5 approximate pricing:
- Input: $3.00 per million tokens
- Output: $15.00 per million tokens

```rust
fn estimate_cost(input_tokens: u32, output_tokens: u32) -> f64 {
    (input_tokens as f64) * 0.000003 + (output_tokens as f64) * 0.000015
}
```

### Token Estimation Formula

Existing `estimate_tokens()` uses ~4 characters per token:

```rust
fn estimate_tokens(text: &str) -> u32 {
    (text.len() / 4).max(1) as u32
}
```

### Backward Compatibility

The `ResultOutput` struct is internal to claudeless. No external API changes are needed. Scenarios using `usage` in `ResponseSpec::Detailed` will now see their values reflected in the output.

---

## Verification Plan

### Unit Tests

Run after each phase:
```bash
cargo test -p claudeless output
cargo test -p claudeless config
```

### Integration Tests

After Phase 6:
```bash
cargo test -p claudeless --test smoke_test
cargo test -- --ignored  # Should show fewer ignored tests
```

### Manual Verification

```bash
# JSON output should show cost_usd and populated usage
claudeless --output-format json -p "hello" | jq '.cost_usd, .usage, .modelUsage'

# Stream-JSON should end with result containing usage
claudeless --output-format stream-json -p "hello" | tail -1 | jq '.'
```

### Full CI Check

```bash
make check
```

---

## Files Changed

| File | Action |
|------|--------|
| `crates/cli/src/output.rs` | Edit - Fix field names, add usage types |
| `crates/cli/src/output_tests.rs` | Edit - Add new tests |
| `crates/cli/tests/smoke_test.rs` | Edit - Un-ignore tests, fix assertions |
| `docs/LIMITATIONS.md` | Edit - Update status |

## Estimated Scope

- ~150 lines of code changes
- ~50 lines of test additions
- ~10 lines of documentation updates
