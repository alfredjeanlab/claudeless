# Epic 2: Claudeless State

## Overview

Extend claudeless to emulate Claude Code's state management: the `~/.claude` directory structure, permission modes, hook protocols, and session state. This makes the simulator suitable for testing oj's integration with Claude's file-based interfaces.

This epic builds on the foundation from Epic 1 (Claudeless Core) by adding:
- **Directory emulation**: Simulated `~/.claude` structure with todos, projects, plans, and settings
- **Permission modes**: File permissions matching real Claude Code
- **Hook simulation**: Bi-directional communication hooks for tool execution, notifications, and permissions
- **Fake time integration**: Configurable delays without wall-clock time via FakeClock
- **Session state**: Conversation tracking across multi-turn interactions
- **State inspection API**: Test helpers to query and assert on simulator state

**What's NOT in this epic** (handled elsewhere):
- MCP server emulation (out of scope for oj)
- IDE integration features
- Actual persistence across test runs (state is ephemeral)

## Project Structure

```
crates/
├── claudeless/
│   ├── Cargo.toml                  # UPDATE: Add new dependencies
│   ├── src/
│   │   ├── lib.rs                  # UPDATE: Export new modules
│   │   ├── main.rs                 # UPDATE: Initialize state directory
│   │   ├── cli.rs                  # UPDATE: Add state-related flags
│   │   ├── state/                  # NEW: State management module
│   │   │   ├── mod.rs              # State module exports
│   │   │   ├── directory.rs        # ~/.claude directory emulation
│   │   │   ├── todos.rs            # Todo list state
│   │   │   ├── projects.rs         # Per-project context
│   │   │   ├── plans.rs            # Saved plans
│   │   │   ├── settings.rs         # Global settings
│   │   │   └── session.rs          # Session/conversation state
│   │   ├── hooks/                  # NEW: Hook simulation module
│   │   │   ├── mod.rs              # Hook module exports
│   │   │   ├── protocol.rs         # Hook message protocol
│   │   │   ├── executor.rs         # Hook execution
│   │   │   └── registry.rs         # Hook registration
│   │   ├── time.rs                 # NEW: FakeClock integration
│   │   ├── inspect.rs              # NEW: State inspection API
│   │   └── ... (existing files)
│   └── tests/
│       ├── state_directory.rs      # NEW: Directory emulation tests
│       ├── hooks.rs                # NEW: Hook simulation tests
│       ├── session_state.rs        # NEW: Session tracking tests
│       └── ... (existing tests)
```

## Dependencies

### Updated Cargo.toml

```toml
[package]
name = "claudeless"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "claudeless"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "2"
regex = "1"
glob = "0.3"
tokio = { version = "1", features = ["fs", "io-std", "time", "sync", "process"] }
tempfile = "3"
sha2 = "0.10"           # NEW: For project hash generation
hex = "0.4"             # NEW: For hash encoding
parking_lot = "0.12"    # NEW: For efficient locking

[dev-dependencies]
proptest = "1"
yare = "3"
```

## Implementation Phases

### Phase 1: State Directory Structure

**Goal**: Implement the `~/.claude` directory emulation with proper structure and file permissions.

**Deliverables**:
1. `StateDirectory` type managing the simulated `~/.claude` structure
2. Directory initialization creating required subdirectories
3. File permission handling matching real Claude Code
4. Path resolution for all state files
5. Clean state reset between tests

**Key Types**:

```rust
// state/directory.rs
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Failed to create directory: {0}")]
    CreateDir(#[from] std::io::Error),
    #[error("State directory not initialized")]
    NotInitialized,
    #[error("Invalid project path: {0}")]
    InvalidProject(String),
}

/// Simulated ~/.claude directory structure
pub struct StateDirectory {
    root: PathBuf,
    initialized: bool,
}

impl StateDirectory {
    pub fn new(root: impl Into<PathBuf>) -> Self;
    pub fn temp() -> std::io::Result<Self>;  // Creates in tempfile::tempdir()
    pub fn initialize(&mut self) -> Result<(), StateError>;  // Creates todos/, projects/, plans/, settings.json
    pub fn root(&self) -> &Path;
    pub fn todos_dir(&self) -> PathBuf;      // root.join("todos")
    pub fn projects_dir(&self) -> PathBuf;   // root.join("projects")
    pub fn plans_dir(&self) -> PathBuf;      // root.join("plans")
    pub fn settings_path(&self) -> PathBuf;  // root.join("settings.json")
    pub fn project_dir(&self, project_path: &Path) -> PathBuf;  // projects_dir.join(project_hash)
    pub fn reset(&mut self) -> Result<(), StateError>;  // Clears contents, keeps structure
    fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), StateError>;  // Unix 0o700
}

/// Generate deterministic SHA-256 hash for project path (first 16 hex chars)
fn project_hash(path: &Path) -> String;
```

