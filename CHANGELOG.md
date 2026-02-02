# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-02-01

### Features

- **MCP Support**: Full MCP server lifecycle — JSON-RPC stdio transport, client/server management, tool executor routing, and init event formatting matching real Claude CLI
- **Session Resume**: `--resume` flag for continuing previous sessions
- **Positional Prompt**: Process positional prompt argument in TTY mode
- **JSONL Recording**: Session writing, tool_use/tool_result recording, error recording, and stop_reason/result records for log extraction
- **CLI Settings**: `--settings` flag for overrides, `--setting-sources` to filter loaded settings, `--no-session-persistence` flag
- **Hook Events**: PreCompact and Stop hook event support, Stop hook gating
- **TUI Dialogs**: Scenario-driven permission dialogs, thinking dialog mid-conversation warning

### Refactored

- **State Management**: Consolidated state modules, simplified StateWriter with MessageIds and WriteContext
- **CLI Organization**: Reorganized CLI struct into nested option groups, removed `--tui`/`--no-tui` and `--tool-mode` flags
- **TUI Architecture**: Flattened app/state/ and app/render/ directories, split commands into focused modules
- **Runtime**: Extracted Runtime/RuntimeBuilder, shared agentic loop via `Runtime::execute()`
- **Scenario Config**: Split ScenarioConfig into focused sub-structs
- **Capture System**: Rewrote capture.sh in TypeScript (Bun), added TOML-based capture specs, merged capture crate into claudeless
- **Types**: Unified API ContentBlock with state::ContentBlock, extracted MessageEnvelope, ToolName enum, TodoStatus enum
- **Tests**: TuiTestSession RAII wrapper, assertion helpers with yare, consolidated test setup

### Fixed

- Hook execution correctness
- MCP execution
- Session ID validation
- Config dir env vars aligned with real Claude CLI
- Canonical system dirs
- Non-TTY mode error without prompt
- stop_reason written to session JSONL instead of null

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
