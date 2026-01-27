# Claudeless

A Claude CLI simulator for integration testing.

## Directory Structure

```
claudeless/
├── crates/           # Rust workspace
│   └── cli/          # CLI binary and library
├── scenarios/        # Test scenario definitions
├── scripts/          # Build and utility scripts
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
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
  - `make lint` (shellcheck)
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `quench check --fix`
  - `cargo test --all`
  - `cargo build --all`
  - `cargo publish --dry-run` (verify crate packaging)
  - `cargo audit`
  - `cargo deny check`
