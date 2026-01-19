# Epic 8: State Directory Implementation

## Overview

Wire up the existing `state/` module so claudeless produces the same state files as real Claude Code. The infrastructure exists (`StateDirectory`, `SessionManager`, `TodoState`, `PlansManager`) but isn't connected to the main binary. This epic integrates state directory management into the CLI execution path and updates formats to match real Claude CLI output.

**What's in this epic:**
- Wire `StateDirectory` into `main.rs` execution flow
- Implement `sessions-index.json` in project directories
- Implement session JSONL format (`{uuid}.jsonl`)
- Fix todo file naming to `{sessionId}-agent-{sessionId}.json`
- Fix plan file format to `{adjective}-{verb}-{noun}.md`
- Pass all `dot_claude_*.rs` integration tests

**What's NOT in this epic:**
- settings.json emulation (Epic 10)
- Full JSONL message format complexity (simplified for testing)
- TUI integration with state (Epic 9)

## Project Structure

```
crates/cli/
├── src/
│   ├── main.rs                # UPDATE: Wire StateDirectory into execution
│   ├── state/
│   │   ├── mod.rs             # UPDATE: Add StateWriter facade
│   │   ├── directory.rs       # EXISTING: StateDirectory (minor updates)
│   │   ├── session.rs         # UPDATE: Add JSONL format support
│   │   ├── todos.rs           # UPDATE: Add Claude-compatible output format
│   │   ├── plans.rs           # UPDATE: Add word-based naming, markdown format
│   │   ├── sessions_index.rs  # NEW: sessions-index.json management
│   │   └── words.rs           # NEW: Word lists for plan naming
│   └── tools/
│       └── builtin/
│           └── stateful.rs    # NEW: TodoWrite and ExitPlanMode handlers
├── tests/
│   ├── dot_claude_projects.rs # EXISTING: Should pass after implementation
│   ├── dot_claude_todos.rs    # EXISTING: Should pass after implementation
│   └── dot_claude_plans.rs    # EXISTING: Should pass after implementation
```

## Dependencies

No new external dependencies. Uses existing:
- `uuid` for session IDs
- `chrono` for timestamps
- `serde_json` for JSON/JSONL output
- `rand` (add to workspace) for random word selection

---

## Phase 1: StateWriter Facade

**Goal**: Create a unified interface for writing to the state directory during execution.

The `StateWriter` struct wraps `StateDirectory` and provides high-level methods for the operations main.rs needs.

**New struct** (`src/state/mod.rs`):

```rust
/// Facade for writing Claude state during execution
pub struct StateWriter {
    dir: StateDirectory,
    session_id: String,
    project_path: PathBuf,
    launch_timestamp: DateTime<Utc>,
}

impl StateWriter {
    pub fn new(
        session_id: impl Into<String>,
        project_path: impl Into<PathBuf>,
        launch_timestamp: DateTime<Utc>,
    ) -> std::io::Result<Self> {
        let mut dir = StateDirectory::resolve()?;
        dir.initialize()?;
        Ok(Self {
            dir,
            session_id: session_id.into(),
            project_path: project_path.into(),
            launch_timestamp,
        })
    }

    /// Record a conversation turn (writes to JSONL and updates index)
    pub fn record_turn(
        &self,
        prompt: &str,
        response: &str,
        tool_calls: &[ToolCallSpec],
    ) -> std::io::Result<()>;

    /// Write todo list (called by TodoWrite tool)
    pub fn write_todos(&self, todos: &[TodoItem]) -> std::io::Result<()>;

    /// Create a plan file (called by ExitPlanMode tool)
    pub fn create_plan(&self, content: &str) -> std::io::Result<String>;
}
```

**Wire into main.rs**:

```rust
// After building SessionContext
let state_writer = StateWriter::new(
    ctx.session_id.to_string(),
    ctx.project_path.clone(),
    ctx.launch_timestamp,
)?;

// After getting response
state_writer.record_turn(&prompt, &response_text, &tool_calls)?;
```

**Verification**:
- `StateWriter::new()` creates state directory structure
- Project directory created with normalized name
- `cargo test -p claudeless state` passes

---

## Phase 2: sessions-index.json Format

**Goal**: Implement the sessions-index.json format that tracks all sessions in a project.

