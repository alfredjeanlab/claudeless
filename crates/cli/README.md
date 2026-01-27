# claudeless

[![Crates.io](https://img.shields.io/crates/v/claudeless.svg)](https://crates.io/crates/claudeless)
[![Documentation](https://docs.rs/claudeless/badge.svg)](https://docs.rs/claudeless)
[![Build Status](https://github.com/alfredjeanlab/claudeless/actions/workflows/ci.yml/badge.svg)](https://github.com/alfredjeanlab/claudeless/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

A Claude CLI simulator for integration testing tools that invoke `claude`.

## Overview

Claudeless provides a controllable test double that responds to the same CLI interface as the real Claude CLI, enabling deterministic integration testing without API costs.

## Installation

### Cargo

```bash
cargo install claudeless
```

## Using in Test Suites

Add claudeless as a dev dependency to get the binary built alongside your tests:

```toml
[dev-dependencies]
claudeless = "0.2"
```

In your tests, use `CARGO_BIN_EXE_claudeless` to find the binary:

```rust
use std::process::Command;

#[test]
fn test_my_claude_integration() {
    let claudeless = env!("CARGO_BIN_EXE_claudeless");

    let output = Command::new(claudeless)
        .args(["--scenario", "tests/fixtures/scenario.toml"])
        .arg("-p")
        .arg("hello world")
        .output()
        .unwrap();

    assert!(output.status.success());
}
```

## Usage

Run with a scenario file:

```bash
claudeless --scenario scenario.toml -p "hello"
```

Run interactively (TUI mode):

```bash
claudeless --scenario scenario.toml
```

## Scenario Files

Control responses with TOML scenario files:

```toml
# scenario.toml
default_response = "I'm not sure how to help with that."

[[responses]]
pattern = { contains = "refactor" }
response = "I'll help refactor that code."

[[responses]]
pattern = { regex = "fix bug #(\\d+)" }
response = { text = "Fixed!", delay_ms = 100 }
```

See the [Scenario Reference](https://docs.rs/claudeless/latest/claudeless/docs/scenarios/) for full documentation.

## Features

- Pattern matching: substring, exact, regex, and glob
- Response delays for timing tests
- Multi-turn conversation simulation
- Tool call simulation
- Failure injection (network errors, rate limits, auth failures)
- Capture log for request inspection
- TUI mode for interactive testing

## License

MIT - Copyright (c) 2026 Alfred Jean LLC

"Claude" is a trademark of Anthropic, PBC. This project is not affiliated with Anthropic.
