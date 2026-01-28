# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-01-27

Initial release of claudeless - a Claude CLI simulator for integration testing.

### Features

- **CLI Interface**: Emulates the `claude` CLI with text, JSON, and stream-JSON output formats
- **TUI Rendering**: Interactive terminal UI with spinner, dialogs, and screenshot-testable output
- **Scenario System**: Pattern matching, multi-turn conversations, failure injection, and mock responses
- **Built-in Tools**: Sandboxed Bash, Read, Write, Edit, Glob, Grep, TodoWrite, ExitPlanMode
- **Permission Modes**: All 6 modes (default, acceptEdits, bypassPermissions, delegate, dontAsk, plan)
- **Hook Protocol**: All 7 events (pre/post tool, notification, permission, session start/end, prompt submit)
- **MCP Config**: JSON/JSON5 parsing and tool registration via `--mcp-config`
- **State Directory**: Simulated projects, todos, plans, sessions, and settings.json
- **Slash Commands**: `/clear`, `/compact`, `/fork`, `/help`, `/context`, `/exit`, `/todos`, `/tasks`, `/export`, `/hooks`, `/memory`
