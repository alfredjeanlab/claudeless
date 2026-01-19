# Epic 3: Claudeless TUI

## Overview

Implement a simplified terminal user interface that visually matches Claude Code's TUI rendering. The TUI responds to keyboard input and shortcuts in the same way as real Claude Code, enabling visual and interaction testing of oj's integration with Claude's interactive terminal mode.

This epic builds on the foundation from Epic 1 (core simulation) and Epic 2 (state management) by adding:
- **TUI framework**: Terminal rendering with crossterm and ratatui
- **Visual layout**: Input prompt, response streaming, status bar, tool blocks, permission prompts
- **Keyboard handling**: Ctrl+C, Ctrl+D, Enter, Arrow keys, Escape
- **Permission dialogs**: Interactive tool permission prompts
- **Streaming simulation**: Token-by-token response rendering with configurable speed
- **Screenshot capture**: Terminal state capture for visual regression testing

**What's NOT in this epic** (intentionally simplified):
- Full visual parity (focus on layout, not pixel-perfect styling)
- Mouse input handling
- Clipboard integration
- Syntax highlighting accuracy
- Window resize handling (use fixed dimensions)

## Project Structure

```
crates/
├── claudeless/
│   ├── Cargo.toml                      # UPDATE: Add TUI dependencies
│   ├── src/
│   │   ├── lib.rs                      # UPDATE: Export TUI modules
│   │   ├── main.rs                     # UPDATE: Add TUI mode entry point
│   │   ├── cli.rs                      # UPDATE: Add --tui flag
│   │   ├── tui/                        # NEW: TUI module
│   │   │   ├── mod.rs                  # TUI module exports
│   │   │   ├── app.rs                  # Application state and main loop
│   │   │   ├── layout.rs               # Layout components and rendering
│   │   │   ├── widgets/                # Custom widgets
│   │   │   │   ├── mod.rs              # Widget exports
│   │   │   │   ├── input.rs            # Input prompt widget
│   │   │   │   ├── response.rs         # Response streaming widget
│   │   │   │   ├── status.rs           # Status bar widget
│   │   │   │   ├── tool_block.rs       # Tool use display widget
│   │   │   │   └── permission.rs       # Permission prompt widget
│   │   │   ├── input.rs                # Keyboard input handling
│   │   │   ├── streaming.rs            # Token streaming simulation
│   │   │   ├── screenshot.rs           # Terminal state capture
│   │   │   └── test_helpers.rs         # TUI testing utilities
│   │   └── ... (existing files)
│   └── tests/
│       ├── tui_rendering.rs            # NEW: TUI layout tests
│       ├── tui_input.rs                # NEW: Keyboard handling tests
│       └── tui_integration.rs          # NEW: Full TUI integration tests
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
# Existing dependencies
clap = { version = "4", features = ["derive", "env"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "2"
regex = "1"
glob = "0.3"
tokio = { version = "1", features = ["fs", "io-std", "time", "sync", "process", "rt-multi-thread"] }
tempfile = "3"
sha2 = "0.10"
hex = "0.4"
parking_lot = "0.12"

# NEW: TUI dependencies
crossterm = { version = "0.28", features = ["event-stream"] }
ratatui = "0.29"
unicode-width = "0.2"
textwrap = "0.16"

[dev-dependencies]
proptest = "1"
yare = "3"
insta = { version = "1", features = ["json"] }  # NEW: Snapshot testing
```

## Implementation Phases

### Phase 1: TUI Framework Setup

**Goal**: Set up the terminal rendering infrastructure using crossterm and ratatui, with basic application lifecycle management.

**Deliverables**:
1. Add TUI dependencies to Cargo.toml
2. `TuiApp` struct managing terminal state and main event loop
3. CLI flag `--tui` to enable TUI mode
4. Automatic TUI detection when stdin is a TTY
5. Clean terminal restoration on exit (normal and panic)

**Key Types**:

