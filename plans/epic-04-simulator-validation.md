# Epic 4: Simulator Validation

## Overview

Validate claudeless against real Claude Code behavior and Anthropic documentation. This epic ensures the simulator accurately models CLI flags, output formats, error conditions, and hook protocols before relying on it for integration testing.

This epic transforms the simulator from "working" to "trustworthy" by:
- **CLI flag audit**: Systematically comparing implemented flags against real `claude --help`
- **Output format validation**: Ensuring JSON and stream-JSON output matches real Claude exactly
- **Hook protocol verification**: Testing hook emission against documented behavior
- **State directory validation**: Confirming ~/.claude structure matches real Claude Code
- **Error behavior comparison**: Triggering real errors and comparing against simulator
- **Documentation review**: Cross-referencing with Anthropic's official documentation
- **Discrepancy fixes**: Updating simulator to match observed behavior
- **Test expansion**: Comprehensive tests for every CLI flag and output variation

**What's NOT in this epic**:
- Testing actual LLM response quality (only CLI/protocol behavior)
- Matching internal implementation details (only external behavior)
- Supporting Claude Code features oj doesn't use

## Project Structure

```
crates/cli/
├── src/
│   ├── cli.rs                      # UPDATE: Add missing flags, fix parsing
│   ├── output.rs                   # UPDATE: Fix JSON format discrepancies
│   ├── hooks/protocol.rs           # UPDATE: Fix hook payload structures
│   ├── state/directory.rs          # UPDATE: Fix directory structure
│   └── validation/                 # NEW: Validation infrastructure
│       ├── mod.rs                  # Validation module exports
│       ├── cli_audit.rs            # CLI flag comparison
│       ├── output_samples.rs       # Output format samples
│       └── report.rs               # Accuracy reporting
└── tests/
    ├── cli_flags.rs                # NEW: Comprehensive CLI flag tests
    ├── output_formats.rs           # NEW: Output format validation tests
    ├── hook_protocol.rs            # NEW: Hook protocol tests
    ├── state_directory.rs          # NEW: State directory tests
    ├── error_behavior.rs           # NEW: Error behavior tests
    └── integration_suite.rs        # NEW: E2E integration suite

docs/claudeless/
├── accuracy-report.md              # NEW: Known limitations document
└── validation-methodology.md       # NEW: How validation was performed
```

## Dependencies

```toml
similar = "2"                           # Text diffing for comparison

[dev-dependencies]
insta = { version = "1", features = ["json", "yaml"] }  # Extended snapshot testing
```

---

## Phase 1: CLI Flag Audit and Gap Analysis

**Goal**: Systematically compare `claudeless --help` against `claude --help` and identify all gaps.

**Real Claude CLI Flags Analysis**:

Based on `claude --help` output, here are the flags organized by implementation status:

**Currently Implemented in claudeless**:
- `prompt` (positional) - The prompt text
- `-p, --print` - Non-interactive mode
- `--model` - Model selection
- `--output-format` - text/json/stream-json
- `--system-prompt` - System prompt
- `-c, --continue` - Continue conversation
- `-r, --resume` - Resume by session ID
- `--allowedTools` - Allowed tools list
- `--disallowedTools` - Disallowed tools list
- `--permission-mode` - Permission mode

**Missing - Needed for oj Testing**:
- `--cwd` - Working directory (partially implemented)
- `--max-tokens` - Maximum tokens (used by oj)
- `--input-file` - Read prompt from file
- `--session-id` - Specific session UUID
- `--verbose` - Verbose output mode
- `-d, --debug` - Debug mode with filtering
- `--input-format` - Input format (text/stream-json)
- `--include-partial-messages` - Partial message streaming
- `--fallback-model` - Model fallback on overload
- `--max-budget-usd` - Budget limit

