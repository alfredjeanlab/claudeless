# Epic 7: Scenario Format Enhancement

## Overview

Enhance the TOML/JSON scenario format to support all configuration needed for integration testing. Scenarios control simulator behavior including model selection, timing, user identity, trust settings, and per-tool configuration.

This epic adds:
- **Session identity fields**: `default_model`, `claude_version`, `user_name`, `session_id`, `project_path`
- **Timing control**: `launch_timestamp` for deterministic tests
- **Environment simulation**: `working_directory`, `trusted`, `permission_mode`
- **Per-tool configuration**: `tool_execution.tools.<ToolName>` with `auto_approve` and `result`
- **Strict validation**: Error on unknown fields, type mismatches

**What's NOT in this epic**:
- Hot-reloading scenarios mid-session
- Scenario inheritance/composition
- External scenario repositories

## Project Structure

```
crates/cli/
├── src/
│   ├── config.rs              # UPDATE: Add new scenario fields, per-tool config
│   ├── scenario.rs            # UPDATE: Add validation, apply defaults
│   ├── cli.rs                 # NO CHANGES (CLI already has --session-id, --cwd, --permission-mode)
│   └── session/
│       ├── mod.rs             # NEW: Session context module
│       └── context.rs         # NEW: SessionContext combining scenario + CLI
├── scenarios/
│   ├── full-featured.toml     # NEW: Example with all fields documented
│   └── deterministic.toml     # NEW: Example for reproducible tests
└── tests/
    ├── scenario_fields.rs     # NEW: Tests for new scenario fields
    └── scenario_validation.rs # NEW: Tests for strict validation
```

## Dependencies

No new dependencies required. Uses existing:
- `serde` for serialization with `#[serde(deny_unknown_fields)]`
- `chrono` (already in workspace) for timestamp handling
- `uuid` (already in workspace) for session ID generation

---

## Phase 1: Core Scenario Fields

**Goal**: Add identity, timing, and environment fields to `ScenarioConfig`.

**Fields to add** (`src/config.rs`):

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioConfig {
    // Existing fields
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub default_response: Option<ResponseSpec>,
    #[serde(default)]
    pub responses: Vec<ResponseRule>,
    #[serde(default)]
    pub conversations: HashMap<String, ConversationSpec>,
    #[serde(default)]
    pub tool_execution: Option<ToolExecutionConfig>,

    // NEW: Session identity
    /// Model to report in output (default: "claude-sonnet-4-20250514")
    /// Overridden by --model CLI flag
    #[serde(default)]
    pub default_model: Option<String>,

    /// Claude version string (default: "2.1.12")
    #[serde(default)]
    pub claude_version: Option<String>,

    /// User display name (default: "Alfred")
    #[serde(default)]
    pub user_name: Option<String>,

    /// Fixed session UUID for deterministic file paths (default: random)
    #[serde(default)]
    pub session_id: Option<String>,

    /// Override project path for state directory naming
    #[serde(default)]
    pub project_path: Option<String>,

    // NEW: Timing
    /// Session start time as ISO 8601 (default: current time)
    /// Enables deterministic tests with fixed timestamps
    #[serde(default)]
    pub launch_timestamp: Option<String>,

    // NEW: Environment
    /// Simulated working directory (default: actual cwd)
    #[serde(default)]
    pub working_directory: Option<String>,

    /// Whether directory is trusted (default: true)
    /// When false, TUI shows trust prompt before proceeding
    #[serde(default = "default_trusted")]
    pub trusted: bool,

    /// Permission mode override
    /// Values: "default", "plan", "full-auto", "accept-edits"
    #[serde(default)]
    pub permission_mode: Option<String>,
}

fn default_trusted() -> bool {
    true
}
```

**Default values** (constants at top of `config.rs`):

```rust
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
pub const DEFAULT_CLAUDE_VERSION: &str = "2.1.12";
pub const DEFAULT_USER_NAME: &str = "Alfred";
```

**Verification**:
- TOML/JSON with new fields parses correctly
- Default values applied when fields omitted
- `#[serde(deny_unknown_fields)]` rejects unknown fields
- `cargo test -p claudeless config` passes

