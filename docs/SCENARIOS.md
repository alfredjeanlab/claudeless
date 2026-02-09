# Scenario Reference

Scenarios define how the Claudeless simulator responds to prompts. They are TOML or JSON files that configure response patterns, failure injection, multi-turn conversations, and tool execution behavior.

## Table of Contents

- [File Format](#file-format)
- [Top-Level Fields](#top-level-fields)
- [Pattern Specifications](#pattern-specifications)
- [Response Rules](#response-rules)
- [Failure Injection](#failure-injection)
- [Follow-Up Turns](#follow-up-turns)
- [Tool Execution](#tool-execution)
- [Validation Rules](#validation-rules)
- [Examples](#examples)

---

## File Format

Scenarios are loaded via the `--scenario` CLI flag:

```bash
claudeless --scenario scenarios/simple.toml -p "hello"
```

Supported formats: **TOML** (preferred) and **JSON**.

### Minimal Example

```toml
[[responses]]
on = "*"
say = "Hello from Claudeless!"
```

---

## Top-Level Fields

### Claude Configuration

Configure simulator identity and environment in the `[claude]` section:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `claude.model` | string | `"claude-opus-4-5-20251101"` | Model to report (overridden by `--model` CLI flag) |
| `claude.version` | string | `"2.1.12"` | Claude version string |
| `claude.username` | string | `"Alfred"` | User display name |
| `claude.session_id` | string | (random) | Fixed UUID for deterministic tests |
| `claude.project_path` | string | (cwd) | Override project path |
| `claude.launch_timestamp` | string | (now) | Fixed timestamp in ISO 8601 with timezone (e.g., `"2025-01-15T10:30:00Z"`) |
| `claude.placeholder` | string | (default) | Placeholder text for input prompt |
| `claude.provider` | string | `"Claude Max"` | Provider name shown in header |
| `claude.working_directory` | string | (cwd) | Simulated working directory |
| `claude.show_welcome_back` | bool | `false` | Show "Welcome back!" splash instead of normal header |
| `claude.welcome_back_right_panel` | array | (default) | Right panel rows for welcome back box |

### Timeouts

Configure various timeout values in the `[claude.timeouts]` section:

```toml
[claude.timeouts]
exit_hint_ms = 2000      # "Press Ctrl-C again" hint duration
compact_delay_ms = 20    # /compact spinner delay
hook_timeout_ms = 5000   # Hook script execution limit
mcp_timeout_ms = 30000   # MCP server response timeout
response_delay_ms = 100  # Delay before sending response
```

All timeouts can also be set via environment variables:

| Field | Env Variable | Default |
|-------|--------------|---------|
| `exit_hint_ms` | `CLAUDELESS_EXIT_HINT_TIMEOUT_MS` | 2000 |
| `compact_delay_ms` | `CLAUDELESS_COMPACT_DELAY_MS` | 20 |
| `hook_timeout_ms` | `CLAUDELESS_HOOK_TIMEOUT_MS` | 5000 |
| `mcp_timeout_ms` | `CLAUDELESS_MCP_TIMEOUT_MS` | 30000 |
| `response_delay_ms` | `CLAUDELESS_RESPONSE_DELAY_MS` | 20 |

**Precedence:** scenario config > environment variable > default

### Environment

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `trusted` | bool | `true` | Whether directory is trusted |
| `logged_in` | bool | `true` | Whether user is logged in (shows setup wizard when false) |
| `permission_mode` | string | `"default"` | Permission mode override |

**Permission Mode Values:**

| Value | Description |
|-------|-------------|
| `default` | Standard prompts for permissions |
| `plan` | Show plan before executing |
| `bypass-permissions` | Skip all permission checks |
| `accept-edits` | Auto-accept edit permissions |
| `dont-ask` | Deny operations that would require permission |
| `delegate` | Delegate to higher authority |

### Response Configuration

| Field | Type | Description |
|-------|------|-------------|
| `responses` | array | Response rules (evaluated in order) |
| `default` | object | Fallback when no pattern matches |
| `tools` | object | Tool execution configuration |

---

## Pattern Specifications

Patterns are specified with the `on` field. Rules are evaluated in order; first match wins.

### Glob (default)

A bare string is a glob pattern. Supports shell-style wildcards (`*`, `?`, `[...]`), exact match (literal text), and catch-all (`*`).

```toml
on = "*.txt"       # shell wildcards
on = "hello"       # exact match
on = "*"           # match any
```

### Contains

Case-sensitive substring match. Requires explicit object form.

```toml
on = { contains = "error" }
```

### Regexp

Full Rust regex syntax. Requires explicit object form.

```toml
on = { regexp = "(?i)fix.*bug" }
```

---

## Response Rules

Response fields are specified directly on the rule alongside the pattern.

> **Note:** The `say` field is optional when `failure` is set (failures don't produce responses). For follow-up turns, `say` is always required (can be empty string `""`).

### Simple Response

```toml
[[responses]]
on = { contains = "hello" }
say = "Hello back!"
```

### Response with Metadata

```toml
[[responses]]
on = { contains = "hello" }
say = "Hello back!"
delay_ms = 100
usage = { input_tokens = 100, output_tokens = 50 }

[[responses.tools]]
call = "Read"
input = { file_path = "/src/main.rs" }
result = "fn main() { ... }"
```

### Response Rule Fields

| Field | Type | Description |
|-------|------|-------------|
| `on` | pattern | Pattern to match against prompt |
| `say` | string | Response text |
| `tools` | array | Simulated tool calls |
| `usage` | object | Token usage (`input_tokens`, `output_tokens`) |
| `delay_ms` | int | Response delay in milliseconds |
| `failure` | object | Failure to inject (see below) |
| `max` | int | Maximum number of times this rule can match |
| `then` | array | Follow-up turns (see below) |

### Tool Call Fields

| Field | Type | Description |
|-------|------|-------------|
| `call` | string | Tool name (e.g., `"Read"`, `"Bash"`, `"Write"`) |
| `input` | object | Tool input parameters |
| `result` | string | Canned result (optional) |

### File References

Reference external files using the `$file` key (resolved relative to scenario file). The file contents replace the `$file` object:

```toml
[[responses.tools]]
call = "Write"
input = { file_path = "/tmp/plan.md", content = { "$file" = "fixtures/plan.md" } }
```

For JSON files (`.json` extension), content is parsed as JSON; otherwise loaded as a string.

### Match Limits

Limit how many times a rule can match:

```toml
[[responses]]
on = { contains = "hello" }
say = "First hello only!"
max = 1
```

### Default Response

Fallback when no pattern matches:

```toml
[default]
say = "I'm not sure how to help with that."
delay_ms = 100
```

---

## Failure Injection

Inject failures instead of normal responses for error handling tests.

### Failure Types

| Type | Fields | Description |
|------|--------|-------------|
| `network_unreachable` | — | Network is unavailable |
| `connection_timeout` | `after_ms` | Connection times out |
| `auth_error` | `message` | Authentication failure |
| `rate_limit` | `retry_after` | Rate limited (seconds) |
| `out_of_credits` | — | Account out of credits |
| `partial_response` | `partial_text` | Incomplete response |
| `malformed_json` | `raw` | Return malformed JSON |

### Examples

```toml
[[responses]]
on = { contains = "timeout" }
failure = { type = "connection_timeout", after_ms = 100 }

[[responses]]
on = { contains = "auth" }
failure = { type = "auth_error", message = "API key expired" }

[[responses]]
on = { contains = "rate" }
failure = { type = "rate_limit", retry_after = 30 }

[[responses]]
on = { contains = "partial" }
failure = { type = "partial_response", partial_text = "I was about to..." }
```

---

## Follow-Up Turns

Response rules can have follow-up `then` turns for multi-step interactions.

### Basic Turn Sequence

```toml
[[responses]]
on = { contains = "login" }
say = "Enter username:"
then = [
    { on = "*", say = "Enter password:" },
    { on = "*", say = "Login successful!" }
]
```

### How Turn Sequences Work

1. When `on` matches, return `say` and activate the turn sequence
2. Subsequent prompts match against the current turn's `on` pattern
3. If turn matches, return its `say` and advance to next turn
4. When all turns complete, sequence deactivates
5. If a turn doesn't match, sequence deactivates and normal matching resumes

### Turn Fields

| Field | Type | Description |
|-------|------|-------------|
| `on` | pattern | Pattern to match for this turn |
| `say` | string | Response text for this turn |
| `tools` | array | Simulated tool calls |
| `usage` | object | Token usage |
| `delay_ms` | int | Response delay |
| `failure` | object | Optional failure for this turn |

### Turns with Failures

```toml
[[responses]]
on = { contains = "auth" }
say = "Authenticating..."
then = [
    { on = "*", say = "", failure = { type = "auth_error", message = "Invalid token" } }
]
```

---

## Tool Execution

Configure how tools are executed during simulation in the `[tools]` section.

### Execution Modes

| Mode | Description |
|------|-------------|
| `mock` | Canned results only; errors if a tool call has no `result` field |
| `live` | Uses canned `result` when provided, otherwise executes the real tool (default) |

### Configuration

```toml
[tools]
mode = "live"

[tools.Bash]
approve = true

[tools.Read]
approve = true
result = "canned file contents"

[tools.Write]
approve = false
error = "Permission denied"
```

### Per-Tool Settings

| Field | Type | Description |
|-------|------|-------------|
| `approve` | bool | Skip permission prompts |
| `result` | string | Canned result (used in both modes) |
| `error` | string | Simulate error response |
| `answers` | object | Pre-configured answers for AskUserQuestion (keys: question text, values: selected label) |

### AskUserQuestion Answers

The `answers` field provides pre-configured responses for the AskUserQuestion tool. In TUI mode, the elicitation dialog is shown but pre-selects matching answers. In print mode, answers are injected automatically. If no answers are configured, the first option for each question is auto-selected.

```toml
[tools.AskUserQuestion]
approve = true

[tools.AskUserQuestion.answers]
"What language?" = "Rust"
"Which features?" = "Logging, Testing"  # comma-separated for multi-select
```

### AskUserQuestion Tool Result

The tool result contains human-readable summary text in `content` and structured JSON in `toolUseResult`:

```json
{
  "type": "tool_result",
  "content": "User has answered your questions: \"What language?\"=\"Rust\". You can now continue with the user's answers in mind.",
  "toolUseResult": {
    "questions": [...],
    "answers": { "What language?": "Rust" }
  }
}
```

Cancel/rejection behaviors (matching real Claude Code):

- **Escape**: `"User declined to answer questions"` (displayed as response text)
- **Enter on empty "Type something."**: Same as cancel — `"User declined to answer questions"`
- **"Chat about this"**: Rejection with clarification message asking Claude to reformulate the questions

---

## Validation Rules

The system enforces strict validation with clear error messages.

### Session ID

Must be a valid UUID:

```example
Valid:   550e8400-e29b-41d4-a716-446655440000
Invalid: not-a-uuid
Error:   Invalid session_id 'not-a-uuid': must be a valid UUID
```

### Launch Timestamp

Must be ISO 8601 with timezone:

```example
Valid:   2025-01-15T10:30:00Z
Valid:   2025-01-15T10:30:00-08:00
Invalid: 2025-01-15T10:30:00
Error:   Invalid launch_timestamp '...': must be ISO 8601 format
```

### Permission Mode

Must be a recognized value:

```example
Valid:   default, plan, bypass-permissions, accept-edits, dont-ask, delegate
Invalid: invalid-mode
Error:   Invalid permission_mode 'invalid-mode': must be one of [...]
```

### Unknown Fields

Typos in field names are rejected:

```example
Invalid: aprove, moode
Error:   unknown field `aprove`
```

---

## Examples

### Simple Responses

```toml
[[responses]]
on = { contains = "hello" }
say = "Hello! How can I help?"

[[responses]]
on = { regexp = "(?i)fix.*bug" }
say = "I'll help fix that bug."

[[responses]]
on = "*"
say = "I'm not sure what you mean."
```

### Deterministic Testing

```toml
trusted = true

[claude]
session_id = "550e8400-e29b-41d4-a716-446655440000"
launch_timestamp = "2025-01-15T10:30:00Z"
username = "TestUser"

[[responses]]
on = "*"
say = "Deterministic response."
```

### Failure Injection

```toml
[[responses]]
on = { contains = "network" }
failure = { type = "network_unreachable" }

[[responses]]
on = { contains = "timeout" }
failure = { type = "connection_timeout", after_ms = 5000 }

[[responses]]
on = { contains = "rate" }
failure = { type = "rate_limit", retry_after = 60 }

[[responses]]
on = "*"
say = "Normal response."
```

### AskUserQuestion

```toml
[claude]
session_id = "550e8400-e29b-41d4-a716-446655440001"

[[responses]]
on = { contains = "help me" }
say = "Let me ask you a few questions first."

[[responses.tools]]
call = "AskUserQuestion"
input = { questions = [{ question = "What language?", header = "Language", options = [{ label = "Rust", description = "Systems programming" }, { label = "Python", description = "Scripting" }], multiSelect = false }] }

[[responses]]
on = "*"
say = "Got it, I'll use the language you selected."

[tools]
mode = "live"

[tools.AskUserQuestion]
approve = true

[tools.AskUserQuestion.answers]
"What language?" = "Rust"
```

### Full-Featured

```toml
trusted = true
permission_mode = "accept-edits"

[claude]
model = "claude-opus-4-5-20251101"
version = "2.1.12"
username = "Developer"
session_id = "550e8400-e29b-41d4-a716-446655440000"
launch_timestamp = "2025-01-15T10:30:00Z"
working_directory = "/Users/test/project"

[[responses]]
on = { contains = "read file" }
say = "Here's the file content:"
delay_ms = 50

[[responses.tools]]
call = "Read"
input = { file_path = "/src/main.rs" }
result = "fn main() { println!(\"Hello\"); }"

[default]
say = "I can help with that."

[tools]
mode = "mock"

[tools.Read]
approve = true
```

---

## Related Files

| Path | Description |
|------|-------------|
| `scenarios/` | Example scenario files |
| `crates/cli/src/config/` | Configuration type definitions |
| `crates/cli/src/scenario.rs` | Scenario loading and execution |
