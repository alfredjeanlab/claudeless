# Claudeless

A Claude CLI simulator for testing tools that call Claude Code.

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

### Homebrew (macOS)

```bash
brew install alfredjeanlab/tap/claudeless
```

### Linux / Manual

```bash
curl -fsSL https://github.com/alfredjeanlab/claudeless/releases/latest/download/install.sh | bash
```

### Cargo

```bash
cargo install claudeless
```

## Usage

Run with a scenario file:

```bash
claudeless --scenario scenarios/simple.toml
```

## Limitations

Claudeless simulates the Claude CLI by emulating its TUI layout, CLI interface, error messages, and filesystem modifications. Scenario files control responses and failures, built-in tools execute in a sandbox, and costs, tokens, and timing are simulated for deterministic assertions. State is written to a temp directory by default (override with `CLAUDELESS_CONFIG_DIR` or `CLAUDE_CONFIG_DIR`).

The core CLI experience is well-supported, including output formats, permission modes, and the hooks protocol. Ongoing work is focused on improving JSON output fidelity and TUI rendering accuracy.

Subagent simulation and MCP server execution are planned for future releases.

### Out of Scope

- Chrome and IDE integrations
- Settings screens (Status, Config, Usage)
- TUI colors are not expected to exactly match, though text layout matches exactly
- Administrative subcommands like `doctor`, `install`, and `update` are not supported

See [docs/LIMITATIONS.md](docs/LIMITATIONS.md).

## License

MIT - Copyright (c) 2026 Alfred Jean LLC

"Claude" is a registered trademark of Anthropic, PBC.
"Claude Code" is a product of Anthropic.
This project is not affiliated with or endorsed by Anthropic.
