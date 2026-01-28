# Capture Crate Setup Review

Plan to extract the capture module from `crates/cli` into a standalone `crates/capture` crate, enabling reuse and cleaner separation of concerns.

## Overview

Extract `crates/cli/src/capture.rs` into a new `crates/capture` crate. This module provides interaction capture and recording for test assertions - a self-contained utility that can be reused independently of the CLI.

**Current state:** Module exists at `crates/cli/src/capture.rs` (~230 lines)
**Target state:** Standalone crate at `crates/capture/` with proper workspace integration

## Project Structure

```
crates/
├── cli/                    # Existing CLI crate (will depend on capture)
│   └── src/
│       └── lib.rs          # Re-exports capture types from new crate
└── capture/                # New capture crate
    ├── Cargo.toml
    └── src/
        ├── lib.rs          # Public API exports
        ├── interaction.rs  # CapturedInteraction, CapturedArgs, CapturedOutcome
        ├── interaction_tests.rs
        ├── log.rs          # CaptureLog implementation
        ├── log_tests.rs
        └── duration_serde.rs  # Duration serialization helpers
```

## Dependencies

**Production dependencies (from existing module):**
- `parking_lot = "0.12"` - Mutex for thread-safe storage
- `serde = { version = "1", features = ["derive"] }` - Serialization
- `serde_json = "1"` - JSON output for JSONL file format

**Dev dependencies (to add):**
- `rstest = "0.26"` - Parameterized testing
- `proptest = "1"` - Property-based testing
- `tempfile = "3"` - Temporary files for file-writing tests

**Lints (match cli crate):**
```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

## Implementation Phases

### Phase 1: Create Crate Scaffold

Create the basic crate structure with workspace integration.

1. Create `crates/capture/Cargo.toml`:
```toml
[package]
name = "claudeless-capture"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Interaction capture and recording for claudeless tests"
repository = "https://github.com/alfredjeanlab/claudeless"
keywords = ["claude", "testing", "capture", "recording"]
categories = ["development-tools::testing"]

[lib]
name = "claudeless_capture"
path = "src/lib.rs"

[dependencies]
parking_lot = "0.12"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
rstest = "0.26"
proptest = "1"
tempfile = "3"

[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

2. Add to workspace `Cargo.toml`:
```toml
members = ["crates/cli", "crates/capture"]
```

**Verification:** `cargo check -p claudeless-capture`

### Phase 2: Extract Module Code

Move capture types into separate module files following claudeless patterns.

1. Create `src/interaction.rs` - Data types:
   - `CapturedInteraction`
   - `CapturedArgs`
   - `CapturedOutcome`

2. Create `src/log.rs` - `CaptureLog` implementation

3. Create `src/duration_serde.rs` - Duration serialization helper (private module)

4. Create `src/lib.rs` - Re-export public API:
```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Interaction capture and recording for test assertions.
//!
//! This crate provides utilities for capturing and recording CLI interactions,
//! useful for testing and debugging claudeless scenarios.

mod duration_serde;
mod interaction;
mod log;

pub use interaction::{CapturedArgs, CapturedInteraction, CapturedOutcome};
pub use log::CaptureLog;
```

**Verification:** `cargo build -p claudeless-capture`

### Phase 3: Add Test Files

Create sibling `_tests.rs` files following CLAUDE.md convention.

1. Create `src/interaction_tests.rs`:
```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

// Serialization tests
// CapturedOutcome variant tests
```

2. Create `src/log_tests.rs`:
```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

// Record/retrieve tests
// File writing tests (using tempfile)
// Thread safety tests
```

3. Add rstest parameterized tests and proptest properties where valuable

**Verification:** `cargo test -p claudeless-capture`

### Phase 4: Update CLI Crate

Update `crates/cli` to use the extracted crate.

1. Add dependency to `crates/cli/Cargo.toml`:
```toml
claudeless-capture = { path = "../capture" }
```

2. Update `crates/cli/src/lib.rs` to re-export from new crate:
```rust
pub use claudeless_capture::{CapturedArgs, CapturedInteraction, CapturedOutcome, CaptureLog};
```

3. Remove `crates/cli/src/capture.rs` and `capture_tests.rs`

4. Update any internal imports in cli crate

**Verification:** `cargo build -p claudeless && cargo test -p claudeless`

### Phase 5: Full Workspace Verification

Run complete verification suite.

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all`
4. `cargo build --all`
5. `cargo publish -p claudeless-capture --dry-run`

**Verification:** `make check` passes

## Key Implementation Details

### Module Split Strategy

The existing `capture.rs` (229 lines) splits cleanly:
- **interaction.rs** (~70 lines): Data types with serde derives
- **log.rs** (~100 lines): CaptureLog with Arc<Mutex> storage
- **duration_serde.rs** (~30 lines): Private serde helper

### Thread Safety

`CaptureLog` uses `Arc<Mutex<_>>` pattern from `parking_lot`:
- Multiple clones share the same underlying storage
- Safe to share across threads for concurrent recording
- File writer also wrapped in Arc<Mutex> for safe concurrent writes

### Serde Conventions

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CapturedOutcome { ... }
```

The `duration_serde` helper handles `Duration` which doesn't have built-in serde support.

### Naming Convention

Crate name: `claudeless-capture` (hyphenated for crates.io)
Lib name: `claudeless_capture` (underscored for Rust imports)

This follows standard Rust conventions and matches common practice.

## Verification Plan

### Unit Tests (per module)

| Module | Tests |
|--------|-------|
| `interaction` | Serialization roundtrip, variant construction |
| `log` | Record/retrieve, counting, filtering, file I/O |

### Property Tests (proptest)

- Arbitrary `CapturedArgs` roundtrips through JSON
- `CaptureLog.len()` equals number of `record()` calls
- `find_*` filters are consistent with manual iteration

### Integration Tests

- CLI crate still functions with extracted dependency
- Re-exported types have correct public visibility

### CI Checklist

```bash
# Must pass before merge
make check

# Individual steps if debugging
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --all
cargo publish -p claudeless-capture --dry-run
cargo audit
cargo deny check
```
