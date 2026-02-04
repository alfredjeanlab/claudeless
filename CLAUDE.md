# Claudeless

A Claude CLI simulator for integration testing.

## Directory Structure

```
claudeless/
├── crates/           # Rust workspace
│   └── cli/          # CLI binary and library
├── scenarios/        # Test scenario definitions
├── scripts/          # Build and utility scripts
├── tests/
│   ├── capture/      # Capture scripts for recording real Claude Code
│   ├── fixtures/     # Immutable snapshots from real Claude Code (DO NOT EDIT)
│   └── specs/        # Integration test specs (capsh + scenario pairs)
└── docs/             # Documentation
```

## Unit Test Convention

Use sibling `_tests.rs` files instead of inline `#[cfg(test)]` modules:

```rust
// src/parser.rs
#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
```

```rust
// src/parser_tests.rs
use super::*;

#[test]
fn parses_empty_input() { ... }
```

**Why separate files?**
- Shorter source files fit better in LLM context windows
- LOC metrics reflect implementation conciseness, not test volume
- Integration tests remain in `tests/` as usual

## Commits

Use conventional commit format: `type(scope): description`
Types: feat, fix, chore, docs, test, refactor

## Landing the Plane

Before committing changes:

- [ ] Unit tests in sibling `_tests.rs` files
- [ ] Run `make check` which will
  - `cargo fmt --all`
  - `cargo clippy --all -- -D warnings`
  - `quench check --fix`
  - `cargo build --all`
  - `cargo test --all`