**New module** (`src/state/sessions_index.rs`):

```rust
/// Entry in sessions-index.json
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionIndexEntry {
    pub session_id: String,
    pub full_path: String,
    pub file_mtime: u64,
    pub first_prompt: String,
    pub message_count: u32,
    pub created: String,     // ISO 8601
    pub modified: String,    // ISO 8601
    pub git_branch: String,
    pub project_path: String,
    pub is_sidechain: bool,
}

/// sessions-index.json structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionsIndex {
    pub version: u32,
    pub entries: Vec<SessionIndexEntry>,
}

impl SessionsIndex {
    pub fn new() -> Self {
        Self { version: 1, entries: vec![] }
    }

    pub fn load(path: &Path) -> std::io::Result<Self>;
    pub fn save(&self, path: &Path) -> std::io::Result<()>;

    pub fn add_or_update(&mut self, entry: SessionIndexEntry);
    pub fn get(&self, session_id: &str) -> Option<&SessionIndexEntry>;
}
```

**Integration with StateWriter**:

```rust
impl StateWriter {
    fn update_sessions_index(
        &self,
        first_prompt: &str,
        message_count: u32,
    ) -> std::io::Result<()> {
        let index_path = self.dir
            .project_dir(&self.project_path)
            .join("sessions-index.json");

        let mut index = if index_path.exists() {
            SessionsIndex::load(&index_path)?
        } else {
            SessionsIndex::new()
        };

        let entry = SessionIndexEntry {
            session_id: self.session_id.clone(),
            full_path: self.session_jsonl_path().to_string_lossy().into(),
            file_mtime: SystemTime::now()
                .duration_since(UNIX_EPOCH)?.as_millis() as u64,
            first_prompt: first_prompt.to_string(),
            message_count,
            created: self.launch_timestamp.to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            git_branch: get_git_branch().unwrap_or_default(),
            project_path: self.project_path.to_string_lossy().into(),
            is_sidechain: false,
        };

        index.add_or_update(entry);
        index.save(&index_path)
    }
}

fn get_git_branch() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}
```

**Verification**:
- `sessions-index.json` created in project directory
- Contains `version: 1` and `entries` array
- Entry has all required fields with correct types
- `test_sessions_index_json_created` passes

---

## Phase 3: Session JSONL Format

**Goal**: Write session history as JSONL (one JSON object per line) matching Claude CLI format.

**JSONL line types** (each line is a complete JSON object):

```rust
/// User message line
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMessageLine {
    #[serde(rename = "type")]
    pub line_type: &'static str,  // "user"
    pub uuid: String,
    pub session_id: String,
    pub timestamp: String,
    pub cwd: String,
    pub message: UserMessage,
}

#[derive(Serialize)]
pub struct UserMessage {
    pub role: &'static str,  // "user"
    pub content: String,
}

/// Assistant message line
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessageLine {
    #[serde(rename = "type")]
    pub line_type: &'static str,  // "assistant"
    pub uuid: String,
    pub parent_uuid: String,
    pub session_id: String,
    pub timestamp: String,
    pub request_id: String,
    pub message: AssistantMessage,
}

#[derive(Serialize)]
pub struct AssistantMessage {
    pub role: &'static str,  // "assistant"
    pub content: Vec<ContentBlock>,
    pub model: String,
}
```

**Update session.rs** to support JSONL output:

```rust
impl Session {
    /// Append a turn to JSONL file
    pub fn append_turn_jsonl(
        &self,
        path: &Path,
        user_uuid: &str,
        assistant_uuid: &str,
        request_id: &str,
        prompt: &str,
        response: &str,
        model: &str,
        cwd: &str,
    ) -> std::io::Result<()> {
        use std::io::Write;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        // Write user message line
        let user_line = UserMessageLine {
            line_type: "user",
            uuid: user_uuid.to_string(),
            session_id: self.id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            cwd: cwd.to_string(),
            message: UserMessage {
                role: "user",
                content: prompt.to_string(),
            },
        };
        writeln!(file, "{}", serde_json::to_string(&user_line)?)?;

        // Write assistant message line
        let assistant_line = AssistantMessageLine {
            line_type: "assistant",
            uuid: assistant_uuid.to_string(),
            parent_uuid: user_uuid.to_string(),
            session_id: self.id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            request_id: request_id.to_string(),
            message: AssistantMessage {
                role: "assistant",
                content: vec![ContentBlock::Text { text: response.to_string() }],
                model: model.to_string(),
            },
        };
        writeln!(file, "{}", serde_json::to_string(&assistant_line)?)?;

        Ok(())
    }
}
```