**Verification**:
- `cargo test -p claudeless state::directory` passes
- Directory structure created correctly
- File permissions set correctly on Unix
- Project hash is deterministic
- Reset clears all state
- Temp directory variant works

---

### Phase 2: Session State Management

**Goal**: Implement conversation/session state tracking across multi-turn interactions.

**Deliverables**:
1. `Session` type tracking conversation history
2. Session persistence to state directory
3. Session resume via `-c` / `-r` flags
4. Multi-session support
5. Session expiration/cleanup

**Key Types**:

```rust
// state/session.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Turn {
    pub seq: u32,
    pub prompt: String,
    pub response: String,
    pub timestamp: SystemTime,
    #[serde(default)]
    pub tool_calls: Vec<TurnToolCall>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TurnToolCall {
    pub tool: String,
    pub input: serde_json::Value,
    pub output: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: SystemTime,
    pub last_active: SystemTime,
    pub project_path: Option<String>,
    pub turns: Vec<Turn>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn add_turn(&mut self, prompt: String, response: String) -> &Turn;
    pub fn last_turn(&self) -> Option<&Turn>;
    pub fn is_expired(&self, max_age: Duration) -> bool;
}

pub struct SessionManager {
    sessions: HashMap<String, Session>,
    current: Option<String>,
    storage_dir: Option<PathBuf>,
}

impl SessionManager {
    pub fn new() -> Self;
    pub fn with_storage(self, dir: impl Into<PathBuf>) -> Self;
    pub fn create_session(&mut self) -> &mut Session;  // Generates session_{hex_timestamp} ID
    pub fn current_session(&mut self) -> &mut Session;  // Get or create current
    pub fn resume(&mut self, id: &str) -> Option<&mut Session>;  // By ID, loads from storage if needed
    pub fn continue_session(&mut self) -> Option<&mut Session>;  // Most recent by last_active
    pub fn save_current(&self) -> std::io::Result<()>;  // Saves to storage_dir/{id}.json
    fn load_session(&self, id: &str) -> Option<Session>;
    pub fn clear(&mut self);
}

fn generate_session_id() -> String;  // "session_{hex_millis}"
```

**Verification**:
- `cargo test -p claudeless state::session` passes
- Sessions track conversation turns
- Session resume works with `-r` flag
- Continue works with `-c` flag
- Sessions persist to and load from storage
- Session expiration detection works

---

### Phase 3: Todos and Plans State

**Goal**: Implement todo list and plan state files matching Claude Code's format.

**Deliverables**:
1. `TodoState` type for todo list management
2. `PlanState` type for saved plans
3. File format matching Claude Code
4. Read/write operations
5. Test helpers for asserting state

**Key Types**:

```rust
// state/todos.rs
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus { Pending, InProgress, Completed }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
    #[serde(default)]
    pub priority: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TodoState {
    pub items: Vec<TodoItem>,
}

impl TodoState {
    pub fn new() -> Self;
    pub fn load(path: &Path) -> std::io::Result<Self>;
    pub fn save(&self, path: &Path) -> std::io::Result<()>;
    pub fn add(&mut self, content: impl Into<String>) -> &TodoItem;  // Assigns "todo_{n}" ID
    pub fn set_status(&mut self, id: &str, status: TodoStatus) -> bool;
    pub fn pending(&self) -> impl Iterator<Item = &TodoItem>;
    pub fn in_progress(&self) -> impl Iterator<Item = &TodoItem>;
    pub fn completed(&self) -> impl Iterator<Item = &TodoItem>;
    pub fn clear(&mut self);
}

// state/plans.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub title: String,
    pub content: String,  // Markdown
    pub created_at: SystemTime,
    pub modified_at: SystemTime,
    pub project_path: Option<String>,
}

impl Plan {
    pub fn new(id: impl Into<String>, title: impl Into<String>, content: impl Into<String>) -> Self;
    pub fn load(path: &Path) -> std::io::Result<Self>;
    pub fn save(&self, path: &Path) -> std::io::Result<()>;
}

pub struct PlansManager { plans_dir: PathBuf }

impl PlansManager {
    pub fn new(plans_dir: impl Into<PathBuf>) -> Self;
    pub fn list(&self) -> std::io::Result<Vec<Plan>>;  // All .json files in plans_dir
    pub fn get(&self, id: &str) -> std::io::Result<Option<Plan>>;
    pub fn save(&self, plan: &Plan) -> std::io::Result<()>;
    pub fn delete(&self, id: &str) -> std::io::Result<bool>;
}
```

**Verification**:
- `cargo test -p claudeless state::todos` passes
- `cargo test -p claudeless state::plans` passes
- Todo items have correct status lifecycle
- Plans save and load correctly
- File format matches expected JSON structure

---

### Phase 4: Hook Simulation

**Goal**: Implement hook simulation for bi-directional communication with oj.

**Deliverables**:
1. Hook protocol types (pre/post tool, notification, permission)
2. Hook execution via subprocesses
3. Hook registration and configuration
4. Hook result handling
5. Test helpers for hook assertions

**Key Types**:

```rust
// hooks/protocol.rs
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreToolExecution, PostToolExecution, Notification, PermissionRequest, SessionStart, SessionEnd,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HookMessage {
    pub event: HookEvent,
    pub session_id: String,
    pub payload: HookPayload,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookPayload {
    ToolExecution {
        tool_name: String,
        tool_input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_output: Option<String>,
    },
    Notification { level: NotificationLevel, title: String, message: String },
    Permission { tool_name: String, action: String, context: serde_json::Value },
    Session { project_path: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel { Info, Warning, Error }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HookResponse {
    #[serde(default = "default_proceed")]  // true
    pub proceed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_payload: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl HookResponse {
    pub fn proceed() -> Self;
    pub fn block(reason: impl Into<String>) -> Self;
}

// hooks/executor.rs
#[derive(Clone, Debug)]
pub struct HookConfig {
    pub script_path: PathBuf,
    pub timeout_ms: u64,
    pub blocking: bool,
}

pub struct HookExecutor {
    hooks: HashMap<HookEvent, Vec<HookConfig>>,
}

impl HookExecutor {
    pub fn new() -> Self;
    pub fn register(&mut self, event: HookEvent, config: HookConfig);
    pub async fn execute(&self, message: &HookMessage) -> Result<Vec<HookResponse>, HookError>;
    // Runs hooks sequentially; blocking hook with proceed=false stops chain
    async fn execute_hook(&self, config: &HookConfig, message: &HookMessage) -> Result<HookResponse, HookError>;
    // Spawns script, writes JSON to stdin, reads JSON from stdout (empty = proceed)
    pub fn has_hooks(&self, event: &HookEvent) -> bool;
    pub fn clear(&mut self);
}

#[derive(Debug, thiserror::Error)]
pub enum HookError { Serialization(...), Spawn(...), Io(...), Timeout, InvalidResponse(...) }

// hooks/registry.rs - Test helper for creating hook scripts
pub struct HookRegistry {
    executor: HookExecutor,
    temp_scripts: Vec<tempfile::TempPath>,
}

impl HookRegistry {
    pub fn new() -> Self;
    pub fn register_script(&mut self, event: HookEvent, script_content: &str, blocking: bool) -> io::Result<()>;
    // Creates temp bash script with #!/bin/bash header, chmod 0o755
    pub fn register_passthrough(&mut self, event: HookEvent) -> io::Result<()>;  // echo '{"proceed": true}'
    pub fn register_blocking(&mut self, event: HookEvent, reason: &str) -> io::Result<()>;
    pub fn executor(&self) -> &HookExecutor;
    pub fn executor_mut(&mut self) -> &mut HookExecutor;
}
```

