# Implementation Plan: test-capture Crate Setup

## Overview

Create a new `crates/test-capture` crate with basic module structure and dependencies. This crate will provide utilities for capturing and comparing test output from Claude CLI integration tests.

## Project Structure

```
crates/test-capture/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs        # Library entry point with module declarations
    └── main.rs       # Binary entry point (placeholder for capture CLI)
```

## Dependencies

**Runtime dependencies:**
- `clap = { version = "4", features = ["derive"] }` - CLI argument parsing
- `tokio = { version = "1", features = ["fs", "rt-multi-thread", "macros"] }` - Async runtime
- `serde = { version = "1", features = ["derive"] }` - Serialization
- `serde_json = "1"` - JSON support
- `similar = "2"` - Diff generation for output comparison
- `tempfile = "3"` - Temporary file/directory management

**Dev dependencies:**
- Standard test deps consistent with cli crate

## Implementation Phases

### Phase 1: Workspace Configuration

Update root `Cargo.toml` to include the new crate:

```toml
members = ["crates/cli", "crates/test-capture"]
```

**Verification:** `cargo metadata` shows test-capture in workspace members.

---

### Phase 2: Create Cargo.toml

Create `crates/test-capture/Cargo.toml` following project conventions:

```toml
[package]
name = "test-capture"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Test capture and comparison utilities for claudeless"

[lib]
name = "test_capture"
path = "src/lib.rs"

[[bin]]
name = "test-capture"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["fs", "rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
similar = "2"
tempfile = "3"

[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

**Verification:** `cargo check -p test-capture` succeeds.

---

### Phase 3: Create lib.rs

Create `crates/test-capture/src/lib.rs` with the project header and placeholder structure:

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Test capture and comparison utilities for claudeless.
//!
//! This crate provides tools for capturing CLI output and comparing
//! it against expected baselines for integration testing.

// Modules will be added as features are implemented
```

**Verification:** `cargo doc -p test-capture` builds documentation.

---

### Phase 4: Create main.rs

Create `crates/test-capture/src/main.rs` with a minimal async main:

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Test capture CLI entry point.

use clap::Parser;

/// Test capture CLI for recording and comparing test output
#[derive(Parser, Debug)]
#[command(name = "test-capture")]
#[command(about = "Capture and compare CLI test output")]
struct Cli {
    /// Placeholder argument
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.verbose {
        eprintln!("test-capture: verbose mode enabled");
    }

    println!("test-capture: ready");
    Ok(())
}
```

**Verification:** `cargo run -p test-capture -- --help` shows usage.

---

### Phase 5: Full Build Verification

Run the complete verification suite:

```bash
make check
```

This validates:
- Formatting (`cargo fmt --check`)
- Linting (`cargo clippy`)
- Tests (`cargo test --all`)
- Build (`cargo build --all`)
- Publish dry-run (`cargo publish --dry-run`)

**Verification:** All checks pass with no errors.

## Key Implementation Details

### Naming Convention
- Crate name: `test-capture` (with hyphen, for Cargo.toml `[package]` name)
- Library name: `test_capture` (with underscore, for `use test_capture::...`)
- Binary name: `test-capture` (matches crate name)

### Lint Configuration
Following the cli crate pattern, strict lints are enforced:
- `unsafe_code = "forbid"` - No unsafe code allowed
- `unwrap_used = "deny"` - Must use proper error handling
- `expect_used = "deny"` - Must use proper error handling
- `panic = "deny"` - Must not panic in library code

### Workspace Inheritance
Uses `version.workspace = true` pattern for:
- `version` - Inherits from `[workspace.package]`
- `edition` - Inherits "2021"
- `license` - Inherits "MIT"

## Verification Plan

1. **Compile check:** `cargo check -p test-capture`
2. **Binary runs:** `cargo run -p test-capture -- --help`
3. **Workspace build:** `cargo build --all`
4. **Full CI check:** `make check`
5. **Documentation:** `cargo doc -p test-capture --open`
