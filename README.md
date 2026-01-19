# Claudeless

A Claude CLI simulator for integration testing.

## Overview

Claudeless provides a controllable test double that responds to the same CLI interface as the real Claude CLI, enabling deterministic integration testing without API costs.

## Features

- Emulates the `claude` CLI interface
- Scenario-based test definitions
- TUI rendering support for screenshot testing
- MCP server simulation
- Permission mode testing
- Hook system for test orchestration

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
```

## Usage

Run with a scenario file:

```bash
claudeless --scenario scenarios/simple.toml
```

## Development

### Prerequisites

- Rust 1.75+
- cargo-audit
- cargo-deny

### Running Tests

```bash
cargo test
```

### CI Checks

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

## License

MIT - Copyright (c) 2026 Alfred Jean LLC
