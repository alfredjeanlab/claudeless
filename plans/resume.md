# Resume Session Implementation Plan

## Overview

Wire up the existing `--resume/-r <session-id>` CLI flag to load and continue an existing session. The infrastructure exists but is not connected:
- CLI accepts the flag (`cli.rs:111-113`)
- `SessionsIndex` tracks sessions in `sessions-index.json`
- `SessionManager::resume()` loads sessions from disk

This plan connects these components in both print mode and TUI mode.

## Project Structure

Key files to modify:
```
crates/cli/src/
├── runtime/
│   ├── builder.rs      # Add resume session loading
│   └── context.rs      # Accept optional session_id for resume
├── main.rs             # Pass resume to TUI SessionManager
└── tui/
    └── app/state.rs    # Initialize with resumed session
```

## Dependencies

No new external dependencies required. Uses existing:
- `uuid` for session ID parsing
- `SessionsIndex`, `SessionManager` from `crate::state`

## Implementation Phases

### Phase 1: Extend RuntimeContext for Resume

**Goal**: Allow `RuntimeContext` to use an existing session ID from `--resume`.

**Changes to `context.rs`**:

```rust
// In RuntimeContext::build_internal(), update session_id resolution:
// Add --resume as highest priority (before --session-id)

let session_id = cli
    .session
    .resume
    .as_ref()
    .and_then(|s| Uuid::parse_str(s).ok())
    .or_else(|| {
        cli.session
            .session_id
            .as_ref()
            .and_then(|s| Uuid::parse_str(s).ok())
    })
    .or_else(|| {
        scenario
            .and_then(|s| s.identity.session_id.as_ref())
            .and_then(|s| Uuid::parse_str(s).ok())
    })
    .unwrap_or_else(Uuid::new_v4);
```

**Verification**: Unit test that `--resume abc123` sets `session_id` to that UUID.

### Phase 2: Validate Resume Session in RuntimeBuilder

**Goal**: Validate that the resumed session exists in `sessions-index.json`.

**Changes to `builder.rs`**:

```rust
// Add new error variant
#[derive(Debug, thiserror::Error)]
pub enum RuntimeBuildError {
    // ... existing variants ...

    #[error("Session not found: {0}")]
    SessionNotFound(String),
}

// Add validation in build() before creating StateWriter:
pub async fn build(self) -> Result<Runtime, RuntimeBuildError> {
    // Validate resume session exists
    if let Some(ref resume_id) = self.cli.session.resume {
        let state_dir = StateDirectory::resolve()
            .map_err(|e| RuntimeBuildError::Validation(e.to_string()))?;

        let working_dir = self.cli.cwd.as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let index_path = state_dir
            .project_dir(&working_dir)
            .join("sessions-index.json");

        if index_path.exists() {
            let index = SessionsIndex::load(&index_path)
                .map_err(|e| RuntimeBuildError::Validation(e.to_string()))?;

            if index.get(resume_id).is_none() {
                return Err(RuntimeBuildError::SessionNotFound(resume_id.clone()));
            }
        } else {
            return Err(RuntimeBuildError::SessionNotFound(resume_id.clone()));
        }
    }

    // ... rest of build() ...
}
```

**Verification**: Test that `--resume nonexistent` returns `SessionNotFound` error.

### Phase 3: Wire Up TUI Mode Resume

**Goal**: When `--resume` is specified, initialize TUI with the resumed session.

**Changes to `main.rs`**:

```rust
async fn run_tui_mode(runtime: Runtime) -> Result<(), Box<dyn std::error::Error>> {
    // ... existing SIGINT handling ...

    let bypass = PermissionBypass::new(/* ... */);
    let is_tty = std::io::stdout().is_terminal();
    let tui_config = TuiConfig::from_runtime(&runtime, bypass.is_active(), is_tty);

    // Create session manager with optional resume
    let mut sessions = SessionManager::new();

    // Check for resume flag
    if let Some(ref resume_id) = runtime.cli().session.resume {
        // Resume existing session (already validated in builder)
        sessions.resume(resume_id);
    }

    let clock = ClockHandle::system();
    let mut app = TuiApp::new(sessions, clock, tui_config, runtime)?;
    // ... rest unchanged ...
}
```

**Verification**: Integration test that TUI resumes session with existing turns visible.