**StateWriter integration**:

```rust
impl StateWriter {
    pub fn record_turn(
        &self,
        prompt: &str,
        response: &str,
        model: &str,
        cwd: &str,
    ) -> std::io::Result<()> {
        let project_dir = self.dir.project_dir(&self.project_path);
        std::fs::create_dir_all(&project_dir)?;

        let jsonl_path = project_dir.join(format!("{}.jsonl", self.session_id));

        let user_uuid = Uuid::new_v4().to_string();
        let assistant_uuid = Uuid::new_v4().to_string();
        let request_id = format!("req_{}", Uuid::new_v4().simple());

        // Create session if needed
        let mut session = Session::new(&self.session_id);
        session.append_turn_jsonl(
            &jsonl_path,
            &user_uuid,
            &assistant_uuid,
            &request_id,
            prompt,
            response,
            model,
            cwd,
        )?;

        // Update sessions-index.json
        self.update_sessions_index(prompt, 2)?;  // 2 messages (user + assistant)

        Ok(())
    }
}
```

**Verification**:
- Session file is `.jsonl` not `.json`
- Each line is valid JSON
- User message has required fields
- Assistant message has required fields
- `test_session_file_is_jsonl_format` passes
- `test_session_jsonl_has_user_message` passes
- `test_session_jsonl_has_assistant_message` passes

---

## Phase 4: Todo File Format

**Goal**: Write todos in Claude CLI format with correct naming.

**File naming**: `{sessionId}-agent-{sessionId}.json`

**Content format** (array of objects):

```json
[
  {
    "content": "Build the project",
    "status": "pending",
    "activeForm": "Building the project"
  }
]
```

**Update todos.rs** to support Claude format:

```rust
/// Todo item in Claude CLI format
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeTodoItem {
    pub content: String,
    pub status: String,
    pub active_form: String,
}

impl ClaudeTodoItem {
    pub fn from_todo(item: &TodoItem) -> Self {
        Self {
            content: item.content.clone(),
            status: match item.status {
                TodoStatus::Pending => "pending",
                TodoStatus::InProgress => "in_progress",
                TodoStatus::Completed => "completed",
            }.to_string(),
            active_form: item.active_form.clone()
                .unwrap_or_else(|| format!("{}...", &item.content)),
        }
    }
}

impl TodoState {
    /// Save in Claude CLI format
    pub fn save_claude_format(&self, path: &Path) -> std::io::Result<()> {
        let items: Vec<ClaudeTodoItem> = self.items
            .iter()
            .map(ClaudeTodoItem::from_todo)
            .collect();
        let json = serde_json::to_string_pretty(&items)?;
        std::fs::write(path, json)
    }
}
```

**StateWriter integration**:

```rust
impl StateWriter {
    /// Get todo file path in Claude format
    fn todo_path(&self) -> PathBuf {
        self.dir.todos_dir().join(format!(
            "{}-agent-{}.json",
            self.session_id, self.session_id
        ))
    }

    pub fn write_todos(&self, items: &[TodoItem]) -> std::io::Result<()> {
        std::fs::create_dir_all(self.dir.todos_dir())?;

        let state = TodoState { items: items.to_vec() };
        state.save_claude_format(&self.todo_path())
    }
}
```

**Verification**:
- Todo file name is `{uuid}-agent-{uuid}.json`
- Content is JSON array
- Items have `content`, `status`, `activeForm`
- Status values are snake_case
- `test_todo_file_naming_convention` passes
- `test_todo_file_content_structure` passes

---

## Phase 5: Plan File Format

**Goal**: Write plans as markdown with word-based naming.

**File naming**: `{adjective}-{verb}-{noun}.md` using random word selection.

**Word lists** (`src/state/words.rs`):

