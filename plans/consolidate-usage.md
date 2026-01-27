# Plan: Consolidate Token Usage Types

## Problem

Four separate types represent token usage with no shared abstraction:

| Type | Location | Purpose |
|------|----------|---------|
| `UsageSpec` | `config.rs:248` | Scenario config token counts |
| `Usage` | `output.rs:207` | JSON output token counts |
| `ResultUsage` | `output.rs:21` | Result output with cost |
| `ExtendedUsage` | `output_events.rs:139` | Stream events with cache tokens |

All represent input/output token counts but lack interoperability.

## Files to Modify

- `crates/cli/src/usage.rs` (new) - Unified usage types
- `crates/cli/src/config.rs` - Use unified type
- `crates/cli/src/output.rs` - Use unified type
- `crates/cli/src/output_events.rs` - Use unified type
- `crates/cli/src/lib.rs` - Export new module

## Implementation

### Step 1: Create unified usage module

Create `crates/cli/src/usage.rs`:

```rust
//! Token usage types for tracking API consumption.

use serde::{Deserialize, Serialize};

/// Basic token counts (input/output only).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenCounts {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl TokenCounts {
    pub fn new(input: u32, output: u32) -> Self {
        Self { input_tokens: input, output_tokens: output }
    }

    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Extended token counts including cache metrics.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExtendedTokenCounts {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    #[serde(default)]
    pub cache_read_input_tokens: u32,
}

impl ExtendedTokenCounts {
    pub fn new(input: u32, output: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        }
    }

    pub fn with_cache(mut self, creation: u32, read: u32) -> Self {
        self.cache_creation_input_tokens = creation;
        self.cache_read_input_tokens = read;
        self
    }
}

impl From<TokenCounts> for ExtendedTokenCounts {
    fn from(counts: TokenCounts) -> Self {
        Self::new(counts.input_tokens, counts.output_tokens)
    }
}

impl From<&ExtendedTokenCounts> for TokenCounts {
    fn from(ext: &ExtendedTokenCounts) -> Self {
        Self::new(ext.input_tokens, ext.output_tokens)
    }
}

/// Token usage with cost calculation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageWithCost {
    #[serde(flatten)]
    pub tokens: ExtendedTokenCounts,
    pub cost_usd: f64,
}

impl UsageWithCost {
    pub fn from_tokens(input: u32, output: u32) -> Self {
        let tokens = ExtendedTokenCounts::new(input, output);
        let cost_usd = estimate_cost(input, output);
        Self { tokens, cost_usd }
    }
}

/// Estimate cost based on Claude Sonnet pricing ($3/M input, $15/M output).
pub fn estimate_cost(input_tokens: u32, output_tokens: u32) -> f64 {
    let input_cost = (input_tokens as f64) * 0.000003;
    let output_cost = (output_tokens as f64) * 0.000015;
    input_cost + output_cost
}
```

### Step 2: Update config.rs

```rust
// Before
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct UsageSpec {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// After
pub use crate::usage::TokenCounts as UsageSpec;
```

### Step 3: Update output.rs

```rust
// Before
pub struct Usage { pub input_tokens: u32, pub output_tokens: u32 }
pub struct ResultUsage { ... }

// After
pub use crate::usage::{TokenCounts as Usage, UsageWithCost as ResultUsage};
// Remove duplicate estimate_cost function
```

### Step 4: Update output_events.rs

```rust
// Before
pub struct ExtendedUsage { ... }

// After
pub use crate::usage::ExtendedTokenCounts as ExtendedUsage;
```

## Migration Path

Use type aliases (`pub use X as Y`) to maintain API compatibility during transition.

## Testing

- Existing tests should pass with type aliases
- Add conversion tests between types

## Lines Changed

- ~50 lines removed (duplicate type definitions)
- ~60 lines added (unified module with conversions)
- Net: +10 lines but much better maintainability
