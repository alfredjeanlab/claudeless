# Contributing to Claudeless

## Prerequisites

- Rust 1.75+
- cargo-audit
- cargo-deny

## Running Tests

```bash
cargo test
```

## CI Checks

```bash
make check
```

This runs:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`