```rust
pub const ADJECTIVES: &[&str] = &[
    "velvety", "swirling", "gleaming", "dancing", "quiet",
    "bright", "ancient", "swift", "gentle", "bold",
    "frozen", "golden", "hollow", "eager", "secret",
    "distant", "misty", "tender", "wild", "calm",
];

pub const VERBS: &[&str] = &[
    "crunching", "gliding", "spinning", "weaving", "drifting",
    "singing", "flowing", "growing", "building", "seeking",
    "watching", "waiting", "running", "falling", "rising",
    "turning", "crossing", "finding", "making", "taking",
];

pub const NOUNS: &[&str] = &[
    "ocean", "forest", "mountain", "river", "meadow",
    "valley", "island", "canyon", "desert", "glacier",
    "thunder", "shadow", "crystal", "ember", "garden",
    "harbor", "beacon", "bridge", "tunnel", "tower",
];

pub fn generate_plan_name() -> String {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();

    format!(
        "{}-{}-{}",
        ADJECTIVES.choose(&mut rng).unwrap(),
        VERBS.choose(&mut rng).unwrap(),
        NOUNS.choose(&mut rng).unwrap()
    )
}
```

**Update plans.rs**:

```rust
impl PlansManager {
    /// Create a new plan with generated name
    pub fn create_markdown(&self, content: &str) -> std::io::Result<String> {
        use super::words::generate_plan_name;

        std::fs::create_dir_all(&self.plans_dir)?;

        // Generate unique name (retry if exists)
        let mut name = generate_plan_name();
        let mut attempts = 0;
        while self.plans_dir.join(format!("{}.md", name)).exists() && attempts < 10 {
            name = generate_plan_name();
            attempts += 1;
        }

        let path = self.plans_dir.join(format!("{}.md", name));
        std::fs::write(&path, content)?;

        Ok(name)
    }
}
```

**StateWriter integration**:

```rust
impl StateWriter {
    pub fn create_plan(&self, content: &str) -> std::io::Result<String> {
        let manager = PlansManager::new(self.dir.plans_dir());
        manager.create_markdown(content)
    }
}
```

**Verification**:
- Plan files are `.md` not `.json`
- Names are `{word}-{word}-{word}.md`
- Each word is lowercase alphabetic
- Content is markdown (contains `#`)
- `test_plan_file_naming_convention` passes
- `test_plan_file_is_markdown` passes

---

## Phase 6: Tool Integration

**Goal**: Wire TodoWrite and ExitPlanMode tools to write to state directory.

**New module** (`src/tools/builtin/stateful.rs`):

```rust
use crate::state::StateWriter;
use crate::config::ToolCallSpec;
use crate::tools::{ToolExecutionResult, ToolResultContent};

pub fn execute_todo_write(
    call: &ToolCallSpec,
    state_writer: &StateWriter,
) -> ToolExecutionResult {
    // Parse todo items from call.input
    let todos = match call.input.get("todos") {
        Some(serde_json::Value::Array(arr)) => {
            arr.iter()
                .filter_map(parse_todo_item)
                .collect()
        }
        _ => vec![],
    };

    match state_writer.write_todos(&todos) {
        Ok(()) => ToolExecutionResult {
            tool_use_id: String::new(),  // Set by caller
            content: vec![ToolResultContent::Text {
                text: format!("Updated {} todo(s)", todos.len()),
            }],
            is_error: false,
        },
        Err(e) => ToolExecutionResult::error(format!("Failed to write todos: {}", e)),
    }
}

pub fn execute_exit_plan_mode(
    call: &ToolCallSpec,
    state_writer: &StateWriter,
) -> ToolExecutionResult {
    let content = call.input.get("plan_content")
        .and_then(|v| v.as_str())
        .unwrap_or("# Plan\n\nNo content provided.");

    match state_writer.create_plan(content) {
        Ok(name) => ToolExecutionResult {
            tool_use_id: String::new(),
            content: vec![ToolResultContent::Text {
                text: format!("Plan saved as {}.md", name),
            }],
            is_error: false,
        },
        Err(e) => ToolExecutionResult::error(format!("Failed to save plan: {}", e)),
    }
}
```

**Wire into executor** (`src/tools/executor.rs`):

