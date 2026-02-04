# Scenario Reference

Scenarios define how the Claudeless simulator responds to prompts. They are TOML or JSON files that configure response patterns, failure injection, multi-turn conversations, and tool execution behavior.

## Table of Contents

- [File Format](#file-format)
- [Top-Level Fields](#top-level-fields)
- [Pattern Specifications](#pattern-specifications)
- [Response Specifications](#response-specifications)
- [Failure Injection](#failure-injection)
- [Turn Sequences](#turn-sequences)
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
name = "minimal"

[[responses]]
pattern = { type = "any" }
response = "Hello from Claudeless!"
```

---

## Top-Level Fields

### Identity

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | `""` | Scenario name for logging/debugging |

### Session Identity

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_model` | string | `"claude-opus-4-5-20251101"` | Model to report (overridden by `--model` CLI flag) |
| `claude_version` | string | `"2.1.12"` | Claude version string |
| `user_name` | string | `"Alfred"` | User display name |
| `session_id` | string | (random) | Fixed UUID for deterministic tests |
| `project_path` | string | (cwd) | Override project path |

### Timing

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `launch_timestamp` | string | (now) | Fixed timestamp in ISO 8601 with timezone (e.g., `"2025-01-15T10:30:00Z"`) |

### Timeouts

Configure various timeout values in the `[timeouts]` section:

```toml
[timeouts]
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
| `response_delay_ms` | `CLAUDELESS_RESPONSE_DELAY_MS` | 0 |

**Precedence:** scenario config > environment variable > default

### Environment

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `working_directory` | string | (cwd) | Simulated working directory |
| `trusted` | bool | `true` | Whether directory is trusted |
| `permission_mode` | string | `"default"` | Permission mode override |

**Permission Mode Values:**

| Value | Description |
|-------|-------------|
| `default` | Standard prompts for permissions |
| `plan` | Show plan before executing |
| `bypass-permissions` | Skip all permission checks |
| `accept-edits` | Auto-accept edit permissions |
| `dont-ask` | Auto-approve all permissions |
| `delegate` | Delegate to higher authority |

### Response Configuration

| Field | Type | Description |
|-------|------|-------------|
| `responses` | array | Response rules (evaluated in order) |
| `default_response` | object | Fallback when no pattern matches |
| `tool_execution` | object | Tool execution configuration |

---

## Pattern Specifications

All patterns use a `pattern` field with a `type` discriminator. Rules are evaluated in order; first match wins.

### Exact Match

Case-sensitive exact string match.

```toml
pattern = { type = "exact", text = "hello" }
```

### Regex Match

Full Rust regex syntax.

```toml
pattern = { type = "regex", pattern = "(?i)fix.*bug" }
```

### Glob Match

Shell-style wildcards (`*`, `?`, `[...]`).

```toml
pattern = { type = "glob", pattern = "*.txt" }
```

### Contains Match

Case-sensitive substring match.

```toml
pattern = { type = "contains", text = "error" }
```

### Any Match

Catch-all pattern; matches any input.

```toml
pattern = { type = "any" }
```

---

## Response Specifications

Responses can be simple strings or detailed objects.

> **Note:** For top-level response rules, the `response` field is optional when `failure` is set (failures don't produce responses). For turn sequences, `response` is always required (can be empty string `""`).

### Simple Response

```toml
[[responses]]
pattern = { type = "contains", text = "hello" }
response = "Hello back!"
```

### Detailed Response

```toml
[[responses]]
pattern = { type = "contains", text = "hello" }

[responses.response]
text = "Hello back!"
delay_ms = 100
usage = { input_tokens = 100, output_tokens = 50 }

[[responses.response.tool_calls]]
tool = "Read"
input = { file_path = "/src/main.rs" }
result = "fn main() { ... }"
```

### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `text` | string | Response text |
| `delay_ms` | int | Response delay in milliseconds |
| `tool_calls` | array | Simulated tool calls |
| `usage` | object | Token usage (`input_tokens`, `output_tokens`) |

### Tool Call Fields

| Field | Type | Description |
|-------|------|-------------|
| `tool` | string | Tool name (e.g., `"Read"`, `"Bash"`, `"Write"`) |
| `input` | object | Tool input parameters |
| `result` | string | Canned result (optional) |

### File References

Reference external files using the `$file` key (resolved relative to scenario file). The file contents replace the `$file` object:

```toml
[[responses.response.tool_calls]]
tool = "Write"
input = { file_path = "/tmp/plan.md", content = { "$file" = "fixtures/plan.md" } }
```

For JSON files (`.json` extension), content is parsed as JSON; otherwise loaded as a string.

### Match Limits

Limit how many times a rule can match:

```toml
[[responses]]
pattern = { type = "contains", text = "hello" }
response = "First hello only!"
max_matches = 1
```

### Default Response

Fallback when no pattern matches:

```toml
[default_response]
text = "I'm not sure how to help with that."
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
pattern = { type = "contains", text = "timeout" }
failure = { type = "connection_timeout", after_ms = 100 }

[[responses]]
pattern = { type = "contains", text = "auth" }
failure = { type = "auth_error", message = "API key expired" }

[[responses]]
pattern = { type = "contains", text = "rate" }
failure = { type = "rate_limit", retry_after = 30 }

[[responses]]
pattern = { type = "contains", text = "partial" }
failure = { type = "partial_response", partial_text = "I was about to..." }
```

---

## Turn Sequences

Response rules can have follow-up `turns` for multi-step interactions.

### Basic Turn Sequence

```toml
[[responses]]
pattern = { type = "contains", text = "login" }
response = "Enter username:"
turns = [
    { expect = { type = "any" }, response = "Enter password:" },
    { expect = { type = "any" }, response = "Login successful!" }
]
```

### How Turn Sequences Work

1. When `pattern` matches, return `response` and activate the turn sequence
2. Subsequent prompts match against the current turn's `expect` pattern
3. If turn matches, return its `response` and advance to next turn
4. When all turns complete, sequence deactivates
5. If a turn doesn't match, sequence deactivates and normal matching resumes

### Turn Fields

| Field | Type | Description |
|-------|------|-------------|
| `expect` | pattern | Pattern to match for this turn |
| `response` | string/object | Response for this turn |
| `failure` | object | Optional failure for this turn |

### Turns with Failures

```toml
[[responses]]
pattern = { type = "contains", text = "auth" }
response = "Authenticating..."
turns = [
    { expect = { type = "any" }, response = "", failure = { type = "auth_error", message = "Invalid token" } }
]
```

---

## Tool Execution

Configure how tools are executed during simulation.

### Execution Modes

| Mode | Description |
|------|-------------|
| `mock` | Canned results only; errors if a tool call has no `result` field |
| `live` | Uses canned `result` when provided, otherwise executes the real tool (default) |

### Configuration

```toml
[tool_execution]
mode = "live"

[tool_execution.tools.Bash]
auto_approve = true

[tool_execution.tools.Read]
auto_approve = true
result = "canned file contents"

[tool_execution.tools.Write]
auto_approve = false
error = "Permission denied"
```

### Per-Tool Settings

| Field | Type | Description |
|-------|------|-------------|
| `auto_approve` | bool | Skip permission prompts |
| `result` | string | Canned result (used in both modes) |
| `error` | string | Simulate error response |
| `answers` | object | Pre-configured answers for AskUserQuestion (keys: question text, values: selected label) |

### AskUserQuestion Answers

The `answers` field provides pre-configured responses for the AskUserQuestion tool. In TUI mode, the elicitation dialog is shown but pre-selects matching answers. In print mode, answers are injected automatically. If no answers are configured, the first option for each question is auto-selected.

```toml
[tool_execution.tools.AskUserQuestion]
auto_approve = true

[tool_execution.tools.AskUserQuestion.answers]
"What language?" = "Rust"
"Which features?" = "Logging, Testing"  # comma-separated for multi-select
```

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
Invalid: defualt_model, moode, auto_aprove
Error:   unknown field `defualt_model`
```

---

## Examples

### Simple Responses

```toml
name = "simple"

[[responses]]
pattern = { type = "contains", text = "hello" }
response = "Hello! How can I help?"

[[responses]]
pattern = { type = "regex", pattern = "(?i)fix.*bug" }
response = "I'll help fix that bug."

[[responses]]
pattern = { type = "any" }
response = "I'm not sure what you mean."
```

### Deterministic Testing

```toml
name = "deterministic"
session_id = "550e8400-e29b-41d4-a716-446655440000"
launch_timestamp = "2025-01-15T10:30:00Z"
user_name = "TestUser"
trusted = true

[[responses]]
pattern = { type = "any" }
response = "Deterministic response."
```

### Failure Injection

```toml
name = "failures"

[[responses]]
pattern = { type = "contains", text = "network" }
failure = { type = "network_unreachable" }

[[responses]]
pattern = { type = "contains", text = "timeout" }
failure = { type = "connection_timeout", after_ms = 5000 }

[[responses]]
pattern = { type = "contains", text = "rate" }
failure = { type = "rate_limit", retry_after = 60 }

[[responses]]
pattern = { type = "any" }
response = "Normal response."
```

### AskUserQuestion

```toml
name = "ask-user-question"
session_id = "550e8400-e29b-41d4-a716-446655440001"

[[responses]]
pattern = { type = "contains", text = "help me" }
[responses.response]
text = "Let me ask you a few questions first."
[[responses.response.tool_calls]]
tool = "AskUserQuestion"
[responses.response.tool_calls.input]
questions = [
    { question = "What language?", header = "Language", options = [
        { label = "Rust", description = "Systems programming" },
        { label = "Python", description = "Scripting" },
    ], multiSelect = false },
]

[[responses]]
pattern = { type = "any" }
response = "Got it, I'll use the language you selected."

[tool_execution]
mode = "live"

[tool_execution.tools.AskUserQuestion]
auto_approve = true

[tool_execution.tools.AskUserQuestion.answers]
"What language?" = "Rust"
```

### Full-Featured

```toml
name = "full-featured"
default_model = "claude-opus-4-5-20251101"
claude_version = "2.1.12"
user_name = "Developer"
session_id = "550e8400-e29b-41d4-a716-446655440000"
launch_timestamp = "2025-01-15T10:30:00Z"
working_directory = "/Users/test/project"
trusted = true
permission_mode = "accept-edits"

[[responses]]
pattern = { type = "contains", text = "read file" }

[responses.response]
text = "Here's the file content:"
delay_ms = 50

[[responses.response.tool_calls]]
tool = "Read"
input = { file_path = "/src/main.rs" }
result = "fn main() { println!(\"Hello\"); }"

[default_response]
text = "I can help with that."

[tool_execution]
mode = "mock"

[tool_execution.tools.Read]
auto_approve = true
```

---

## Related Files

| Path | Description |
|------|-------------|
| `scenarios/` | Example scenario files |
| `crates/cli/src/config.rs` | Configuration type definitions |
| `crates/cli/src/scenario.rs` | Scenario loading and execution |