```rust
// tui/app.rs
pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    mode: AppMode,
    input_buffer: String,
    cursor_pos: usize,
    response_content: String,
    is_streaming: bool,
    status: StatusInfo,
    scenario: Arc<Mutex<Scenario>>,
    sessions: Arc<Mutex<SessionManager>>,
    clock: ClockHandle,
    history: Vec<String>,
    history_index: Option<usize>,
    pending_permission: Option<PermissionRequest>,
    should_exit: bool,
    exit_reason: Option<ExitReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppMode { Input, Responding, Permission, Thinking }

#[derive(Clone, Debug, Default)]
pub struct StatusInfo { pub model: String, pub input_tokens: u32, pub output_tokens: u32, pub session_id: Option<String> }

#[derive(Clone, Debug)]
pub struct PermissionRequest { pub tool_name: String, pub action: String, pub context: String, pub selected: PermissionChoice }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionChoice { Allow, Deny }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExitReason { UserQuit, Interrupted, Completed, Error(String) }

impl TuiApp {
    pub fn new(scenario: Scenario, sessions: SessionManager, clock: ClockHandle) -> io::Result<Self>;
    // enable_raw_mode, EnterAlternateScreen, CrosstermBackend
    pub fn run(&mut self) -> io::Result<ExitReason>;  // Main event loop; installs panic hook to restore terminal
    fn draw(&mut self) -> io::Result<()>;             // Calls layout::render
    fn handle_events(&mut self) -> io::Result<()>;    // Polls events at 50ms interval
    fn handle_key_event(&mut self, key: KeyEvent);    // Dispatches to mode-specific handler
    pub fn exit(&mut self, reason: ExitReason);

    // Accessors
    pub fn mode(&self) -> &AppMode;
    pub fn input_buffer(&self) -> &str;
    pub fn cursor_pos(&self) -> usize;
    pub fn response_content(&self) -> &str;
    pub fn is_streaming(&self) -> bool;
    pub fn status(&self) -> &StatusInfo;
    pub fn pending_permission(&self) -> Option<&PermissionRequest>;
}

impl Drop for TuiApp {
    // disable_raw_mode, LeaveAlternateScreen
}

// cli.rs - Add TUI flags
pub struct Cli {
    #[arg(long, env = "CLAUDELESS_TUI")]
    pub tui: bool,
    #[arg(long)]
    pub no_tui: bool,
}

impl Cli {
    pub fn should_use_tui(&self) -> bool;  // no_tui→false, tui→true, else auto-detect via atty
}

// main.rs - Branch on should_use_tui()
fn run_tui_mode(cli: &Cli) -> anyhow::Result<()>;  // TuiApp::new().run(), exit 130 on Interrupted
fn run_print_mode(cli: &Cli) -> anyhow::Result<()>;
```

**Verification**:
- `cargo build -p claudeless` succeeds with TUI dependencies
- `claudeless --tui` enters TUI mode
- `claudeless --no-tui -p "test"` uses print mode
- Terminal restored correctly on exit
- Terminal restored on panic

---

### Phase 2: Layout and Widget Components

**Goal**: Implement the visual layout matching Claude Code's terminal interface with modular widget components.

**Deliverables**:
1. Main layout dividing screen into regions (input, response, status)
2. Input prompt widget with cursor and typing
3. Response area widget with scrolling
4. Status bar widget showing model, tokens, session
5. Tool use display blocks
6. Permission prompt modal widget

**Key Types**:

```rust
// tui/layout.rs
pub const TUI_WIDTH: u16 = 120;
pub const TUI_HEIGHT: u16 = 40;

pub fn render(frame: &mut Frame, app: &TuiApp) {
    // Layout: response (flexible), input (3 lines), status (1 line)
    // Renders: widgets::response, widgets::input, widgets::status
    // If pending_permission, overlays widgets::permission modal
}

// tui/widgets/input.rs
pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp) {
    // Border: Cyan if Input mode, DarkGray otherwise
    // Prompt: "❯ " (green) + input_buffer
    // Cursor position if active
}

// tui/widgets/response.rs
pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp) {
    // Title: "Claude (thinking...)" | "Claude (streaming...)" | "Claude"
    // Border: Yellow (thinking), Cyan (responding), White (default)
    // Content: response_content + "▌" cursor if streaming
    // Wrap enabled
}

// tui/widgets/status.rs
pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp) {
    // " Model: {model} │ Tokens: {input}↓ {output}↑ │ Session: {id}"
    // DarkGray background
}

// tui/widgets/tool_block.rs
#[derive(Clone, Debug)]
pub struct ToolBlockState {
    pub tool_name: String,
    pub status: ToolStatus,
    pub input_preview: String,
    pub output_preview: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolStatus { Pending, Running, Completed, Failed }

pub fn render(frame: &mut Frame, area: Rect, state: &ToolBlockState) {
    // Icons: ◯ (Yellow), ◐ (Cyan), ✓ (Green), ✗ (Red)
    // Title: " {icon} {tool_name} "
    // Content: input_preview, output_preview
}

// tui/widgets/permission.rs
pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp) {
    // Centered 60x8 modal with Clear background
    // Title: " Permission Required: {tool_name} "
    // Content: action, context, [Allow]/[Deny] buttons
    // Selected button: Green/Red background
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect;
```