---

## Phase 2: Per-Tool Configuration

**Goal**: Add per-tool settings for auto-approve and canned results.

**Config structure** (`src/config.rs`):

```rust
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolExecutionConfig {
    #[serde(default)]
    pub mode: ToolExecutionMode,

    #[serde(default)]
    pub sandbox_root: Option<String>,

    #[serde(default)]
    pub allow_real_bash: bool,

    // NEW: Per-tool settings
    /// Per-tool configuration overrides
    #[serde(default)]
    pub tools: HashMap<String, ToolConfig>,
}

/// Configuration for a specific tool
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolConfig {
    /// Skip permission prompt for this tool
    #[serde(default)]
    pub auto_approve: bool,

    /// Canned result for this tool (overrides execution)
    #[serde(default)]
    pub result: Option<String>,

    /// Simulate error for this tool
    #[serde(default)]
    pub error: Option<String>,
}
```

**Example TOML**:

```toml
[tool_execution]
mode = "simulated"

[tool_execution.tools.Bash]
auto_approve = true

[tool_execution.tools.Read]
auto_approve = true
result = "file contents here"

[tool_execution.tools.Write]
error = "Permission denied"
```

**Integration with executor** (`src/tools/executor.rs`):

```rust
impl PermissionCheckingExecutor {
    pub fn should_auto_approve(&self, tool_name: &str) -> bool {
        self.tool_config
            .get(tool_name)
            .map(|c| c.auto_approve)
            .unwrap_or(false)
    }
}

impl MockExecutor {
    pub fn execute(&self, call: &ToolCallSpec, ctx: &ExecutionContext) -> ToolExecutionResult {
        // Check for per-tool canned result first
        if let Some(config) = ctx.tool_config.get(&call.tool) {
            if let Some(error) = &config.error {
                return ToolExecutionResult::error(error.clone());
            }
            if let Some(result) = &config.result {
                return ToolExecutionResult::success(result.clone());
            }
        }
        // Fall back to call.result or default
        // ...
    }
}
```

**Verification**:
- Per-tool config parses from TOML/JSON
- `auto_approve` skips permission prompts
- `result` overrides tool execution output
- `error` returns tool error
- `cargo test -p claudeless tools` passes

---

## Phase 3: Session Context

**Goal**: Create a unified `SessionContext` that merges scenario config with CLI args.

**New module** (`src/session/context.rs`):

```rust
use crate::cli::Cli;
use crate::config::{ScenarioConfig, DEFAULT_MODEL, DEFAULT_CLAUDE_VERSION, DEFAULT_USER_NAME};
use crate::permission::PermissionMode;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use uuid::Uuid;

/// Merged configuration from scenario + CLI, with defaults applied
#[derive(Clone, Debug)]
pub struct SessionContext {
    pub model: String,
    pub claude_version: String,
    pub user_name: String,
    pub session_id: Uuid,
    pub project_path: PathBuf,
    pub working_directory: PathBuf,
    pub launch_timestamp: DateTime<Utc>,
    pub trusted: bool,
    pub permission_mode: PermissionMode,
}

impl SessionContext {
    /// Build context from scenario and CLI, applying precedence rules:
    /// CLI args > scenario config > defaults
    pub fn build(scenario: Option<&ScenarioConfig>, cli: &Cli) -> Self {
        let model = cli.model.clone(); // CLI always wins for model

        let claude_version = scenario
            .and_then(|s| s.claude_version.clone())
            .unwrap_or_else(|| DEFAULT_CLAUDE_VERSION.to_string());

        let user_name = scenario
            .and_then(|s| s.user_name.clone())
            .unwrap_or_else(|| DEFAULT_USER_NAME.to_string());

        let session_id = cli.session_id.as_ref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .or_else(|| scenario.and_then(|s| s.session_id.as_ref())
                .and_then(|s| Uuid::parse_str(s).ok()))
            .unwrap_or_else(Uuid::new_v4);

        let working_directory = cli.cwd.as_ref()
            .map(PathBuf::from)
            .or_else(|| scenario.and_then(|s| s.working_directory.as_ref()).map(PathBuf::from))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let project_path = scenario
            .and_then(|s| s.project_path.as_ref())
            .map(PathBuf::from)
            .unwrap_or_else(|| working_directory.clone());

        let launch_timestamp = scenario
            .and_then(|s| s.launch_timestamp.as_ref())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let trusted = scenario.map(|s| s.trusted).unwrap_or(true);

        let permission_mode = scenario
            .and_then(|s| s.permission_mode.as_ref())
            .and_then(|s| s.parse().ok())
            .unwrap_or(cli.permission_mode.clone());

        Self {
            model,
            claude_version,
            user_name,
            session_id,
            project_path,
            working_directory,
            launch_timestamp,
            trusted,
            permission_mode,
        }
    }
}
```

