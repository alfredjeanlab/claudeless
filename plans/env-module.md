# Plan: Centralized `env.rs` Module

## Overview

Add a centralized `env.rs` module to the claudeless CLI crate that serves as the single source of truth for all runtime environment variables. A `build.rs` script generates string constants for env var names, and `env.rs` provides typed accessor functions. All existing call sites are updated to use the new module.

## Project Structure

```
crates/cli/
├── build.rs              # NEW — generates env var name constants
├── src/
│   ├── env.rs            # NEW — typed accessors for all env vars
│   ├── env_tests.rs      # NEW — unit tests
│   ├── lib.rs            # MODIFIED — add `pub mod env;`
│   ├── config.rs         # MODIFIED — use crate::env for timeout env vars
│   ├── state/directory.rs # MODIFIED — use crate::env for dir env vars
│   ├── mcp/config.rs     # MODIFIED — use crate::env for MCP timeout
│   ├── tui/app/format.rs # MODIFIED — use crate::env for HOME
│   └── api.rs            # MODIFIED — use crate::env for build paths
```

## Dependencies

No new external dependencies. Uses only `std::env::var` and `std::path::PathBuf`.

## Implementation Phases

### Phase 1: Create `build.rs` with env var name constants

Create `crates/cli/build.rs` that writes a generated file containing `&str` constants for every env var name. This eliminates stringly-typed references scattered across the codebase.

**File: `crates/cli/build.rs`**

```rust
use std::io::Write;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let path = std::path::Path::new(&out_dir).join("env_names.rs");
    let mut f = std::fs::File::create(path).unwrap();

    let vars = [
        ("CLAUDELESS_CONFIG_DIR", "CLAUDELESS_CONFIG_DIR"),
        ("CLAUDELESS_STATE_DIR", "CLAUDELESS_STATE_DIR"),
        ("CLAUDE_CONFIG_DIR", "CLAUDE_CONFIG_DIR"),
        ("CLAUDELESS_EXIT_HINT_TIMEOUT_MS", "CLAUDELESS_EXIT_HINT_TIMEOUT_MS"),
        ("CLAUDELESS_COMPACT_DELAY_MS", "CLAUDELESS_COMPACT_DELAY_MS"),
        ("CLAUDELESS_HOOK_TIMEOUT_MS", "CLAUDELESS_HOOK_TIMEOUT_MS"),
        ("CLAUDELESS_MCP_TIMEOUT_MS", "CLAUDELESS_MCP_TIMEOUT_MS"),
        ("CLAUDELESS_RESPONSE_DELAY_MS", "CLAUDELESS_RESPONSE_DELAY_MS"),
        ("CARGO_BIN_EXE_CLAUDELESS", "CARGO_BIN_EXE_claudeless"),
        ("CARGO_TARGET_DIR", "CARGO_TARGET_DIR"),
        ("HOME", "HOME"),
    ];

    for (const_name, env_name) in vars {
        writeln!(f, "pub const {const_name}: &str = \"{env_name}\";").unwrap();
    }
}
```

The constant names match the env var names except `CARGO_BIN_EXE_claudeless` which becomes `CARGO_BIN_EXE_CLAUDELESS` as a valid Rust identifier.

**Verification:** `cargo build` succeeds and the generated file exists in `OUT_DIR`.

### Phase 2: Create `env.rs` module with typed accessors

Create `crates/cli/src/env.rs` that includes the generated constants and provides accessor functions.

**File: `crates/cli/src/env.rs`**

