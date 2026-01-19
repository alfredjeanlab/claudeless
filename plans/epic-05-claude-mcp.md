# Epic 5: MCP-Like Behaviors & Permission Bypass Validation

## Overview

Add and validate support for MCP-like behaviors in claudeless, enabling testing of tool injection patterns that oj will use frequently. This epic also implements and validates `--dangerously-skip-permissions` support, and closes notable test gaps identified during the Epic 4 validation work.

This epic builds on the foundation from Epics 1-4 by adding:
- **MCP Configuration Parsing**: Accept and process `--mcp-config` for tool injection scenarios
- **Simulated MCP Tools**: Framework for injecting custom tools from configuration
- **Permission Bypass Validation**: Full implementation of `--dangerously-skip-permissions` and `--allow-dangerously-skip-permissions`
- **Permission Mode Enforcement**: Wire up the `--permission-mode` flag to actually control behavior
- **Test Gap Closure**: Address methodology gaps in unit and integration tests

**What's NOT in this epic**:
- Full MCP protocol implementation (WebSocket connections, remote servers)
- MCP server discovery or dynamic tool loading
- IDE/Chrome integration
- External MCP server execution

## Project Structure

```
crates/
├── claudeless/
│   ├── Cargo.toml                      # UPDATE: Add MCP config dependencies
│   ├── src/
│   │   ├── lib.rs                      # UPDATE: Export new modules
│   │   ├── main.rs                     # UPDATE: Wire up permission checking
│   │   ├── cli.rs                      # UPDATE: Add permission/MCP flags
│   │   ├── mcp/                        # NEW: MCP simulation module
│   │   │   ├── mod.rs                  # MCP module exports
│   │   │   ├── config.rs               # MCP config file parsing
│   │   │   ├── tools.rs                # Tool definitions from MCP config
│   │   │   └── server.rs               # Simulated MCP server state
│   │   ├── permission/                 # NEW: Permission handling module
│   │   │   ├── mod.rs                  # Permission module exports
│   │   │   ├── mode.rs                 # Permission mode enum and logic
│   │   │   ├── check.rs                # Permission checking logic
│   │   │   └── bypass.rs               # Bypass flag handling
│   │   ├── output.rs                   # UPDATE: Include MCP servers in init
│   │   ├── validation/
│   │   │   ├── cli_audit.rs            # UPDATE: MCP/permission flag status
│   │   │   └── ...
│   │   └── ... (existing files)
│   ├── tests/
│   │   ├── mcp_config.rs               # NEW: MCP configuration tests
│   │   ├── permission_modes.rs         # NEW: Permission mode tests
│   │   ├── permission_bypass.rs        # NEW: Skip-permissions tests
│   │   ├── integration_mcp.rs          # NEW: MCP integration tests
│   │   └── ... (existing tests)
│   └── docs/
│       └── ACCURACY.md                 # UPDATE: MCP/permission status
```

## Dependencies

### Updated Cargo.toml

```toml
[package]
name = "claudeless"
version = "0.1.0"
edition = "2024"

[dependencies]
# Existing dependencies unchanged...

# NEW: MCP config parsing
json5 = "0.4"                           # MCP configs often use JSON5 (comments allowed)

[dev-dependencies]
# Existing dev dependencies...
rstest = "0.25"                         # Parametrized test cases
assert_cmd = "2"                        # CLI integration testing
predicates = "3"                        # Assertion predicates
```

## Implementation Phases

### Phase 1: Permission Bypass Implementation

**Goal**: Implement `--dangerously-skip-permissions` and `--allow-dangerously-skip-permissions` flags with proper behavior matching real Claude.

**Deliverables**:
1. Add permission bypass flags to CLI struct
2. Implement permission mode enum with all real Claude modes
3. Wire permission checking into execution flow
4. Exit with error when `--dangerously-skip-permissions` used without `--allow-dangerously-skip-permissions`
5. Auto-allow all permissions when bypass is active

**Real Claude Behavior Analysis**:

From `claude --help`:
```
--allow-dangerously-skip-permissions    Enable bypassing all permission checks as an option,
                                        without it being enabled by default. Recommended only
                                        for sandboxes with no internet access.
--dangerously-skip-permissions          Bypass all permission checks. Recommended only for
                                        sandboxes with no internet access.
--permission-mode <mode>                Permission mode (choices: "acceptEdits", "bypassPermissions",
                                        "default", "delegate", "dontAsk", "plan")
```

**Key Types**:

```rust
// src/cli.rs - Add permission flags
pub struct Cli {
    #[arg(long, env = "CLAUDE_ALLOW_DANGEROUSLY_SKIP_PERMISSIONS")]
    pub allow_dangerously_skip_permissions: bool,
    #[arg(long, env = "CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS")]
    pub dangerously_skip_permissions: bool,
    #[arg(long, value_enum, default_value = "default")]
    pub permission_mode: PermissionMode,
}

#[derive(Clone, Debug, Default, ValueEnum, PartialEq, Eq)]
pub enum PermissionMode {
    AcceptEdits, BypassPermissions, #[default] Default, Delegate, DontAsk, Plan,
}

// src/permission/bypass.rs
pub enum BypassValidation { Enabled, Disabled, NotAllowed }

pub struct PermissionBypass { allow_bypass: bool, bypass_requested: bool }

impl PermissionBypass {
    pub fn from_cli(cli: &Cli) -> Self;
    pub fn validate(&self) -> BypassValidation;  // (true,true)→Enabled, (true,false)→NotAllowed, (false,_)→Disabled
    pub fn is_active(&self) -> bool;
    pub fn error_message() -> &'static str;
}

// src/permission/check.rs
pub enum PermissionResult {
    Allowed,
    Denied { reason: String },
    NeedsPrompt { tool: String, action: String },
}

pub struct PermissionChecker { mode: PermissionMode, bypass: PermissionBypass }

impl PermissionChecker {
    pub fn new(mode: PermissionMode, bypass: PermissionBypass) -> Self;
    pub fn check(&self, tool_name: &str, action: &str) -> PermissionResult;
    // bypass.is_active() → Allowed; BypassPermissions → Allowed;
    // AcceptEdits + edit action → Allowed; DontAsk/Plan → Denied; else → NeedsPrompt
    pub fn is_bypassed(&self) -> bool;
}

// src/main.rs - Wire permission checking
fn main() {
    let bypass = PermissionBypass::from_cli(&cli);
    if matches!(bypass.validate(), BypassValidation::NotAllowed) { exit(1); }
    let checker = PermissionChecker::new(cli.permission_mode.clone(), bypass);
    // ... use checker for tool execution ...
}
```

**Verification**:
- `claudeless --dangerously-skip-permissions -p "test"` exits with error (missing allow flag)
- `claudeless --allow-dangerously-skip-permissions --dangerously-skip-permissions -p "test"` succeeds
- `claudeless --permission-mode bypassPermissions -p "test"` allows all permissions
- `claudeless --permission-mode plan -p "test"` denies execution
- `cargo test -p claudeless permission` passes

---

### Phase 2: MCP Configuration Parsing

**Goal**: Parse MCP configuration files and extract tool definitions for injection into the simulator.

**Deliverables**:
1. MCP config file format parser (JSON5 for comment support)
2. Tool definition extraction
3. `--mcp-config` flag support (single or multiple configs)
4. `--strict-mcp-config` to ignore other MCP sources
5. Error handling for invalid configs

**MCP Config File Format**:

Based on Claude documentation and MCP protocol spec:

```json5
// ~/.claude/mcp_config.json (or custom path)
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@anthropic-ai/mcp-server-filesystem", "/path/to/allowed/dir"],
      "env": {
        "CUSTOM_VAR": "value"
      }
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@anthropic-ai/mcp-server-github"],
      "env": {
        "GITHUB_TOKEN": "${GITHUB_TOKEN}"
      }
    }
  }
}
```

**Key Types**:

```rust
// src/mcp/config.rs
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfig {
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerDef>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpServerDef {
    pub command: String,
    #[serde(default)] pub args: Vec<String>,
    #[serde(default)] pub env: HashMap<String, String>,
    #[serde(default)] pub cwd: Option<String>,
    #[serde(default = "default_timeout")] pub timeout_ms: u64,  // 30000
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server_name: String,
}

impl McpConfig {
    pub fn load(path: &Path) -> Result<Self, McpConfigError>;
    pub fn parse(content: &str) -> Result<Self, McpConfigError>;  // JSON5 then JSON fallback
    pub fn from_json_str(json: &str) -> Result<Self, McpConfigError>;
    pub fn merge(configs: impl IntoIterator<Item = Self>) -> Self;  // later overrides earlier
    pub fn server_names(&self) -> Vec<&str>;
}

#[derive(Debug, thiserror::Error)]
pub enum McpConfigError { Io(...), Parse(...), InvalidServer(...) }

// src/cli.rs - Add MCP flags
pub struct Cli {
    #[arg(long, value_name = "configs")]
    pub mcp_config: Vec<String>,
    #[arg(long)]
    pub strict_mcp_config: bool,
    #[arg(long)]
    pub mcp_debug: bool,
}
```