### Phase 4: Wire Up Print Mode Resume

**Goal**: Print mode should append to existing session JSONL.

**Changes to `builder.rs`**:

The current `StateWriter::new()` creates a new session file. For resume, we need to:
1. Use the existing session's JSONL file path
2. Load message count for proper index tracking

```rust
// In build(), modify StateWriter creation:
let state_writer = if !self.cli.session.no_session_persistence {
    let session_id = runtime_ctx.session_id.to_string();

    // Load existing message count if resuming
    let initial_message_count = if self.cli.session.resume.is_some() {
        // Count lines in existing JSONL file
        load_session_message_count(&runtime_ctx.project_path, &session_id)
            .unwrap_or(0)
    } else {
        0
    };

    StateWriter::new_with_count(
        session_id,
        &runtime_ctx.project_path,
        runtime_ctx.launch_timestamp,
        &runtime_ctx.model,
        &runtime_ctx.working_directory,
        initial_message_count,
    )
    .ok()
    .map(|w| Arc::new(RwLock::new(w)))
} else {
    None
};
```

**Changes to `state/mod.rs`** (StateWriter):

```rust
impl StateWriter {
    /// Create a new state writer for resuming a session.
    pub fn new_with_count(
        session_id: impl Into<String>,
        project_path: impl Into<PathBuf>,
        launch_timestamp: DateTime<Utc>,
        model: impl Into<String>,
        cwd: impl Into<PathBuf>,
        message_count: u32,
    ) -> std::io::Result<Self> {
        let mut dir = StateDirectory::resolve()?;
        dir.initialize().map_err(std::io::Error::other)?;

        // Load first_prompt from index if resuming
        let session_id_str = session_id.into();
        let project_path_buf = project_path.into();
        let first_prompt = Self::load_first_prompt(&dir, &project_path_buf, &session_id_str);

        Ok(Self {
            dir,
            session_id: session_id_str,
            project_path: project_path_buf,
            launch_timestamp,
            model: model.into(),
            cwd: cwd.into(),
            first_prompt,
            message_count,
        })
    }

    fn load_first_prompt(
        dir: &StateDirectory,
        project_path: &Path,
        session_id: &str,
    ) -> Option<String> {
        let index_path = dir.project_dir(project_path).join("sessions-index.json");
        SessionsIndex::load(&index_path)
            .ok()
            .and_then(|idx| idx.get(session_id).map(|e| e.first_prompt.clone()))
    }
}
```

**Verification**: Test that print mode with `--resume` appends to existing JSONL.

## Key Implementation Details

### Session ID Priority

The session ID resolution order is:
1. `--resume <id>` - Use existing session
2. `--session-id <id>` - Use specific ID (for new sessions)
3. Scenario config session ID
4. Generate new UUID

### Session File Loading

For TUI mode, `SessionManager::resume()` loads the `.json` session file from storage.

For print mode, `StateWriter` works with `.jsonl` files. Resume appends to the existing file rather than creating a new one.

### Error Handling

- Invalid session ID format: Handled by UUID parsing (generates new ID)
- Session not found: `RuntimeBuildError::SessionNotFound` error before runtime starts
- File read errors: Propagated as `RuntimeBuildError::Validation`

## Verification Plan

### Unit Tests

1. **context_tests.rs**: `test_resume_session_id_priority`
   - Verify `--resume` takes precedence over `--session-id`

2. **builder_tests.rs**: `test_resume_session_not_found`
   - Verify error when resuming nonexistent session

3. **builder_tests.rs**: `test_resume_session_validation`
   - Verify successful validation when session exists

### Integration Tests

1. **tests/resume.rs**: `test_tui_resume_displays_history`
   - Create session, add turns, resume, verify turns visible

2. **tests/resume.rs**: `test_print_resume_appends_to_jsonl`
   - Create session via print mode, resume, verify messages appended

3. **tests/resume.rs**: `test_resume_nonexistent_session_error`
   - Verify CLI exits with error for invalid session ID

### Manual Testing

```bash
# Create a session
claude -p "Hello"

# Get session ID from output or sessions-index.json
SESSION_ID=$(jq -r '.entries[0].sessionId' ~/.claude/projects/.../sessions-index.json)

# Resume the session
claude --resume $SESSION_ID

# Verify conversation continues
```