**Verification**:
- `cargo test -p claudeless tui::layout` passes
- Layout renders with correct proportions
- Input widget shows cursor position
- Response area wraps long text
- Status bar shows all fields
- Permission modal centers correctly

---

### Phase 3: Keyboard Input Handling

**Goal**: Implement comprehensive keyboard input handling matching Claude Code's shortcuts.

**Deliverables**:
1. Input mode: typing, backspace, delete, cursor movement
2. Ctrl+C for interrupt/cancel
3. Ctrl+D for exit
4. Enter for input submission
5. Arrow keys for history navigation
6. Escape for mode switching
7. Tab for completion (basic)
8. Permission dialog navigation

**Key Types**:

```rust
// tui/input.rs
impl TuiApp {
    pub fn handle_input_key(&mut self, key: KeyEvent) {
        // Ctrl+C: handle_interrupt()
        // Ctrl+D: exit(UserQuit) if empty
        // Ctrl+L: clear response_content
        // Enter: submit_input()
        // Escape: clear input_buffer
        // Backspace/Delete: remove char before/at cursor
        // Left/Right: move cursor
        // Up/Down: navigate_history(-1/+1)
        // Home/Ctrl+A: cursor to start
        // End/Ctrl+E: cursor to end
        // Ctrl+U: clear before cursor
        // Ctrl+K: clear after cursor
        // Ctrl+W: delete_word_before_cursor()
        // Char(c): insert at cursor, reset history_index
    }

    pub fn handle_responding_key(&mut self, key: KeyEvent) {
        // Ctrl+C or Escape: handle_interrupt()
    }

    pub fn handle_permission_key(&mut self, key: KeyEvent) {
        // Left/Right/Tab: toggle Allow/Deny
        // Enter: confirm_permission()
        // Y/y: set Allow, confirm
        // N/n: set Deny, confirm
        // Escape: set Deny, confirm
    }

    fn handle_interrupt(&mut self) {
        // Input mode: clear input or exit if empty
        // Responding/Thinking: cancel, append "[Interrupted]"
        // Permission: deny and confirm
    }

    fn navigate_history(&mut self, direction: i32);  // Up=-1, Down=+1
    fn delete_word_before_cursor(&mut self);         // Ctrl+W behavior
    fn submit_input(&mut self);                      // Add to history, process_prompt
    fn confirm_permission(&mut self);                // Take pending, return to Input, append result
}
```

**Verification**:
- `cargo test -p claudeless tui::input` passes
- All keyboard shortcuts work correctly
- History navigation wraps correctly
- Cursor movement respects bounds
- Ctrl+C interrupts in all modes
- Permission dialog responds to Y/N

---

### Phase 4: Response Streaming Simulation

**Goal**: Implement token-by-token response streaming with configurable speed for realistic simulation.

**Deliverables**:
1. `StreamingResponse` type managing streaming state
2. Configurable streaming speed (tokens per second)
3. Integration with FakeClock for deterministic timing
4. Thinking indicator before streaming starts
5. Proper token counting during streaming

**Key Types**:

```rust
// tui/streaming.rs
#[derive(Clone, Debug)]
pub struct StreamingConfig {
    pub tokens_per_second: u32,   // 0 = instant
    pub thinking_delay_ms: u64,
    pub min_chunk_size: usize,
    pub max_chunk_size: usize,
}

impl StreamingConfig {
    // Default: 50 tps, 500ms thinking, 1-5 chunk
    pub fn instant() -> Self;  // 0 tps, 0ms, 100 chunk
    pub fn slow() -> Self;     // 10 tps, 1000ms, 1-3 chunk
}

pub struct StreamingResponse {
    full_text: String,
    position: usize,
    config: StreamingConfig,
    tokens_streamed: u32,
    complete: bool,
    clock: ClockHandle,
}

impl StreamingResponse {
    pub fn new(text: String, config: StreamingConfig, clock: ClockHandle) -> Self;
    pub async fn next_chunk(&mut self) -> Option<String>;  // Returns chunk, delays if tps > 0
    pub fn tokens_streamed(&self) -> u32;  // ~4 chars per token
    pub fn is_complete(&self) -> bool;
    pub fn full_text(&self) -> &str;
    pub fn skip_to_end(&mut self);  // For interrupt handling
}

pub struct TokenStream { response: StreamingResponse }
impl TokenStream {
    pub fn new(response: StreamingResponse) -> Self;
    pub fn into_channel(self) -> mpsc::Receiver<String>;  // Spawns async task
}

// tui/app.rs - Streaming integration
impl TuiApp {
    pub fn process_prompt(&mut self, prompt: String) {
        // Set Thinking mode, clear response
        // Record turn in session
        // Match scenario for response_text
        // start_streaming(response_text)
    }

    fn start_streaming(&mut self, text: String) {
        // Set Responding mode, is_streaming = true
        // Create StreamingResponse with config and clock
        // Set response_content (sync: full text; async: use TokenStream)
        // Update token counts
        // Update session turn.response
        // Return to Input mode
    }

    pub fn show_permission_request(&mut self, tool_name: String, action: String, context: String) {
        // Set pending_permission, Permission mode
    }
}
```