**Verification**:
- `cargo test -p claudeless mcp::config` passes
- JSON and JSON5 configs parse correctly
- Multiple `--mcp-config` values merge correctly
- Invalid configs produce helpful errors
- Environment variable syntax preserved in config

---

### Phase 3: Simulated MCP Tools

**Goal**: Implement a framework for simulating tools injected via MCP configuration.

**Deliverables**:
1. `McpServer` type representing a simulated MCP server
2. Tool registry for MCP-provided tools
3. Integration with scenario matching (MCP tools can have scripted responses)
4. Output format updates to include MCP servers in init message
5. Tool availability filtering based on MCP config

**Key Types**:

```rust
// src/mcp/server.rs
#[derive(Clone, Debug)]
pub struct McpServer {
    pub name: String,
    pub definition: McpServerDef,
    pub tools: Vec<McpToolDef>,
    pub status: McpServerStatus,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum McpServerStatus { #[default] Uninitialized, Running, Failed(String), Disconnected }

impl McpServer {
    pub fn from_def(name: impl Into<String>, def: McpServerDef) -> Self;
    pub fn register_tool(&mut self, tool: McpToolDef);
    pub fn start(&mut self);
    pub fn is_running(&self) -> bool;
}

#[derive(Clone, Debug, Default)]
pub struct McpManager {
    servers: HashMap<String, McpServer>,
    tool_server_map: HashMap<String, String>,
}

impl McpManager {
    pub fn new() -> Self;
    pub fn from_config(config: &McpConfig) -> Self;  // auto-starts servers
    pub fn register_tool(&mut self, server_name: &str, tool: McpToolDef);
    pub fn tools(&self) -> Vec<&McpToolDef>;
    pub fn tool_names(&self) -> Vec<String>;
    pub fn server_names(&self) -> Vec<String>;
    pub fn has_tool(&self, name: &str) -> bool;
    pub fn server_for_tool(&self, tool_name: &str) -> Option<&McpServer>;
}

// src/mcp/tools.rs
pub struct McpToolTemplates;
impl McpToolTemplates {
    pub fn filesystem_tools(server_name: &str) -> Vec<McpToolDef>;  // read_file, write_file, list_directory
    pub fn github_tools(server_name: &str) -> Vec<McpToolDef>;      // create_issue, create_pull_request
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpToolCall { pub name: String, pub arguments: Value, pub server: Option<String> }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpToolResult { pub content: Value, pub success: bool, pub error: Option<String> }

// src/output.rs - Add MCP to system init
impl OutputWriter<W> {
    pub fn write_system_init_with_mcp(&mut self, session_id: &str, tools: Vec<String>, mcp_servers: Vec<String>) -> io::Result<()>;
}
```

**Verification**:
- `cargo test -p claudeless mcp` passes
- Tool templates produce valid tool definitions
- MCP servers appear in stream-json init message
- Tool registry correctly maps tools to servers
- Server lifecycle states work correctly

---

### Phase 4: Output and Hook Integration

**Goal**: Wire MCP servers into output formats and hook protocol.

**Deliverables**:
1. System init message includes MCP server names
2. Tool calls can be attributed to MCP servers
3. Hook payloads include MCP context when relevant
4. MCP debug output for troubleshooting

**Key Types**:

```rust
// src/output.rs - Extended system init
#[derive(Clone, Debug, Serialize)]
pub struct SystemInitEvent {
    #[serde(rename = "type")] pub event_type: &'static str,
    pub subtype: &'static str,
    pub session_id: String,
    pub tools: Vec<String>,
    pub mcp_servers: Vec<String>,
}

impl SystemInitEvent {
    pub fn new(session_id: impl Into<String>, builtin_tools: Vec<String>, mcp_manager: &McpManager) -> Self;
    // Combines builtin_tools with mcp_manager.tool_names()
}

// src/hooks/protocol.rs - Add MCP variant
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookPayload {
    // ... existing variants ...
    McpToolExecution {
        tool_name: String,
        server_name: String,
        tool_input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_output: Option<String>,
    },
}
```

**Verification**:
- Stream-JSON output includes `mcp_servers` array
- Tool calls from MCP show server attribution
- Hook payloads distinguish MCP vs built-in tools
- `--mcp-debug` shows server status information

---

### Phase 5: Test Gap Closure

**Goal**: Address test methodology gaps and improve coverage for new and existing functionality.

**Deliverables**:
1. Parametrized tests for all permission modes
2. Integration tests for MCP configuration loading
3. End-to-end tests for permission bypass flows
4. Validation test updates for new flags
5. Test methodology improvements (rstest adoption)

