# Capture Configuration

Capture specifications define how to record and validate TUI/CLI behavior for integration testing. They support:

- **Key sequences**: Automated keyboard input for TUI testing
- **Expected states**: Conditions to validate during capture
- **Normalization rules**: Transform output for deterministic comparison

## Basic Structure

```toml
name = "my-scenario"

[capture]
name = "capture-name"
capture_type = "tui"  # tui, cli, or dot_claude
timeout_ms = 30000
retry_count = 0

[[capture.key_sequences]]
keys = ["h", "e", "l", "l", "o", "Enter"]

[[capture.normalization_rules]]
type = "strip_ansi"
```

## Capture Types

| Type | Description |
|------|-------------|
| `tui` | Terminal UI mode (ratatui-based) - default |
| `cli` | CLI stdout/stderr capture |
| `dot_claude` | .claude directory state capture |

## Key Sequences

Define keyboard input to send to the TUI:

```toml
[[capture.key_sequences]]
name = "type hello"           # Optional: human-readable name
keys = ["h", "e", "l", "l", "o", "Enter"]
delay_ms = 100                # Optional: delay before sending

[capture.key_sequences.wait_for]
type = "prompt_ready"         # Optional: wait condition
```

### Key Format

Keys use string representation compatible with crossterm:

| Type | Examples |
|------|----------|
| Single characters | `"a"`, `"A"`, `"1"`, `"!"` |
| Named keys | `"Enter"`, `"Escape"`, `"Tab"`, `"Backspace"`, `"Delete"` |
| Arrow keys | `"Up"`, `"Down"`, `"Left"`, `"Right"` |
| Modifiers | `"Ctrl+c"`, `"Alt+Enter"`, `"Shift+Tab"` |
| Function keys | `"F1"`, `"F12"` |

### Wait Conditions

Available `wait_for` conditions:

```toml
# Wait for text pattern to appear
[capture.key_sequences.wait_for]
type = "text_visible"
pattern = "Ready"

# Wait for prompt to be ready for input
[capture.key_sequences.wait_for]
type = "prompt_ready"

# Wait for response to complete
[capture.key_sequences.wait_for]
type = "response_complete"

# Wait for specific UI element
[capture.key_sequences.wait_for]
type = "element_visible"
selector = "#main-input"
```

## Expected States

Validate conditions at specific points during capture:

```toml
[[capture.expected_states]]
name = "check response"       # Optional: name for error reporting
after_sequence = 1            # Optional: validate after this key sequence index

[[capture.expected_states.conditions]]
type = "text_visible"
pattern = "Hello"

[[capture.expected_states.conditions]]
type = "prompt_ready"
```

## Normalization Rules

Transform captured output for deterministic comparison:

### Strip ANSI

Remove ANSI escape codes:

```toml
[[capture.normalization_rules]]
type = "strip_ansi"
```

### Normalize Timestamps

Replace timestamps with placeholder:

```toml
[[capture.normalization_rules]]
type = "normalize_timestamps"
format = "iso8601"  # Optional
```

Matches formats like:
- `2025-01-15T10:30:00Z`
- `2025-01-15 10:30:00`
- `2025-01-15T10:30:00.123+00:00`

### Normalize UUIDs

Replace UUIDs with placeholder:

```toml
[[capture.normalization_rules]]
type = "normalize_uuids"
```

### Normalize Paths

Replace home directory and project paths:

```toml
[[capture.normalization_rules]]
type = "normalize_paths"
base = "/home/user/project"  # Optional: replace with [PROJECT]
```

### Replace Pattern

Replace regex matches with fixed string:

```toml
[[capture.normalization_rules]]
type = "replace"
pattern = "/Users/[^/]+/"
replacement = "/Users/[USER]/"
flags = "i"  # Optional: i = case-insensitive
```

### Remove Lines

Remove lines matching pattern:

```toml
[[capture.normalization_rules]]
type = "remove_lines"
pattern = "^#"  # Remove comment lines
```

## Rule Application Order

Normalization rules are applied in order. Place rules that might affect other patterns first:

```toml
[[capture.normalization_rules]]
type = "strip_ansi"  # First: clean ANSI codes

[[capture.normalization_rules]]
type = "normalize_timestamps"  # Then: normalize data

[[capture.normalization_rules]]
type = "replace"
pattern = "secret-key-[a-z0-9]+"
replacement = "[SECRET]"  # Finally: custom replacements
```

## Complete Example

```toml
name = "interactive-test"
claude_version = "2.1.12"

[capture]
name = "full-capture"
capture_type = "tui"
retry_count = 2
timeout_ms = 30000
output_file = "output.jsonl"

# Send initial prompt
[[capture.key_sequences]]
name = "enter question"
keys = ["W", "h", "a", "t", " ", "i", "s", " ", "2", "+", "2", "?", "Enter"]
delay_ms = 50

[capture.key_sequences.wait_for]
type = "prompt_ready"

# Wait for response
[[capture.key_sequences]]
name = "wait for answer"
keys = []

[capture.key_sequences.wait_for]
type = "response_complete"

# Validate response
[[capture.expected_states]]
name = "answer visible"
after_sequence = 1

[[capture.expected_states.conditions]]
type = "text_visible"
pattern = "4"

# Normalize output
[[capture.normalization_rules]]
type = "strip_ansi"

[[capture.normalization_rules]]
type = "normalize_timestamps"

[[capture.normalization_rules]]
type = "normalize_uuids"

[[capture.normalization_rules]]
type = "normalize_paths"

[[capture.normalization_rules]]
type = "replace"
pattern = "session-[a-f0-9]+"
replacement = "[SESSION]"
```

## Metadata

Store arbitrary metadata for tooling:

```toml
[capture.metadata]
author = "test-suite"
version = 1
tags = ["smoke", "integration"]
```

## Integration with Scenarios

Capture configuration is part of the scenario file:

```toml
# Scenario config
name = "my-test"
claude_version = "2.1.12"

# Response patterns
[[responses]]
pattern = { type = "contains", text = "hello" }
response = "Hi there!"

# Capture config (optional)
[capture]
name = "capture-session"
capture_type = "tui"
# ...
```