**Wire into main** (`src/main.rs`):

```rust
let ctx = SessionContext::build(scenario.as_ref().map(|s| s.config()), &cli);

// Use ctx.model instead of cli.model
// Use ctx.session_id for state directory paths
// Use ctx.trusted to decide if trust prompt needed
```

**Verification**:
- CLI args override scenario values
- Scenario values override defaults
- Session ID is deterministic when specified
- `launch_timestamp` parses ISO 8601 correctly
- `cargo test -p claudeless session` passes

---

## Phase 4: Strict Validation

**Goal**: Fail fast on invalid scenarios with helpful error messages.

**Validation in scenario loading** (`src/scenario.rs`):

```rust
impl Scenario {
    pub fn from_config(config: ScenarioConfig) -> Result<Self, ScenarioError> {
        // Validate session_id format if provided
        if let Some(ref id) = config.session_id {
            if Uuid::parse_str(id).is_err() {
                return Err(ScenarioError::Validation(format!(
                    "Invalid session_id '{}': must be a valid UUID",
                    id
                )));
            }
        }

        // Validate launch_timestamp format if provided
        if let Some(ref ts) = config.launch_timestamp {
            if DateTime::parse_from_rfc3339(ts).is_err() {
                return Err(ScenarioError::Validation(format!(
                    "Invalid launch_timestamp '{}': must be ISO 8601 format",
                    ts
                )));
            }
        }

        // Validate permission_mode if provided
        if let Some(ref mode) = config.permission_mode {
            let valid = ["default", "plan", "full-auto", "accept-edits"];
            if !valid.contains(&mode.as_str()) {
                return Err(ScenarioError::Validation(format!(
                    "Invalid permission_mode '{}': must be one of {:?}",
                    mode, valid
                )));
            }
        }

        // Existing pattern compilation...
    }
}

#[derive(Debug, Error)]
pub enum ScenarioError {
    // Existing variants...

    #[error("Validation error: {0}")]
    Validation(String),
}
```

**Unknown field rejection** is handled by `#[serde(deny_unknown_fields)]` on all config structs.

**Verification**:
- Invalid UUID in `session_id` returns clear error
- Invalid timestamp format returns clear error
- Invalid `permission_mode` returns clear error
- Unknown fields in TOML/JSON return parse error
- `cargo test -p claudeless scenario_validation` passes

---

## Phase 5: Example Scenarios and Documentation

**Goal**: Create example scenarios and document the format.

**Full-featured example** (`scenarios/full-featured.toml`):

```toml
# Full-Featured Scenario Example
# Demonstrates all available configuration options

name = "full-featured-demo"

# Session Identity
default_model = "claude-sonnet-4-20250514"  # Overridden by --model CLI flag
claude_version = "2.1.12"
user_name = "Alfred"
session_id = "550e8400-e29b-41d4-a716-446655440000"  # Fixed UUID for deterministic tests
project_path = "/Users/test/myproject"               # Override for state directory naming

# Timing
launch_timestamp = "2025-01-15T10:30:00Z"  # ISO 8601 for deterministic tests

# Environment
working_directory = "/Users/test/myproject"
trusted = true                              # Set to false to trigger trust prompt
permission_mode = "default"                 # Options: default, plan, full-auto, accept-edits

# Default response when no pattern matches
[default_response]
text = "I'm not sure how to help with that."
delay_ms = 100

# Response rules (evaluated in order)
[[responses]]
pattern = { type = "contains", text = "hello" }
response = "Hello! How can I help you today?"

[[responses]]
pattern = { type = "regex", pattern = "(?i)fix.*bug" }
[responses.response]
text = "I'll help you fix that bug. Let me read the file first."
delay_ms = 200
[[responses.response.tool_calls]]
tool = "Read"
input = { file_path = "/src/main.rs" }

# Tool execution configuration
[tool_execution]
mode = "simulated"
sandbox_root = "/tmp/claudeless-sandbox"

# Per-tool settings
[tool_execution.tools.Bash]
auto_approve = true

[tool_execution.tools.Read]
auto_approve = true

[tool_execution.tools.Write]
auto_approve = false
```

