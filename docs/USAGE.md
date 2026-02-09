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
| `--failure <MODE>` | `CLAUDELESS_FAILURE` | Inject failure (see below) |
| `--claude-version <VER>` | `CLAUDELESS_CLAUDE_VERSION` | Claude version to simulate |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `CLAUDELESS_CONFIG_DIR` | State directory override (highest priority) |
| `CLAUDE_CONFIG_DIR` | State directory (standard Claude Code variable) |
| `CLAUDELESS_CLAUDE_VERSION` | Claude version to simulate |

If neither config dir variable is set, a temporary directory is used to avoid touching real `~/.claude`.

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
[[responses]]
on = "*"
say = "Hello from Claudeless!"
```

### Pattern Types

Bare strings are glob patterns (default). Use object form for `contains` and `regexp`.

| Type | Example | Description |
|------|---------|-------------|
| glob | `"*"`, `"hello"`, `"*.txt"` | Shell wildcards, exact match, catch-all (default) |
| contains | `{ contains = "error" }` | Substring match |
| regexp | `{ regexp = "(?i)fix.*bug" }` | Regex match |

### Response Rules

Response fields are specified directly on the rule alongside the pattern.

**Simple:**
```toml
[[responses]]
on = { contains = "hello" }
say = "Plain text response"
```

**With tool calls:**
```toml
[[responses]]
on = { contains = "hello" }
say = "Response with metadata"
delay_ms = 100
usage = { input_tokens = 100, output_tokens = 50 }

[[responses.tools]]
call = "Read"
input = { file_path = "/src/main.rs" }
result = "fn main() { ... }"
```

### Failure Injection

```toml
[[responses]]
on = { contains = "timeout" }
failure = { type = "connection_timeout", after_ms = 5000 }
```

### Multi-Turn Conversations

```toml
[[responses]]
on = { contains = "help" }
say = "What do you need?"
then = [
    { on = { contains = "debug" }, say = "Starting debugger..." },
    { on = "*", say = "I'll look into that." }
]
```

### Deterministic Testing

```toml
[claude]
session_id = "550e8400-e29b-41d4-a716-446655440000"
launch_timestamp = "2025-01-15T10:30:00Z"
username = "TestUser"
```

### Tool Execution Config

Tool execution mode is configured in the scenario file's `[tools]` section.
The default mode is `live` (execute tools directly).

| Mode | Description |
|------|-------------|
| `mock` | Return pre-configured results from scenario |
| `live` | Execute built-in tools directly (default) |

```toml
[tools]
mode = "mock"  # or "live" (default)

[tools.Bash]
approve = true

[tools.Write]
approve = false
error = "Permission denied"
```

### AskUserQuestion (Elicitation)

The AskUserQuestion tool presents questions with selectable options. In TUI mode, an interactive elicitation dialog is shown. In print mode, pre-configured answers are used or the first option is auto-selected.

**TUI keyboard interaction** (matches real Claude Code):

| Key | Behavior |
|-----|----------|
| `↑` / `↓` | Navigate options; Up wraps from first option to "Type something."; Down clamps at "Chat about this" |
| `1`–`9` | Select defined option by number and immediately submit |
| `Space` | Toggle selection (multi-select); types space on "Type something." row |
| `Enter` | Submit highlighted option |
| `Escape` | Cancel — returns `"User declined to answer questions"` |
| Letters | Ignored on defined options; types into free-text on "Type something." row |
| `Backspace` | Deletes last character on "Type something." row |

**Extra options** appended after defined options:

- **Type something.** — Free-text input. Navigate here with arrow keys, then type. Enter submits the typed text as the answer. Enter with empty text cancels.
- **Chat about this** — Below separator. Sends a clarification rejection asking Claude to reformulate.

```toml
[tools.AskUserQuestion]
approve = true

[tools.AskUserQuestion.answers]
"What language?" = "Rust"
```

## Compatible Claude CLI Flags

Claudeless accepts all standard Claude CLI flags for compatibility:

```example
-p, --print                    Non-interactive single response
--model <MODEL>                Model name (ignored, for compatibility)
--output-format <FORMAT>       text | json | stream-json
--permission-mode <MODE>       default | plan | bypass-permissions | ...
--allow-dangerously-skip-permissions  Enable bypass option
--dangerously-skip-permissions        Bypass all permission checks
--continue, -c                 Continue previous conversation
--resume, -r <ID>              Resume specific conversation
--session-id <UUID>            Use specific session ID
--no-session-persistence       Disable session persistence (print mode only)
--cwd <DIR>                    Working directory
--system-prompt <TEXT>         System prompt
--append-system-prompt <TEXT>  Append to default system prompt
--allowedTools <TOOL>          Allow specific tools
--disallowedTools <TOOL>       Disallow specific tools
--input-file <FILE>            Read prompt from file
--input-format <FORMAT>        Input format (text | stream-json)
--verbose                      Verbose output mode
-d, --debug [FILTER]           Debug mode with optional filter
--mcp-config <CONFIG>          MCP server configuration
--strict-mcp-config            Only use servers from --mcp-config
--mcp-debug                    Enable MCP debug output
--settings <FILE_OR_JSON>      Load settings from file or inline JSON
--setting-sources <SOURCES>    Comma-separated setting sources (user, project, local)
--max-budget-usd <AMOUNT>      Maximum budget in USD
--fallback-model <MODEL>       Fallback model on overload
```

Additional compatibility flags (accepted, ignored):

```example
--add-dir <DIR>                Additional directories for tool access
--agent <AGENT>                Agent for the session
--agents <JSON>                Custom agent definitions
--betas <BETA>                 Beta headers
--chrome / --no-chrome         Chrome integration
--debug-file <PATH>            Debug log file path
--disable-slash-commands       Disable all skills
--file <FILE>                  File resources to download
--fork-session                 Create new session ID on resume
--from-pr [PR]                 Resume session linked to a PR
--ide                          IDE integration
--json-schema <SCHEMA>         Structured output validation
--plugin-dir <DIR>             Plugin directories
--replay-user-messages         Re-emit user messages on stdout
--tools <TOOL>                 Specify available built-in tools
--include-partial-messages     Include partial chunks (stream-json)
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

**Live tool execution:**
```bash
# Tools execute by default (live mode)
claudeless --scenario tools.toml \
           -p "edit the file"
```

## Further Reading

- [Scenario Reference](SCENARIOS.md) — Full scenario format documentation
- [Limitations](LIMITATIONS.md) — Known limitations and out-of-scope features