**Verification**:
- `cargo test -p claudeless hooks` passes
- Hook messages serialize correctly
- Hook scripts execute with correct input
- Timeout handling works
- Blocking hooks stop processing
- Registry creates executable temp scripts

---

### Phase 5: FakeClock Integration

**Goal**: Integrate configurable time delays without wall-clock time for deterministic testing.

**Deliverables**:
1. `Clock` trait for time abstraction
2. `FakeClock` implementation with controllable time
3. Integration with response delays
4. Integration with session expiration
5. Test helpers for time manipulation

**Key Types**:

```rust
// time.rs
pub trait Clock: Send + Sync {
    fn now_millis(&self) -> u64;
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send;
}

#[derive(Clone, Debug, Default)]
pub struct SystemClock;
impl Clock for SystemClock {
    // now_millis: SystemTime::now() as millis
    // sleep: tokio::time::sleep(duration).await
}

#[derive(Clone, Debug)]
pub struct FakeClock {
    current_millis: Arc<AtomicU64>,
    auto_advance: bool,
}

impl FakeClock {
    pub fn new(start_millis: u64) -> Self;
    pub fn at_epoch() -> Self;  // Starts at 0
    pub fn at_now() -> Self;    // Starts at current system time
    pub fn set_auto_advance(&mut self, auto_advance: bool);
    pub fn advance(&self, duration: Duration);
    pub fn advance_ms(&self, ms: u64);
    pub fn set(&self, millis: u64);
}

impl Clock for FakeClock {
    // now_millis: returns current_millis
    // sleep: if auto_advance { advance(duration) }; returns immediately (no actual sleep)
}

#[derive(Clone)]
pub enum ClockHandle { System(SystemClock), Fake(FakeClock) }

impl ClockHandle {
    pub fn system() -> Self;
    pub fn fake() -> Self;                    // FakeClock::at_now()
    pub fn fake_at(millis: u64) -> Self;      // FakeClock::new(millis)
    pub fn as_fake(&self) -> Option<&FakeClock>;
}

impl Clock for ClockHandle { /* delegates to inner */ }
```

**Verification**:
- `cargo test -p claudeless time` passes
- FakeClock advances time instantly
- Auto-advance on sleep works
- Manual time manipulation works
- ClockHandle polymorphism works

---

### Phase 6: State Inspection API

**Goal**: Implement test helpers to query and assert on simulator state.

**Deliverables**:
1. `StateInspector` for querying state
2. Assertion helpers for todos, plans, sessions
3. Directory state queries
4. Hook invocation history
5. Integration with existing test API

**Key Types**:

```rust
// inspect.rs
pub struct StateInspector {
    state_dir: Arc<Mutex<StateDirectory>>,
    sessions: Arc<Mutex<SessionManager>>,
    todos: Arc<Mutex<TodoState>>,
    hook_history: Arc<Mutex<Vec<HookMessage>>>,
}

impl StateInspector {
    pub fn new(state_dir: Arc<Mutex<StateDirectory>>, sessions: Arc<Mutex<SessionManager>>, todos: Arc<Mutex<TodoState>>) -> Self;

    // Todo assertions
    pub fn assert_todo_count(&self, expected: usize);
    pub fn assert_pending_count(&self, expected: usize);
    pub fn assert_completed_count(&self, expected: usize);
    pub fn assert_todo_exists(&self, content: &str);  // Substring match
    pub fn assert_todo_status(&self, content: &str, expected_status: TodoStatus);

    // Session assertions
    pub fn assert_session_count(&self, expected: usize);
    pub fn assert_turn_count(&self, expected: usize);  // Current session
    pub fn assert_last_prompt(&self, expected: &str);  // Contains match

    // Hook assertions
    pub fn record_hook(&self, message: HookMessage);
    pub fn assert_hook_invoked(&self, event: HookEvent);
    pub fn assert_hook_count(&self, event: HookEvent, expected: usize);
    pub fn hook_invocations(&self, event: HookEvent) -> Vec<HookMessage>;

    // Directory assertions
    pub fn assert_initialized(&self);  // Checks root, todos, projects, plans dirs exist
    pub fn assert_project_dir_exists(&self, project_path: &Path);

    // Reset
    pub fn reset(&self);  // Clears todos, sessions, hook_history
}

// Extension to SimulatorHandle
impl SimulatorHandle {
    pub fn inspector(&self) -> Option<&StateInspector>;
}
```