**Deterministic test example** (`scenarios/deterministic.toml`):

```toml
# Deterministic Scenario for Reproducible Tests
# All dynamic values are fixed for consistent test output

name = "deterministic"

# Fixed identity for reproducible state directory paths
session_id = "00000000-0000-0000-0000-000000000001"
project_path = "/test/project"

# Fixed timestamp for reproducible file mtimes
launch_timestamp = "2025-01-01T00:00:00Z"

# Fixed user for consistent display
user_name = "TestUser"

# Trusted to skip trust prompt
trusted = true

# Simple response for testing
default_response = "Test response"
```

**Verification**:
- Example scenarios load without errors
- Examples demonstrate all fields
- `cargo test -p claudeless --test scenario_fields` passes

---

## Key Implementation Details

### Precedence Rules

When the same setting can come from multiple sources:

| Setting | CLI Flag | Scenario Field | Default |
|---------|----------|----------------|---------|
| model | `--model` (wins) | `default_model` | "claude-sonnet-4-20250514" |
| session_id | `--session-id` (wins) | `session_id` | random UUID |
| cwd | `--cwd` (wins) | `working_directory` | actual cwd |
| permission_mode | `--permission-mode` | `permission_mode` | "default" |

### Timestamp Format

`launch_timestamp` uses ISO 8601 / RFC 3339 format:
- `2025-01-15T10:30:00Z` (UTC)
- `2025-01-15T10:30:00-08:00` (with timezone)

### Per-Tool Config vs Response Tool Calls

- `tool_execution.tools.<Name>.result` - Global default for a tool
- `responses[].response.tool_calls[].result` - Specific result for a matched prompt

Response-specific results take precedence over global tool config.

### Deny Unknown Fields

All config structs use `#[serde(deny_unknown_fields)]` to catch typos:

```toml
# This will fail to parse:
[tool_execution]
moode = "mock"  # Typo: should be "mode"
```

---

## Verification Plan

### Unit Tests

| Module | Key Tests |
|--------|-----------|
| `config.rs` | Parse all new fields, defaults applied, deny unknown |
| `scenario.rs` | Validation errors for invalid values |
| `session/context.rs` | Precedence rules, default application |

### Integration Tests

| Test File | Description |
|-----------|-------------|
| `scenario_fields.rs` | Every new field parses and affects behavior |
| `scenario_validation.rs` | Invalid scenarios produce clear errors |

### Test Commands

```bash
# All scenario tests
cargo test -p claudeless scenario

# Just validation tests
cargo test -p claudeless --test scenario_validation

# Run example scenarios
cargo run -p claudeless -- --scenario scenarios/full-featured.toml -p "hello"

# Verify deterministic output
cargo run -p claudeless -- --scenario scenarios/deterministic.toml -p "test" --output-format json
# Should always produce identical session_id in output

# Full CI check
make check
```

### Manual Verification Checklist

- [ ] `full-featured.toml` loads without errors
- [ ] `deterministic.toml` produces identical output on repeated runs
- [ ] Unknown fields in TOML produce clear parse error
- [ ] Invalid UUID in `session_id` produces validation error
- [ ] Invalid timestamp in `launch_timestamp` produces validation error
- [ ] `trusted: false` triggers trust prompt in TUI mode
- [ ] Per-tool `auto_approve` skips permission prompts
- [ ] Per-tool `result` overrides tool execution
