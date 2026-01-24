# Refactor Output Plan

**Root Feature:** `cl-f708`

## Overview

Reduce `output.rs` from 924 lines to under 750 by:
1. Extracting repetitive patterns into helper methods
2. Moving "Real Claude Format Types" to a sibling module

Current: 924 lines → Target: <750 lines

## Project Structure

```
crates/cli/src/
├── output.rs            # OutputWriter + core types (~650 lines after refactor)
├── output_events.rs     # Real Claude format event types (~220 lines)
└── output_tests.rs      # Tests (unchanged)
```

## Dependencies

No new dependencies. Internal refactor only.

## Implementation Phases

### Phase 1: DRY Up ResponseSpec Extraction

**Milestone**: Reduce repetitive match patterns

The same `ResponseSpec` match appears 4 times (lines 584, 596, 638, 817):

```rust
let (text, usage) = match response {
    ResponseSpec::Simple(s) => (s.clone(), None),
    ResponseSpec::Detailed { text, usage, .. } => (text.clone(), usage.clone()),
};
```

**Changes**:

1. Add helper method to `ResponseSpec` in `config.rs`:
   ```rust
   impl ResponseSpec {
       /// Extract text and optional usage from a response.
       pub fn text_and_usage(&self) -> (String, Option<UsageSpec>) {
           match self {
               ResponseSpec::Simple(s) => (s.clone(), None),
               ResponseSpec::Detailed { text, usage, .. } => (text.clone(), usage.clone()),
           }
       }
   }
   ```

2. Replace all 4 occurrences in `output.rs`:
   ```rust
   let (text, usage) = response.text_and_usage();
   ```

**Saves**: ~12 lines

### Phase 2: Extract Serde Error Helper

**Milestone**: Remove repeated error mapping boilerplate

This pattern appears 5+ times:

```rust
serde_json::to_string(&value)
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
```

**Changes**:

Add private helper function:

```rust
fn to_json<T: serde::Serialize>(value: &T) -> std::io::Result<String> {
    serde_json::to_string(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
```

Replace usages:

```rust
// Before
let json = serde_json::to_string(event)
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

// After
let json = to_json(event)?;
```

**Saves**: ~10 lines

### Phase 3: Simplify ResultOutput Constructors

**Milestone**: Reduce duplication in success/error/rate_limit constructors

The `ResultOutput` constructors (lines 75-186) share significant boilerplate.

**Changes**:

Add a base constructor and use struct update syntax:

```rust
impl ResultOutput {
    /// Create a base result with common defaults.
    fn base(session_id: String) -> Self {
        Self {
            output_type: "result".to_string(),
            subtype: "success".to_string(),
            cost_usd: 0.0,
            is_error: false,
            duration_ms: 0,
            duration_api_ms: 0,
            num_turns: 0,
            result: None,
            error: None,
            session_id,
            uuid: uuid_stub(),
            retry_after: None,
            model_usage: ModelUsage::default(),
            usage: ResultUsage::from_tokens(0, 0),
            permission_denials: vec![],
        }
    }

    pub fn success(result: String, session_id: String, duration_ms: u64) -> Self {
        let output_tokens = estimate_tokens(&result);
        let input_tokens = 100;
        let usage = ResultUsage::from_tokens(input_tokens, output_tokens);
        let mut model_usage = ModelUsage::default();
        model_usage.models.insert(
            "claude-opus-4-5-20251101".to_string(),
            ResultUsage::from_tokens(input_tokens, output_tokens),
        );

        Self {
            cost_usd: usage.cost_usd,
            duration_ms,
            duration_api_ms: duration_ms.saturating_sub(50),
            num_turns: 1,
            result: Some(result),
            model_usage,
            usage,
            ..Self::base(session_id)
        }
    }

    // Similar for error(), rate_limit(), success_with_usage()
}
```

**Saves**: ~25 lines

### Phase 4: Extract Real Claude Format Types to Sibling Module

**Milestone**: Split file under 750 lines

Move lines 338-551 (Real Claude Format Types section) to `output_events.rs`.

**Types to move** (all with constructors):
- `SystemInitEvent`
- `AssistantEvent`
- `AssistantMessageContent`
- `CondensedAssistantEvent`
- `CondensedMessage`
- `ExtendedUsage`
- `ContentBlockStartEvent`
- `ContentBlockDeltaEvent`
- `ContentBlockStopEvent`

**Changes**:

1. Create `crates/cli/src/output_events.rs`:
   ```rust
   //! Event types matching real Claude CLI output format.

   use serde::{Deserialize, Serialize};

   /// Generate a deterministic UUID-like stub for testing.
   fn uuid_stub() -> String {
       "01234567890abcdef".to_string()
   }

   // ... moved types ...
   ```

2. Update `output.rs`:
   ```rust
   mod output_events;
   pub use output_events::*;
   ```

3. Add `#[cfg(test)] #[path = "output_events_tests.rs"] mod tests;` to `output_events.rs` if tests are needed later.

**Saves**: ~213 lines from `output.rs`

### Phase 5: Verify and Clean Up

**Milestone**: All checks pass

1. Run `cargo fmt --all`
2. Run `cargo clippy --all-targets --all-features -- -D warnings`
3. Run `cargo test --all`
4. Run `quench check --fix` to verify cloc passes

## Key Implementation Details

### Module Structure

After refactoring:

| File | Contents | Est. Lines |
|------|----------|------------|
| `output.rs` | `ResultUsage`, `ModelUsage`, `ResultOutput`, `JsonResponse`, `ContentBlock`, `Usage`, `StreamEvent`, `ToolResultBlock`, `OutputWriter` | ~650 |
| `output_events.rs` | Real Claude format event types (`SystemInitEvent`, `AssistantEvent`, etc.) | ~220 |
| `output_tests.rs` | All tests (unchanged) | 535 |

### ResponseSpec Helper Location

Add to `config.rs` because:
- `ResponseSpec` is defined there
- Avoids orphan impl rules
- Natural place for response-related methods

### Re-exports

`output.rs` will re-export all types from `output_events.rs`:

```rust
mod output_events;
pub use output_events::{
    AssistantEvent, AssistantMessageContent, CondensedAssistantEvent,
    CondensedMessage, ContentBlockDeltaEvent, ContentBlockStartEvent,
    ContentBlockStopEvent, ExtendedUsage, SystemInitEvent,
};
```

This maintains backward compatibility - callers continue to `use crate::output::*`.

### Preserving uuid_stub

The `uuid_stub()` function is needed in both modules:
- `output.rs` uses it in `ResultOutput` constructors
- `output_events.rs` uses it in `CondensedAssistantEvent::new`

Options:
1. Define in both files (duplicate, but private and trivial)
2. Make it `pub(crate)` in one file and import in the other

Prefer option 1 for simplicity since it's a 3-line function.

## Verification Plan

### Phase Gate Checks

After each phase:
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

### Final Verification

```bash
make check
```

Expected output:
```
PASS: escapes, agents, docs, cloc
```

### Line Count Verification

After all phases:
```bash
wc -l crates/cli/src/output.rs
# Should be < 750

wc -l crates/cli/src/output_events.rs
# Should be < 750
```
