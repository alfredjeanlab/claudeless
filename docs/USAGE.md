# Usage Guide

Claudeless is a Claude CLI simulator for deterministic integration testing without API costs.

## Quick Start

```sh
# Run with a scenario
claudeless --scenario scenarios/simple.toml -p "hello"

# Run interactively (TUI mode)
claudeless --scenario scenarios/simple.toml
```

## Claudeless-Specific Options

These flags and environment variables are unique to claudeless (not in the real Claude CLI).

### CLI Flags

| Flag | Env Variable | Description |
|------|--------------|-------------|
| `--scenario <FILE>` | `CLAUDELESS_SCENARIO` | Scenario file (TOML/JSON) |
| `--capture <FILE>` | `CLAUDELESS_CAPTURE` | Log all interactions to file |
| `--failure <MODE>` | `CLAUDELESS_FAILURE` | Inject failure (see below) |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `CLAUDELESS_CONFIG_DIR` | State directory override (highest priority) |
| `CLAUDE_CONFIG_DIR` | State directory (standard Claude Code variable) |

If neither is set, a temporary directory is used to avoid touching real `~/.claude`.

### Failure Modes

```bash
claudeless --failure network-unreachable -p "test"
claudeless --failure connection-timeout -p "test"
claudeless --failure auth-error -p "test"
claudeless --failure rate-limit -p "test"
claudeless --failure out-of-credits -p "test"
claudeless --failure partial-response -p "test"
claudeless --failure malformed-json -p "test"
```

## Scenario Files

Scenarios control simulator responses. Use TOML (preferred) or JSON.

### Minimal Example

```toml
name = "minimal"

[[responses]]
pattern = { type = "any" }
response = "Hello from Claudeless!"
```

### Pattern Types

| Type | Example | Description |
|------|---------|-------------|
| `exact` | `{ type = "exact", text = "hello" }` | Exact match |
| `contains` | `{ type = "contains", text = "error" }` | Substring match |
| `regex` | `{ type = "regex", pattern = "(?i)fix.*bug" }` | Regex match |
| `glob` | `{ type = "glob", pattern = "*.txt" }` | Shell wildcards |
| `any` | `{ type = "any" }` | Catch-all |

### Response Types

**Simple:**
```toml
response = "Plain text response"
```

**Detailed:**
```toml
[responses.response]
text = "Response with metadata"
delay_ms = 100
usage = { input_tokens = 100, output_tokens = 50 }

[[responses.response.tool_calls]]
tool = "Read"
input = { file_path = "/src/main.rs" }
result = "fn main() { ... }"
```

### Failure Injection

```toml
[[responses]]
pattern = { type = "contains", text = "timeout" }
response = ""
failure = { type = "connection_timeout", after_ms = 5000 }
```

### Multi-Turn Conversations

```toml
[[responses]]
pattern = { type = "contains", text = "help" }
response = "What do you need?"
turns = [
    { expect = { type = "contains", text = "debug" }, response = "Starting debugger..." },
    { expect = { type = "any" }, response = "I'll look into that." }
]
```

### Deterministic Testing

```toml
session_id = "550e8400-e29b-41d4-a716-446655440000"
launch_timestamp = "2025-01-15T10:30:00Z"
user_name = "TestUser"
```

### Tool Execution Config

Tool execution mode is configured in the scenario file's `[tool_execution]` section.
The default mode is `live` (execute tools directly).

| Mode | Description |
|------|-------------|
| `mock` | Return pre-configured results from scenario |
| `live` | Execute built-in tools directly (default) |

```toml
[tool_execution]
mode = "mock"  # or "live" (default)

[tool_execution.tools.Bash]
auto_approve = true

[tool_execution.tools.Write]
auto_approve = false
error = "Permission denied"
```

## Compatible Claude CLI Flags

Claudeless accepts all standard Claude CLI flags for compatibility:

```example
-p, --print                    Non-interactive single response
--model <MODEL>                Model name (ignored, for compatibility)
--output-format <FORMAT>       text | json | stream-json
--permission-mode <MODE>       default | plan | bypass-permissions | ...
--continue-conversation, -c    Continue previous conversation
--resume, -r <ID>              Resume specific conversation
--session-id <UUID>            Use specific session ID
--cwd <DIR>                    Working directory
--system-prompt <TEXT>         System prompt
--allowedTools <TOOL>          Allow specific tools
--disallowedTools <TOOL>       Disallow specific tools
--mcp-config <CONFIG>          MCP server configuration
```

## Examples

**CI pipeline test:**
```bash
CLAUDELESS_CONFIG_DIR=/tmp/test-state \
claudeless --scenario ci-review.toml \
           --output-format json \
           -p "review this PR"
```

**Error handling test:**
```bash
claudeless --failure rate-limit -p "test" || echo "Handled rate limit"
```

**Capture interactions:**
```bash
claudeless --scenario test.toml --capture /tmp/log.jsonl -p "hello"
```

**Live tool execution:**
```bash
# Tools execute by default (live mode)
claudeless --scenario tools.toml \
           -p "edit the file"
```

## Further Reading

- [Scenario Reference](SCENARIOS.md) — Full scenario format documentation
- [Limitations](LIMITATIONS.md) — Known limitations and out-of-scope features