**Verification**:
- `cargo test -p claudeless inspect` passes
- Todo assertions work correctly
- Session assertions work correctly
- Hook invocation tracking works
- Directory assertions verify structure
- Reset clears all state

---

## Key Implementation Details

### State Directory Environment Variable

Resolution order: `CLAUDELESS_STATE_DIR` → `$HOME/.claude` → `.claude`

### Hook Script Protocol

Scripts receive JSON on stdin, output JSON on stdout:
```json
// Input
{"event": "pre_tool_execution", "session_id": "session_abc123",
 "payload": {"type": "tool_execution", "tool_name": "Bash", "tool_input": {"command": "ls -la"}}}

// Output
{"proceed": true, "modified_payload": null, "error": null}
```

### Session ID Format

Pattern: `session_<hex_timestamp>` (e.g., `session_18fa3b2c1d0`)
- Deterministic with FakeClock in tests

### Project Hash Generation

SHA-256 of canonical path, first 16 hex chars: `/Users/dev/project` → `a1b2c3d4e5f6g7h8`

### File Permissions (Unix)

- Directories: `0700`
- Files: `0600`
- Settings: `0644`

### Integration with Existing API

```rust
let sim = SimulatorBuilder::new()
    .respond_to("hello", "Hi!")
    .with_state_dir("/tmp/test-claude")
    .with_fake_clock()
    .with_hooks(|registry| {
        registry.register_passthrough(HookEvent::PreToolExecution)?;
        Ok(())
    })
    .build_in_process();

sim.todos().add("Test task");
sim.inspector().assert_todo_count(1);
sim.clock().advance_ms(5000);
sim.execute("hello");
sim.inspector().assert_turn_count(1);
```

## Verification Plan

### Unit Tests

Run with: `cargo test -p claudeless --lib`

| Module | Key Tests |
|--------|-----------|
| `state::directory` | Initialization, path resolution, permissions, reset |
| `state::session` | Creation, turns, resume, expiration, persistence |
| `state::todos` | CRUD operations, status transitions, queries |
| `state::plans` | Save/load, listing, deletion |
| `hooks::protocol` | Serialization, all event types, all payloads |
| `hooks::executor` | Script execution, timeout, blocking |
| `hooks::registry` | Script creation, convenience methods |
| `time` | FakeClock operations, auto-advance, ClockHandle |
| `inspect` | All assertion methods, hook tracking |

### Integration Tests

Run with: `cargo test -p claudeless --test '*'`

| Test | Description |
|------|-------------|
| `state_directory` | End-to-end directory lifecycle |
| `session_state` | Multi-turn conversation tracking |
| `hooks` | Hook execution with real scripts |
| `state_persistence` | Save/load across invocations |

### Test Commands

```bash
# All simulator tests
cargo test -p claudeless

# State module tests only
cargo test -p claudeless state

# Hook tests only
cargo test -p claudeless hooks

# Integration tests
cargo test -p claudeless --test state_directory
cargo test -p claudeless --test session_state
cargo test -p claudeless --test hooks

# Run with state directory
CLAUDELESS_STATE_DIR=/tmp/test-claude ./target/debug/claudeless -p "hello"

# Verify state files created
ls -la /tmp/test-claude/
```

### Manual Verification Checklist

- [ ] State directory created with correct structure
- [ ] File permissions correct on Unix
- [ ] Session continues with `-c` flag
- [ ] Session resumes with `-r` flag
- [ ] Hooks execute and can block
- [ ] FakeClock advances instantly
- [ ] Todo state persists between calls
- [ ] Plans save and load correctly
- [ ] State reset clears everything
- [ ] Inspector assertions pass/fail correctly