**Missing - Low Priority** (features oj doesn't use):
- `--add-dir` - Additional directories
- `--agent` / `--agents` - Custom agents
- `--betas` - Beta headers
- `--chrome` / `--no-chrome` - Chrome integration
- `--mcp-config` / `--mcp-debug` / `--strict-mcp-config` - MCP servers
- `--plugin-dir` - Plugin directories
- `--tools` - Built-in tool list
- `--json-schema` - Structured output
- `--replay-user-messages` - Re-emit user messages
- `--setting-sources` / `--settings` - Settings loading
- `--fork-session` - Fork session on resume
- `--append-system-prompt` - Append to system prompt
- `--no-session-persistence` - Disable persistence
- `--file` - File resources
- `--ide` - IDE auto-connect
- `--disable-slash-commands` - Disable skills
- `--allow-dangerously-skip-permissions` - Enable bypass option
- `--dangerously-skip-permissions` - Bypass permissions

**Key Types**:

```rust
// src/validation/cli_audit.rs

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlagStatus {
    Implemented,
    Partial(String),
    MissingNeeded,
    MissingLowPriority,
    NotSupported(String),
}

#[derive(Clone, Debug)]
pub struct FlagDef {
    pub name: &'static str,
    pub short: Option<char>,
    pub takes_value: bool,
    pub description: &'static str,
    pub status: FlagStatus,
}

pub struct CliAudit {
    flags: BTreeMap<&'static str, FlagDef>,
}

impl CliAudit {
    pub fn new() -> Self;
    // Populates all known Claude CLI flags with status

    pub fn flags_with_status(&self, status: &FlagStatus) -> Vec<&FlagDef>;
    pub fn to_markdown(&self) -> String;
    // Generates audit report
}
```

```rust
// src/cli.rs (additions for missing flags)

#[derive(Parser, Debug)]
pub struct Cli {
    // ... existing fields ...

    #[arg(long)]
    pub max_tokens: Option<u32>,

    #[arg(long, value_parser = ["text", "stream-json"], default_value = "text")]
    pub input_format: String,

    #[arg(long)]
    pub session_id: Option<String>,

    #[arg(long)]
    pub verbose: bool,

    #[arg(short = 'd', long)]
    pub debug: Option<Option<String>>,

    #[arg(long)]
    pub include_partial_messages: bool,

    #[arg(long)]
    pub fallback_model: Option<String>,

    #[arg(long)]
    pub max_budget_usd: Option<f64>,
}
```

**Verification**:
- `claudeless --help` shows all oj-relevant flags
- All new flags parse correctly
- `cargo test -p claudeless cli` passes with new flag tests

---

## Phase 2: Output Format Validation

**Goal**: Ensure JSON and stream-JSON output formats exactly match real Claude Code output.

**Output Format Analysis**:

Real Claude `--output-format json` produces:
```json
{
  "type": "result",
  "subtype": "success",
  "cost_usd": 0.003,
  "is_error": false,
  "duration_ms": 1234,
  "duration_api_ms": 1100,
  "num_turns": 1,
  "result": "Response text here",
  "session_id": "abc123..."
}
```

Real Claude `--output-format stream-json` produces NDJSON:
```json
{"type":"system","subtype":"init","session_id":"...","tools":[...],"mcp_servers":[...]}
{"type":"assistant","subtype":"message_start","message":{...}}
{"type":"content_block_start","subtype":"text","index":0}
{"type":"content_block_delta","subtype":"text_delta","index":0,"delta":"Hello"}
{"type":"content_block_stop","index":0}
{"type":"assistant","subtype":"message_delta","usage":{...}}
{"type":"assistant","subtype":"message_stop"}
{"type":"result","subtype":"success","cost_usd":0.001,...}
```

**Key Discrepancies to Fix**:

1. **Message structure**: Simulator uses generic `message_start`, real Claude uses `{"type":"assistant","subtype":"message_start"}`
2. **Result wrapper**: Simulator missing `type: "result"` wrapper with cost/duration
3. **Init message**: Simulator missing system init with session_id/tools
4. **Content deltas**: Real Claude includes `index` and `subtype` fields

**Key Types**:

```rust
// src/output.rs (fixes)

#[derive(Clone, Debug, Serialize)]
pub struct ResultOutput {
    #[serde(rename = "type")]
    pub output_type: &'static str,  // "result"
    pub subtype: &'static str,       // "success" or "error"
    pub cost_usd: f64,
    pub is_error: bool,
    pub duration_ms: u64,
    pub duration_api_ms: u64,
    pub num_turns: u32,
    pub result: String,
    pub session_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "system")]
    SystemInit { subtype: &'static str, session_id: String, tools: Vec<String>, mcp_servers: Vec<String> },

    #[serde(rename = "assistant")]
    AssistantMessage { subtype: String, message: Option<AssistantMessageContent>, usage: Option<UsageInfo> },

    #[serde(rename = "content_block_start")]
    ContentBlockStart { subtype: String, index: u32 },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { subtype: String, index: u32, delta: String },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },

    #[serde(rename = "result")]
    Result(ResultOutput),
}

#[derive(Clone, Debug, Serialize)]
pub struct UsageInfo {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
}
```

**Verification**:
- Simulator JSON output matches `REAL_JSON_OUTPUT` structure
- Simulator stream-JSON output matches `REAL_STREAM_JSON_OUTPUT` structure
- `cargo test -p claudeless output` passes with golden tests

---

## Phase 3: Hook Protocol Verification

**Goal**: Verify hook emission matches Claude Code documentation and observed behavior.

**Hook Protocol Analysis**:

Claude Code hooks are configured in `~/.claude/settings.json` or project `.claude/settings.json`:
```json
{
  "hooks": {
    "PreToolExecution": "path/to/script.sh",
    "PostToolExecution": "path/to/script.sh",
    "Notification": "path/to/script.sh"
  }
}
```

Hook scripts receive JSON on stdin:
```json
{
  "event": "PreToolExecution",
  "session_id": "...",
  "payload": {
    "tool_name": "Bash",
    "tool_input": {"command": "ls -la"}
  }
}
```

And respond with JSON on stdout:
```json
{
  "proceed": true,
  "modified_payload": null
}
```

**Verification Tests**:
- `test_pre_tool_execution_payload_matches_spec` - Verifies event, session_id, and payload structure
- `test_hook_response_parsing` - Verifies proceed=true/false and error handling
- `test_notification_payload_matches_spec` - Verifies level/title/message fields

**Verification**:
- Hook messages serialize to documented format
- Hook responses parse correctly
- All hook events emit at correct times
- `cargo test -p claudeless hooks` passes

---

## Phase 4: State Directory Validation

**Goal**: Verify ~/.claude directory structure matches real Claude Code.

**Real ~/.claude Structure**:

```
~/.claude/
├── settings.json                    # Global settings
├── statsig/                         # Analytics (not emulated)
├── projects/
│   └── <project-hash>/
│       ├── settings.json            # Project-specific settings
│       ├── CLAUDE.md                # Project context
│       └── conversation-cache/      # Conversation state
├── todos/
│   └── <session-id>.json            # Todo lists per session
└── ide-clients/                     # IDE integration (not emulated)
```

**Key Code**:

```rust
// src/state/directory.rs (validation additions)

impl StateDirectory {
    pub fn validate_structure(&self) -> Result<Vec<String>, StateError>;
    // Checks required directories exist
    // Validates settings.json is valid JSON
    // Verifies project directories have correct structure
    // Returns list of warnings for any issues

    pub fn project_hash(path: &str) -> String;
    // SHA256 of absolute path, truncated to 16 hex chars
    // Must be deterministic and match real Claude
}
```

**Verification Tests**:
- `test_initialized_structure_matches_real_claude` - Verifies settings.json, projects/, todos/ exist
- `test_project_hash_is_deterministic` - Same path always produces same hash
- `test_settings_json_format` - Valid JSON object format

**Verification**:
- `StateDirectory::initialize()` creates correct structure
- Project hashes match real Claude's algorithm
- Settings files have correct format
- `cargo test -p claudeless state` passes

---

## Phase 5: Error Behavior Comparison

**Goal**: Verify simulator error responses match real Claude Code error behavior.

**Real Error Responses**:

**Authentication Error** (`ANTHROPIC_API_KEY` invalid):
```json
{"type":"result","subtype":"error","cost_usd":0,"is_error":true,"duration_ms":123,"error":"Invalid API key"}
```

**Rate Limit** (429):
```json
{"type":"result","subtype":"error","cost_usd":0,"is_error":true,"duration_ms":50,"error":"Rate limited. Retry after 60 seconds.","retry_after":60}
```

**Network Error**:
```json
{"type":"result","subtype":"error","cost_usd":0,"is_error":true,"duration_ms":5000,"error":"Network error: Connection refused"}
```

**Key Types**:

```rust
// src/failure.rs (error format fixes)

#[derive(Clone, Debug, Serialize)]
pub struct ErrorResult {
    #[serde(rename = "type")]
    pub result_type: &'static str,  // "result"
    pub subtype: &'static str,       // "error"
    pub cost_usd: f64,
    pub is_error: bool,
    pub duration_ms: u64,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
}

impl ErrorResult {
    pub fn auth_error(message: &str) -> Self;
    pub fn rate_limit(retry_after: u64) -> Self;
    pub fn network_error(message: &str) -> Self;
}
```

**Verification**:
- Each failure mode produces correctly formatted output
- Error messages match real Claude phrasing
- Exit codes match real Claude behavior
- `cargo test -p claudeless failure` passes

---

## Phase 6: Comprehensive Test Suite

**Goal**: Create comprehensive test coverage for all validated behaviors.

**Test Files**:

```rust
// tests/cli_flags.rs
// Parameterized tests for every flag combination

#[parameterized(
    print_short = { "-p", true },
    print_long = { "--print", true },
    no_print = { "", false },
)]
fn test_print_flag(args: &str, expected: bool);

#[parameterized(
    text = { "text", "text" },
    json = { "json", "json" },
    stream_json = { "stream-json", "stream-json" },
)]
fn test_output_format(format: &str, expected: &str);

fn test_model_flag();
fn test_session_id_flag();
fn test_max_tokens_flag();
fn test_continue_flag();
fn test_allowed_tools_multiple();
```

```rust
// tests/output_formats.rs

fn test_json_output_matches_golden();
// Verifies type, subtype, is_error, session_id structure

fn test_stream_json_event_sequence();
// Verifies 7-event sequence with valid JSON per line

fn test_stream_json_includes_init_message();
// Verifies system init with session_id and tools
```

```rust
// tests/integration_suite.rs

fn run_sim(args: &[&str]) -> (String, String, i32);
// Helper to run claudeless and capture output

fn test_basic_prompt_returns_response();
fn test_json_output_format();
fn test_auth_error_injection();
fn test_scenario_file_loading();
fn test_state_directory_creation();
```

**Verification**:
- `cargo test -p claudeless --test cli_flags` passes
- `cargo test -p claudeless --test output_formats` passes
- `cargo test -p claudeless --test integration_suite` passes
- All edge cases covered

---

## Phase 7: Accuracy Report and Documentation

**Goal**: Document all findings, known limitations, and validation methodology.

**Accuracy Report** (`docs/claudeless/accuracy-report.md`):

```markdown
# Claudeless Accuracy Report

## Validation Status

### CLI Flags
| Flag | Status | Notes |
|------|--------|-------|
| `-p, --print` | ✅ Match | Exact behavior match |
| `--output-format` | ✅ Match | All three formats supported |
| `--model` | ✅ Match | Accepts all model names |
| `--max-tokens` | ✅ Match | Limits response length |
| `--mcp-config` | ❌ Not supported | Out of scope for oj |

### Output Formats
| Format | Status | Notes |
|--------|--------|-------|
| text | ✅ Match | Plain text output |
| json | ✅ Match | Result wrapper structure |
| stream-json | ✅ Match | NDJSON event sequence |

### Hook Protocol
| Event | Status | Notes |
|-------|--------|-------|
| PreToolExecution | ✅ Match | Payload structure verified |
| PostToolExecution | ✅ Match | Includes tool_output |
| Notification | ✅ Match | level/title/message fields |

### Error Behavior
| Error Type | Status | Notes |
|------------|--------|-------|
| Auth error | ✅ Match | Exit code 1, error JSON |
| Rate limit | ✅ Match | Includes retry_after |
| Network error | ✅ Match | 5s timeout behavior |

## Known Limitations

1. **No actual LLM processing**: All responses are scripted via scenarios
2. **Cost tracking**: Always reports $0.00 cost
3. **Token counting**: Estimates based on character count
4. **MCP servers**: Not emulated (out of scope)
5. **IDE integration**: Not emulated (out of scope)
```

**Validation Methodology** (`docs/claudeless/validation-methodology.md`):

Describes how each component was validated:
1. CLI Flag Audit - captured `claude --help`, categorized, implemented
2. Output Format Validation - captured real output, created golden files
3. Hook Protocol Verification - read docs, tested with real hooks
4. Error Behavior Comparison - triggered each error, captured output

---

## Key Implementation Details

### Output Format Compatibility

The simulator must produce output that can be parsed by the same code that parses real Claude output.

### Golden Test Pattern

Use snapshot testing for output validation:

```rust
#[test]
fn test_json_output_structure() {
    let output = generate_json_output("Hello");
    insta::assert_json_snapshot!(output);
}
```

### Exit Codes

Match real Claude exit codes:

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (auth, network, etc.) |
| 2 | Partial response |
| 130 | Interrupted (Ctrl+C) |

---

## Verification Plan

### Unit Tests

| Module | Key Tests |
|--------|-----------|
| `cli` | All flag parsing, combinations |
| `output` | JSON structure, stream events |
| `hooks/protocol` | Payload serialization |
| `state/directory` | Structure validation |
| `failure` | Error format matching |
| `validation` | Audit completeness |

### Integration Tests

| Test File | Description |
|-----------|-------------|
| `cli_flags.rs` | Every CLI flag combination |
| `output_formats.rs` | Golden output comparison |
| `hook_protocol.rs` | Hook emission and response |
| `state_directory.rs` | Directory lifecycle |
| `error_behavior.rs` | All failure modes |
| `integration_suite.rs` | E2E workflows |

### Manual Verification Checklist

- [ ] `claudeless --help` matches documented flags
- [ ] JSON output parses identically to real Claude
- [ ] Stream-JSON event sequence matches real Claude
- [ ] Hook payloads match documentation
- [ ] Error messages match real Claude phrasing
- [ ] Exit codes match real Claude behavior
- [ ] State directory structure matches ~/.claude

### Test Commands

```bash
# All validation tests
cargo test -p claudeless

# Just CLI flag tests
cargo test -p claudeless --test cli_flags

# Just output format tests
cargo test -p claudeless --test output_formats

# Run full CI check
make check
```

### Comparison Script

For manual comparison against real Claude:

```bash
#!/bin/bash
# scripts/compare-claude.sh

# Capture real Claude output
claude -p --output-format json "Hello" > /tmp/real-claude.json

# Capture simulator output
cargo run -p claudeless -- -p --output-format json "Hello" > /tmp/sim-claude.json

# Compare structure (ignoring dynamic values)
jq 'del(.session_id, .duration_ms, .duration_api_ms, .result)' /tmp/real-claude.json > /tmp/real-struct.json
jq 'del(.session_id, .duration_ms, .duration_api_ms, .result)' /tmp/sim-claude.json > /tmp/sim-struct.json

diff /tmp/real-struct.json /tmp/sim-struct.json && echo "Structures match!"
```