```rust
//! Centralized environment variable access.
//!
//! All runtime environment variables used by claudeless are defined here.
//! Use these accessors instead of calling `std::env::var()` directly.

/// Generated env var name constants.
mod names {
    include!(concat!(env!("OUT_DIR"), "/env_names.rs"));
}

// Re-export name constants for callers that need the raw name string.
pub use names::*;

use std::path::PathBuf;

/// `CLAUDELESS_CONFIG_DIR` — Claudeless-specific config directory override.
pub fn config_dir() -> Option<PathBuf> {
    std::env::var(names::CLAUDELESS_CONFIG_DIR).ok().map(PathBuf::from)
}

/// `CLAUDELESS_STATE_DIR` — Legacy state directory (backwards compatibility).
pub fn state_dir() -> Option<PathBuf> {
    std::env::var(names::CLAUDELESS_STATE_DIR).ok().map(PathBuf::from)
}

/// `CLAUDE_CONFIG_DIR` — Standard Claude Code config directory.
pub fn claude_config_dir() -> Option<PathBuf> {
    std::env::var(names::CLAUDE_CONFIG_DIR).ok().map(PathBuf::from)
}

/// `CLAUDELESS_EXIT_HINT_TIMEOUT_MS` — Exit hint display duration.
pub fn exit_hint_timeout_ms() -> Option<u64> {
    var_u64(names::CLAUDELESS_EXIT_HINT_TIMEOUT_MS)
}

/// `CLAUDELESS_COMPACT_DELAY_MS` — Delay before compacting output.
pub fn compact_delay_ms() -> Option<u64> {
    var_u64(names::CLAUDELESS_COMPACT_DELAY_MS)
}

/// `CLAUDELESS_HOOK_TIMEOUT_MS` — Hook execution timeout.
pub fn hook_timeout_ms() -> Option<u64> {
    var_u64(names::CLAUDELESS_HOOK_TIMEOUT_MS)
}

/// `CLAUDELESS_MCP_TIMEOUT_MS` — MCP server timeout. Default 30000.
pub fn mcp_timeout_ms() -> Option<u64> {
    var_u64(names::CLAUDELESS_MCP_TIMEOUT_MS)
}

/// `CLAUDELESS_RESPONSE_DELAY_MS` — Response delay between messages.
pub fn response_delay_ms() -> Option<u64> {
    var_u64(names::CLAUDELESS_RESPONSE_DELAY_MS)
}

/// `CARGO_BIN_EXE_claudeless` — Path to compiled binary (set by cargo test).
pub fn cargo_bin_exe() -> Option<String> {
    std::env::var(names::CARGO_BIN_EXE_CLAUDELESS).ok()
}

/// `CARGO_TARGET_DIR` — Cargo build target directory.
pub fn cargo_target_dir() -> Option<PathBuf> {
    std::env::var(names::CARGO_TARGET_DIR).ok().map(PathBuf::from)
}

/// `HOME` — User's home directory.
pub fn home() -> Option<PathBuf> {
    std::env::var(names::HOME).ok().map(PathBuf::from)
}

fn var_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}
```

Register the module in `lib.rs`:

```rust
#[doc(hidden)]
pub mod env;
```

**Verification:** `cargo build` and `cargo test` pass. `env.rs` compiles with the generated constants.

### Phase 3: Create `env_tests.rs`

Create `crates/cli/src/env_tests.rs` following the project convention.

Tests should cover:
- `config_dir()` returns `None` when unset, `Some(PathBuf)` when set
- `mcp_timeout_ms()` returns `None` when unset, parses valid u64, returns `None` for non-numeric
- `home()` returns a value (always set in CI/dev)
- `cargo_bin_exe()` returns `None` outside cargo test context

Use `std::env::set_var` / `std::env::remove_var` in tests. Since env vars are process-global, run these tests with `#[serial_test::serial]` or accept the race (consistent with existing test patterns in the project). Check whether `serial_test` is already a dependency; if not, the simplest approach is to use unique env var test values and accept minor race potential, or just use the `env::names::*` constants with `set_var`/`remove_var` in each test.

**Verification:** `cargo test env_tests` passes.

### Phase 4: Update `state/directory.rs` call site

Replace direct `std::env::var` calls in `StateDirectory::resolve()`:

```rust
// Before
if let Ok(dir) = std::env::var("CLAUDELESS_CONFIG_DIR") {
    ...
} else if let Ok(dir) = std::env::var("CLAUDELESS_STATE_DIR") {
    ...
} else if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
    ...
}

// After
if let Some(dir) = crate::env::config_dir() {
    Ok(Self::new(dir))
} else if let Some(dir) = crate::env::state_dir() {
    Ok(Self::new(dir))
} else if let Some(dir) = crate::env::claude_config_dir() {
    Ok(Self::new(dir))
} else {
    Self::temp()
}
```

**Verification:** Existing `directory_tests.rs` still pass.

### Phase 5: Update `config.rs`, `mcp/config.rs`, `tui/app/format.rs`, and `api.rs`

**`config.rs`** — Replace `Self::env_u64(...)` calls with `crate::env` accessors:

```rust
// Before
.or_else(|| Self::env_u64("CLAUDELESS_EXIT_HINT_TIMEOUT_MS"))

// After
.or_else(crate::env::exit_hint_timeout_ms)
```

Remove the private `env_u64` helper method from `ResolvedTimeouts` since it's now in `env.rs`.

**`mcp/config.rs`** — Replace `default_timeout()`:

```rust
fn default_timeout() -> u64 {
    crate::env::mcp_timeout_ms().unwrap_or(30000)
}
```

**`tui/app/format.rs`** — Replace `std::env::var("HOME")`:

```rust
// Before
if let Ok(home) = std::env::var("HOME") {
    let home_path = std::path::PathBuf::from(&home);

// After
if let Some(home_path) = crate::env::home() {
```

**`api.rs`** — Replace binary path resolution:

```rust
// Before
if let Ok(path) = std::env::var("CARGO_BIN_EXE_claudeless") {
    return std::path::PathBuf::from(path);
}
let target_dir = std::env::var("CARGO_TARGET_DIR")
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|_| std::path::PathBuf::from("target"));

// After
if let Some(path) = crate::env::cargo_bin_exe() {
    return std::path::PathBuf::from(path);
}
let target_dir = crate::env::cargo_target_dir()
    .unwrap_or_else(|| std::path::PathBuf::from("target"));
```

**Verification:** All existing tests pass. `cargo clippy` reports no warnings.

### Phase 6: Final verification

Run `make check` which covers:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo publish --dry-run`
- lint and audit checks

Grep the codebase for stray `std::env::var` calls in `crates/cli/src/` (excluding `env.rs` itself and compile-time `env!()` macros) to confirm none remain.

## Key Implementation Details

- **No `lazy_static`/`once_cell`**: Each accessor is a plain function calling `std::env::var()`. This keeps things simple, avoids caching stale values, and matches the current behavior.
- **`Option` return types for all accessors**: Callers decide defaults. The timeout accessors return `Option<u64>` so `ResolvedTimeouts` can chain `.or_else()` with its own defaults unchanged.
- **`build.rs` generates only name constants**: The accessor logic lives in hand-written `env.rs`, not generated code. This keeps the generated portion minimal and easy to understand.
- **Compile-time `env!()` macros unchanged**: `env!("CARGO_PKG_VERSION")`, `env!("CARGO_MANIFEST_DIR")` etc. are resolved by rustc at compile time and don't belong in runtime env access.
- **Module placement**: `env` is added to `lib.rs` as a `#[doc(hidden)] pub mod env` to match the existing pattern for internal modules.

## Verification Plan

1. **Phase 1-2**: `cargo build` succeeds with the new `build.rs` and `env.rs`
2. **Phase 3**: `cargo test env_tests` — unit tests for accessor functions
3. **Phase 4-5**: `cargo test --all` — all existing tests still pass after call-site migration
4. **Phase 6**: `make check` — full CI-equivalent validation
5. **Manual grep**: `rg 'std::env::var' crates/cli/src/ --glob '!env.rs'` returns only `env!()` compile-time macros (or nothing)
