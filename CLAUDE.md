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

## Landing the Plane

Before committing changes:

- [ ] Run `make check` which will
  - `make lint` (shellcheck)
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all`
  - `cargo build --all`
  - `cargo audit`
  - `cargo deny check`