**Key Test Files**:

```rust
// tests/permission_modes.rs - Comprehensive permission mode tests
use assert_cmd::Command;
use predicates::prelude::*;
use rstest::rstest;

#[rstest]
#[case("default", 0)]
#[case("acceptEdits", 0)]
#[case("bypassPermissions", 0)]
#[case("plan", 0)]
#[case("dontAsk", 0)]
#[case("delegate", 0)]
fn test_permission_mode_flag_accepted(#[case] mode: &str, #[case] expected_exit: i32) {
    // claudeless -p --permission-mode {mode} "hello" → exit code {expected_exit}
}

fn test_invalid_permission_mode_rejected() {
    // --permission-mode invalid → failure, stderr contains "invalid"
}

fn test_bypass_without_allow_fails() {
    // --dangerously-skip-permissions alone → failure, stderr mentions --allow-...
}

fn test_bypass_with_allow_succeeds() {
    // --allow-dangerously-skip-permissions --dangerously-skip-permissions → success
}

fn test_allow_without_bypass_is_noop() {
    // --allow-dangerously-skip-permissions alone → success (no effect)
}

fn test_bypass_via_env_vars() {
    // CLAUDE_ALLOW_DANGEROUSLY_SKIP_PERMISSIONS=1 CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS=1 → success
}
```

```rust
// tests/mcp_config.rs - MCP configuration loading tests
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use std::io::Write;

fn write_config(content: &str) -> NamedTempFile { /* write to temp file */ }

fn test_load_mcp_config_from_file() {
    // --mcp-config {file.json} → success, stdout contains "mcp_servers":["test"]
}

fn test_load_mcp_config_inline_json() {
    // --mcp-config '{"mcpServers":{"inline":{"command":"node"}}}' → success
}

fn test_multiple_mcp_configs() {
    // --mcp-config a.json --mcp-config b.json → both servers appear
}

fn test_strict_mcp_config_ignores_defaults() {
    // --strict-mcp-config --mcp-config file.json → only uses specified config
}

fn test_invalid_mcp_config_produces_error() {
    // --mcp-config "not valid json" → failure, stderr contains "parse"
}
```

```rust
// tests/integration_mcp.rs - End-to-end MCP integration tests
use claudeless::mcp::{config::McpConfig, server::McpManager, tools::McpToolTemplates};

fn test_mcp_manager_initialization() {
    let config = McpConfig::parse(r#"{"mcpServers":{"filesystem":{"command":"npx",...}}}"#).unwrap();
    let mut manager = McpManager::from_config(&config);
    for tool in McpToolTemplates::filesystem_tools("filesystem") {
        manager.register_tool("filesystem", tool);
    }
    assert!(manager.has_tool("read_file"));
    assert!(manager.has_tool("write_file"));
    assert!(manager.has_tool("list_directory"));
}

fn test_scenario_with_mcp_tools() {
    let sim = SimulatorBuilder::new()
        .respond_to("read the file", "I'll read that file for you.")
        .with_mcp_config(r#"{"mcpServers":{"fs":{"command":"echo"}}}"#)
        .build_in_process();
    let response = sim.execute("please read the file /tmp/test.txt");
    assert!(response.contains("read that file"));
}
```

**Verification**:
- `cargo test -p claudeless --test permission_modes` passes
- `cargo test -p claudeless --test permission_bypass` passes
- `cargo test -p claudeless --test mcp_config` passes
- `cargo test -p claudeless --test integration_mcp` passes
- All new tests use rstest for parametrization where beneficial

---

### Phase 6: Validation and Documentation

**Goal**: Update accuracy documentation and validation status for new functionality.

**Deliverables**:
1. Updated ACCURACY.md with MCP and permission status
2. Updated cli_audit.rs with new flag status
3. Validation tests for new behaviors
4. README updates for new functionality

**Key Updates**:

```markdown
<!-- docs/ACCURACY.md additions -->

## CLI Flags - Implemented (Match Real Claude)
| `--allow-dangerously-skip-permissions` | Enable bypass option |
| `--dangerously-skip-permissions` | Bypass all permissions (requires allow) |
| `--mcp-config` | Load MCP server configuration |
| `--strict-mcp-config` | Only use specified MCP configs |
| `--mcp-debug` | Show MCP server debug info |

## Permission Modes - All ✅ Match
default, acceptEdits, bypassPermissions, delegate, dontAsk, plan

## MCP Support (Partial)
| Config file parsing | ✅ | JSON and JSON5 |
| Server name in output | ✅ | `mcp_servers` array |
| Tool registration | ✅ | Via config or API |
| Actual server execution | ❌ | Simulated only |
| Dynamic tool discovery | ❌ | Manual registration |
| Server health checks | ❌ | Always "running" |

## Known Limitations
1. MCP servers not actually executed - tools registered manually/via templates
2. Permission prompts simulated - tests use bypass flags
```

