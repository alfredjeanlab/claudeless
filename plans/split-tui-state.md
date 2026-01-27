# Plan: Split TUI State Architecture

## Problem

`TuiAppStateInner` is a god struct with 50+ fields mixing unrelated concerns:
- Input handling (buffer, cursor, undo, stash)
- UI mode and dialogs
- Session/scenario data
- Permission state
- Configuration
- Timing/display state

`RenderState` duplicates 40+ fields, creating a leaky abstraction.

## Files to Modify

- `crates/cli/src/tui/app/state.rs` - Split into focused structs
- `crates/cli/src/tui/app/types.rs` - Simplify RenderState
- `crates/cli/src/tui/app/input.rs` - Use InputState
- `crates/cli/src/tui/app/commands.rs` - Use new state organization
- `crates/cli/src/tui/app/dialogs.rs` - Use DialogState

## Implementation

### Step 1: Define focused state structs

Create `crates/cli/src/tui/app/state/input.rs`:

```rust
/// Input editing state
#[derive(Clone, Debug, Default)]
pub struct InputState {
    /// Current input buffer
    pub buffer: String,
    /// Cursor position
    pub cursor_pos: usize,
    /// Command history
    pub history: Vec<String>,
    /// Current history navigation index
    pub history_index: Option<usize>,
    /// Undo stack for input changes
    pub undo_stack: Vec<String>,
    /// Stashed input for later restoration
    pub stash: Option<String>,
    /// Show stash indicator
    pub show_stash_indicator: bool,
    /// Shell mode active
    pub shell_mode: bool,
}

impl InputState {
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_pos = 0;
        self.undo_stack.clear();
    }

    pub fn submit(&mut self) -> String {
        let input = std::mem::take(&mut self.buffer);
        self.cursor_pos = 0;
        self.undo_stack.clear();
        if !input.is_empty() {
            self.history.push(input.clone());
        }
        self.history_index = None;
        input
    }
}
```

Create `crates/cli/src/tui/app/state/dialog.rs`:

```rust
/// Active dialog state (only one dialog can be active at a time)
#[derive(Clone, Debug, Default)]
pub enum DialogState {
    #[default]
    None,
    Trust(TrustPromptState),
    Thinking(ThinkingDialog),
    Tasks(TasksDialog),
    Export(ExportDialog),
    Help(HelpDialog),
    Hooks(HooksDialog),
    Memory(MemoryDialog),
    ModelPicker(ModelPickerDialog),
    Permission(PermissionRequest),
}

impl DialogState {
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn dismiss(&mut self) {
        *self = Self::None;
    }
}
```

Create `crates/cli/src/tui/app/state/display.rs`:

```rust
/// Display/rendering state
#[derive(Clone, Debug, Default)]
pub struct DisplayState {
    /// Current response content
    pub response_content: String,
    /// Whether response is streaming
    pub is_streaming: bool,
    /// Whether current content is command output
    pub is_command_output: bool,
    /// Conversation history display
    pub conversation_display: String,
    /// Whether conversation was compacted
    pub is_compacted: bool,
    /// Terminal width
    pub terminal_width: u16,
    /// Show shortcuts panel
    pub show_shortcuts_panel: bool,
    /// Slash menu state
    pub slash_menu: Option<SlashMenuState>,
    /// Exit hint
    pub exit_hint: Option<ExitHint>,
    pub exit_hint_shown_at: Option<u64>,
}
```

### Step 2: Refactor TuiAppStateInner

```rust
pub(super) struct TuiAppStateInner {
    // Focused state groups
    pub input: InputState,
    pub dialog: DialogState,
    pub display: DisplayState,

    // Core dependencies (unchanged)
    pub scenario: Arc<Mutex<Scenario>>,
    pub sessions: Arc<Mutex<SessionManager>>,
    pub clock: ClockHandle,
    pub config: TuiConfig,

    // Session state
    pub mode: AppMode,
    pub status: StatusInfo,
    pub permission_mode: PermissionMode,
    pub session_grants: HashSet<SessionPermissionKey>,
    pub trust_granted: bool,
    pub thinking_enabled: bool,

    // Exit state
    pub should_exit: bool,
    pub exit_reason: Option<ExitReason>,
    pub exit_message: Option<String>,

    // Compacting state
    pub is_compacting: bool,
    pub compacting_started: Option<std::time::Instant>,

    // Data
    pub todos: TodoState,
}
```

### Step 3: Simplify RenderState

Instead of copying all fields, derive render-specific views:

```rust
/// Snapshot for rendering - minimal data needed by render functions
pub struct RenderState {
    pub mode: AppMode,
    pub input: InputState,  // Clone of input state
    pub dialog: DialogState, // Clone of dialog state
    pub display: DisplayState, // Clone of display state
    pub status: StatusInfo,
    pub permission_mode: PermissionMode,
    pub thinking_enabled: bool,
    pub user_name: String,
    pub claude_version: Option<String>,
    pub is_tty: bool,
}

impl TuiAppState {
    pub fn render_state(&self) -> RenderState {
        let inner = self.inner.lock();
        RenderState {
            mode: inner.mode.clone(),
            input: inner.input.clone(),
            dialog: inner.dialog.clone(),
            display: inner.display.clone(),
            status: inner.status.clone(),
            permission_mode: inner.permission_mode.clone(),
            thinking_enabled: inner.thinking_enabled,
            user_name: inner.config.user_name.clone(),
            claude_version: inner.config.claude_version.clone(),
            is_tty: inner.config.is_tty,
        }
    }
}
```

### Step 4: Update accessors

Update methods to use new structure:

```rust
// Before
inner.input_buffer.push(c);
inner.cursor_pos += 1;

// After
inner.input.buffer.push(c);
inner.input.cursor_pos += 1;
```

## Migration Strategy

1. Create new state structs alongside existing fields
2. Add deprecation warnings to old field access
3. Migrate one subsystem at a time (input first, then dialog, then display)
4. Remove old fields once all code is migrated

## Testing

- All existing TUI tests must pass
- State cloning behavior unchanged
- Render output identical

## Lines Changed

- ~100 lines restructured in state.rs
- ~50 lines simplified in types.rs
- ~100 lines updated across input.rs, commands.rs, dialogs.rs
- Net: Similar LOC but much better organization