**Verification**:
- `cargo test -p claudeless tui::streaming` passes
- Streaming produces correct chunks
- Token counting is accurate
- FakeClock integration works (instant in tests)
- Interrupt stops streaming correctly
- Thinking indicator shows before streaming

---

### Phase 5: Screenshot Capture

**Goal**: Implement programmatic capture of terminal state for visual regression testing.

**Deliverables**:
1. `Screenshot` type representing captured terminal state
2. Capture current terminal buffer
3. Text-based output for comparison
4. Integration with insta for snapshot testing
5. Comparison helpers for visual diffs

**Key Types**:

```rust
// tui/screenshot.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Screenshot {
    pub width: u16,
    pub height: u16,
    pub lines: Vec<String>,
    pub metadata: ScreenshotMetadata,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScreenshotMetadata {
    pub timestamp: u64,
    pub mode: String,
    pub label: Option<String>,
}

impl Screenshot {
    pub fn from_buffer(buffer: &Buffer, metadata: ScreenshotMetadata) -> Self;  // Reads cells, trims trailing whitespace
    pub fn to_string(&self) -> String;  // lines.join("\n")
    pub fn diff(&self, other: &Screenshot) -> Vec<LineDiff>;
    pub fn matches(&self, other: &Screenshot) -> bool;
}

#[derive(Clone, Debug)]
pub struct LineDiff { pub line_number: usize, pub expected: String, pub actual: String }

pub struct ScreenshotCapture {
    terminal: Terminal<TestBackend>,
    captures: Vec<Screenshot>,
}

impl ScreenshotCapture {
    pub fn new(width: u16, height: u16) -> Self;
    pub fn capture<F>(&mut self, render_fn: F, label: Option<&str>) -> Screenshot where F: FnOnce(&mut Frame);
    pub fn captures(&self) -> &[Screenshot];
    pub fn last(&self) -> Option<&Screenshot>;
    pub fn clear(&mut self);
}

#[macro_export]
macro_rules! assert_screenshot { /* compares lines with trim_end */ }

// tui/test_helpers.rs
pub struct TuiAppState {
    pub mode: AppMode,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub response_content: String,
    pub is_streaming: bool,
    pub history: Vec<String>,
    pub should_exit: bool,
    pub exit_reason: Option<ExitReason>,
}

pub struct TuiTestHarness {
    capture: ScreenshotCapture,
    clock: FakeClock,
    app_state: TuiAppState,
}

impl TuiTestHarness {
    pub fn new() -> Self;  // 120x40, FakeClock::at_epoch(), Input mode
    pub fn type_input(&mut self, text: &str);
    pub fn press_enter(&mut self);  // Moves input to history
    pub fn press_ctrl_c(&mut self);  // Clears input or exits if empty
    pub fn set_response(&mut self, content: &str);
    pub fn capture(&mut self, label: Option<&str>) -> Screenshot;  // Renders simplified layout
    pub fn clock(&self) -> &FakeClock;
    pub fn advance_time(&self, ms: u64);
}
```

**Verification**:
- `cargo test -p claudeless tui::screenshot` passes
- Screenshots capture correct content
- Diff detection works correctly
- Snapshot testing integration works
- Test harness simulates input correctly

---

### Phase 6: Integration and CLI Wiring

**Goal**: Wire up all TUI components and integrate with the main CLI entry point.

**Deliverables**:
1. Complete TUI module exports in lib.rs
2. CLI --tui flag integration in main.rs
3. TTY detection for automatic TUI mode
4. Scenario integration with TUI
5. Session state persistence
6. End-to-end integration tests

**Key Code**:

```rust
// tui/mod.rs
mod app; mod input; mod layout; mod screenshot; mod streaming; mod test_helpers;
pub mod widgets;

pub use app::{AppMode, ExitReason, PermissionChoice, PermissionRequest, StatusInfo, TuiApp};
pub use screenshot::{LineDiff, Screenshot, ScreenshotCapture, ScreenshotMetadata};
pub use streaming::{StreamingConfig, StreamingResponse, TokenStream};
pub use test_helpers::{TuiAppState, TuiTestHarness};

// tui/widgets/mod.rs
pub mod input; pub mod permission; pub mod response; pub mod status; pub mod tool_block;
pub use tool_block::{ToolBlockState, ToolStatus};

// lib.rs
pub mod tui;
pub use tui::{TuiApp, TuiTestHarness};

// main.rs - should_use_tui() branching, load_scenario()
```

**Integration Tests** (`tests/tui_integration.rs`):
- `test_basic_input_flow`: Initial state, type_input, capture screenshot
- `test_ctrl_c_clears_input`: Clears buffer but doesn't exit
- `test_ctrl_c_exits_on_empty`: Exits with Interrupted
- `test_history_navigation`: Enter adds to history
- `test_screenshot_capture`: set_response, capture, verify content

**Verification**:
- `cargo build -p claudeless` succeeds
- `cargo test -p claudeless tui` passes
- `claudeless --tui` enters TUI mode
- TTY detection works correctly
- Integration tests pass
- Screenshots capture expected content

---

## Key Implementation Details

### TTY Detection
`should_use_tui()`: no_tui→false, tui→true, else `!print && atty::is(Stdin)`

### Terminal Restoration
- `Drop for TuiApp`: disable_raw_mode, LeaveAlternateScreen
- Panic hook: same restoration before calling original hook

### Fixed Dimensions
`TUI_WIDTH: 120`, `TUI_HEIGHT: 40` (no resize handling)

### Streaming Speed Control
- Default: 50 tps, 500ms thinking, 1-5 chunk size
- Tests: `StreamingConfig::instant()` (0 tps, no delays)

### Exit Codes
| 0 | Completed/UserQuit | 1 | Error | 130 | Interrupted (Ctrl+C) |

### Integration with Simulator Core
TUI reuses `scenario.match_prompt()` for response matching

## Verification Plan

### Unit Tests

Run with: `cargo test -p claudeless tui --lib`

| Module | Key Tests |
|--------|-----------|
| `tui::app` | Initialization, mode transitions, exit handling |
| `tui::input` | All keyboard shortcuts, cursor movement, history |
| `tui::layout` | Layout proportions, widget placement |
| `tui::streaming` | Chunk generation, timing, token counting |
| `tui::screenshot` | Capture, diff, comparison |
| `tui::widgets::*` | Individual widget rendering |

### Integration Tests

Run with: `cargo test -p claudeless --test 'tui_*'`

| Test File | Description |
|-----------|-------------|
| `tui_rendering.rs` | Layout and widget rendering |
| `tui_input.rs` | Keyboard input handling end-to-end |
| `tui_integration.rs` | Full TUI workflow |

### Snapshot Tests

Using insta for visual regression:

```rust
#[test]
fn test_input_mode_layout() {
    let mut harness = TuiTestHarness::new();
    harness.type_input("test prompt");
    let screenshot = harness.capture(None);
    insta::assert_snapshot!(screenshot.to_string());
}
```

### Manual Verification Checklist

- [ ] `claudeless --tui` enters TUI mode
- [ ] Typing appears in input area
- [ ] Enter submits and shows response
- [ ] Ctrl+C clears input / exits on empty
- [ ] Ctrl+D exits on empty input
- [ ] Arrow keys navigate history
- [ ] Response streams token by token
- [ ] Status bar shows token counts
- [ ] Permission prompts render correctly
- [ ] Y/N respond to permission prompts
- [ ] Terminal restored on exit
- [ ] Terminal restored on panic

### Test Commands

```bash
# All TUI tests
cargo test -p claudeless tui

# Unit tests only
cargo test -p claudeless tui --lib

# Integration tests
cargo test -p claudeless --test 'tui_*'

# Run TUI manually
cargo run -p claudeless -- --tui --scenario scenarios/simple.toml

# Force print mode (no TUI)
cargo run -p claudeless -- --no-tui -p "test"

# Update snapshots
cargo insta test -p claudeless --accept
```

### Performance Verification

- TUI should render at 60 fps (16ms frame time)
- Input latency < 50ms
- Streaming should not block input handling
- Memory usage stable during long sessions