```rust
// src/validation/cli_audit.rs - Update flag statuses
impl CliAudit {
    pub fn new() -> Self {
        // Permission flags - NOW IMPLEMENTED
        flags.insert("allow-dangerously-skip-permissions", FlagDef { status: FlagStatus::Implemented, ... });
        flags.insert("dangerously-skip-permissions", FlagDef { status: FlagStatus::Implemented, ... });

        // MCP flags - NOW PARTIAL/IMPLEMENTED
        flags.insert("mcp-config", FlagDef {
            status: FlagStatus::Partial("Config parsing only, no server execution".into()), ...
        });
        flags.insert("strict-mcp-config", FlagDef { status: FlagStatus::Implemented, ... });
        flags.insert("mcp-debug", FlagDef { status: FlagStatus::Implemented, ... });
    }
}
```

**Verification**:
- ACCURACY.md reflects actual implementation status
- CLI audit matches implemented flags
- `cargo test -p claudeless validation` passes
- All validation tests updated for new behaviors

---

## Key Implementation Details

### Permission Flow

```
CLI Args → PermissionBypass → Validate → PermissionChecker → Result
              ↓                   ↓              ↓
        allow + bypass?     NotAllowed?      Mode-based
              ↓                   ↓          decision
          Enabled            Exit(1)            ↓
              ↓                              Allowed/
         All Allowed                         Denied/
                                            NeedsPrompt
```

### MCP Config Resolution

```
1. --mcp-config args (files or inline JSON)
2. Parse each config (JSON5 supported)
3. Merge configs (later overrides earlier)
4. If --strict-mcp-config: skip default locations
5. Otherwise: merge with ~/.claude/mcp_config.json (if exists)
6. Initialize McpManager with merged config
7. Include servers in system init output
```

### Test Methodology Improvements

| Before | After |
|--------|-------|
| Manual test cases | rstest parametrized tests |
| Print-based debugging | assert_cmd predicates |
| Inline JSON strings | NamedTempFile configs |
| Missing edge cases | Comprehensive mode coverage |
| No env var testing | CLAUDE_* env var tests |

### Exit Codes

| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Error (including permission bypass validation failure) |
| 2 | Partial response |
| 130 | Interrupted |

## Verification Plan

### Unit Tests

Run with: `cargo test -p claudeless --lib`

| Module | Key Tests |
|--------|-----------|
| `permission::bypass` | Validation states, error messages |
| `permission::check` | All modes, bypass interaction |
| `mcp::config` | JSON/JSON5 parsing, merge, env vars |
| `mcp::server` | Lifecycle, tool registration |
| `mcp::tools` | Templates, call/result types |
| `validation/cli_audit` | Flag status accuracy |

### Integration Tests

Run with: `cargo test -p claudeless --test '*'`

| Test File | Description |
|-----------|-------------|
| `permission_modes.rs` | All permission modes via CLI |
| `permission_bypass.rs` | Bypass flag combinations |
| `mcp_config.rs` | Config loading scenarios |
| `integration_mcp.rs` | E2E MCP flows |

### Test Commands

```bash
# All tests
cargo test -p claudeless

# Permission tests only
cargo test -p claudeless permission
cargo test -p claudeless --test permission_modes
cargo test -p claudeless --test permission_bypass

# MCP tests only
cargo test -p claudeless mcp
cargo test -p claudeless --test mcp_config
cargo test -p claudeless --test integration_mcp

# Validation tests
cargo test -p claudeless validation

# Full CI check
make check
```

### Manual Verification Checklist

- [ ] `claudeless --help` shows new permission and MCP flags
- [ ] `--dangerously-skip-permissions` alone produces error
- [ ] `--allow-dangerously-skip-permissions --dangerously-skip-permissions` works
- [ ] `--permission-mode bypassPermissions` allows all operations
- [ ] `--permission-mode plan` works without errors
- [ ] `--mcp-config file.json` loads and shows servers in output
- [ ] `--mcp-config '{"mcpServers":...}'` inline JSON works
- [ ] Multiple `--mcp-config` values merge correctly
- [ ] `--strict-mcp-config` ignores default MCP locations
- [ ] `--mcp-debug` produces debug output
- [ ] Invalid MCP config produces helpful error
- [ ] Stream-JSON output includes `mcp_servers` array
- [ ] Environment variables work for all new flags