```rust
impl MockExecutor {
    pub fn execute_with_state(
        &self,
        call: &ToolCallSpec,
        tool_use_id: &str,
        ctx: &ExecutionContext,
        state_writer: Option<&StateWriter>,
    ) -> ToolExecutionResult {
        // Handle stateful tools if state writer available
        if let Some(writer) = state_writer {
            match call.tool.as_str() {
                "TodoWrite" => {
                    let mut result = stateful::execute_todo_write(call, writer);
                    result.tool_use_id = tool_use_id.to_string();
                    return result;
                }
                "ExitPlanMode" => {
                    let mut result = stateful::execute_exit_plan_mode(call, writer);
                    result.tool_use_id = tool_use_id.to_string();
                    return result;
                }
                _ => {}
            }
        }

        // Fall through to existing execution
        self.execute(call, tool_use_id, ctx)
    }
}
```

**Update main.rs** to pass StateWriter to executor:

```rust
// In tool execution loop
for (i, call) in tool_calls.iter().enumerate() {
    let tool_use_id = format!("toolu_{:08x}", i);
    let result = executor.execute_with_state(
        call,
        &tool_use_id,
        &ctx,
        Some(&state_writer),
    );
    writer.write_tool_result(&result)?;
}
```

**Verification**:
- TodoWrite creates todo file in correct location with correct format
- ExitPlanMode creates plan file in correct location with correct format
- All `dot_claude_*.rs` integration tests pass

---

## Key Implementation Details

### State Directory Resolution

```
CLAUDELESS_STATE_DIR env var (explicit)
       ↓ (not set)
tempfile::tempdir() (safety default)
```

The simulator NEVER touches `~/.claude` unless explicitly configured.

### Project Directory Naming

Path `/Users/user/Developer/myproject` becomes `-Users-user-Developer-myproject`:
- Replace `/` with `-`
- Replace `.` with `-`
- Preserves leading `-` for absolute paths

### JSONL vs JSON

| File | Format | Extension |
|------|--------|-----------|
| sessions-index | JSON | `.json` |
| session history | JSONL | `.jsonl` |
| todos | JSON | `.json` |
| plans | Markdown | `.md` |

### Message UUIDs

Each message gets a new UUID. Assistant's `parentUuid` references the preceding user message UUID.

### Plan Name Generation

Random selection from word lists. If name collision, retry up to 10 times.

---

## Verification Plan

### Unit Tests

| Module | Key Tests |
|--------|-----------|
| `sessions_index.rs` | Load/save, add/update entries, JSON format |
| `session.rs` | JSONL append, line format, field presence |
| `todos.rs` | Claude format output, status mapping |
| `plans.rs` | Markdown creation, word-based naming |
| `words.rs` | Word list coverage, name format |

### Integration Tests

| Test File | Description |
|-----------|-------------|
| `dot_claude_projects.rs` | Project dir naming, sessions-index format, JSONL format |
| `dot_claude_todos.rs` | Todo file naming, content structure |
| `dot_claude_plans.rs` | Plan file naming, markdown content |

### Test Commands

```bash
# Run all state tests
cargo test -p claudeless state

# Run integration tests specifically
cargo test -p claudeless --test dot_claude_projects
cargo test -p claudeless --test dot_claude_todos
cargo test -p claudeless --test dot_claude_plans

# Full CI check
make check

# Manual verification
CLAUDELESS_STATE_DIR=/tmp/claude-test cargo run -p claudeless -- \
  --scenario scenarios/deterministic.toml -p "Create a todo list"
ls -la /tmp/claude-test/
```

### Manual Verification Checklist

- [ ] State directory created at `$CLAUDELESS_STATE_DIR` when set
- [ ] State directory created in temp when env var not set
- [ ] Project dir has normalized path name (no `/` or `.`)
- [ ] `sessions-index.json` has version 1 and entries array
- [ ] Session file is `.jsonl` with valid JSON per line
- [ ] User message has uuid, sessionId, timestamp, cwd, message
- [ ] Assistant message has uuid, parentUuid, requestId, model
- [ ] Todo file named `{uuid}-agent-{uuid}.json`
- [ ] Todo items have content, status, activeForm
- [ ] Plan file named `{adj}-{verb}-{noun}.md`
- [ ] Plan content is markdown with heading
- [ ] All `dot_claude_*.rs` tests pass
- [ ] `make check` passes
