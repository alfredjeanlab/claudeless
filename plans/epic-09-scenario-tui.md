# Epic 9: Scenario-Driven TUI

## Overview

Enhance the TUI to respect scenario configuration and produce output matching real Claude Code's TUI. The infrastructure exists (TuiApp, widgets, input handling) but is missing key behaviors: trust prompts, proper model display, user name in prompts, and thinking toggle. This epic wires scenario fields (`trusted`, `user_name`, `default_model`) into the TUI and implements missing dialogs to pass all `tui_*.rs` integration tests.

**What's in this epic:**
- Trust prompt dialog when `trusted: false` in scenario
- User name display in input prompt (from scenario)
- Model display format matching real Claude Code
- Thinking toggle dialog (Ctrl+T)
- Wire scenario config through SessionContext to TUI
- Pass all `tui_*.rs` integration tests

**What's NOT in this epic:**
- Settings file support (Epic 10)
- Full visual parity with real Claude (focus on testable layout)
- Mouse input, window resize handling
- Async streaming (synchronous is sufficient for tests)

## Project Structure

```
crates/cli/
├── src/
│   ├── main.rs                    # UPDATE: Pass scenario config to TUI
│   ├── tui/
│   │   ├── mod.rs                 # UPDATE: Export new types
│   │   ├── app.rs                 # UPDATE: Trust state, scenario config
│   │   ├── input.rs               # UPDATE: Ctrl+T handler, trust prompt keys
│   │   ├── layout.rs              # UPDATE: Conditional trust prompt rendering
│   │   └── widgets/
│   │       ├── mod.rs             # UPDATE: Export trust widget
│   │       ├── trust.rs           # NEW: Trust prompt widget
│   │       ├── thinking.rs        # NEW: Thinking toggle widget
│   │       ├── input.rs           # UPDATE: User name display
│   │       └── status.rs          # UPDATE: Model format
│   └── session/
│       └── context.rs             # EXISTING: SessionContext (trusted field)
├── tests/
│   ├── tui_trust.rs               # Should pass after Phase 2
│   ├── tui_model.rs               # Should pass after Phase 3
│   ├── tui_thinking.rs            # Should pass after Phase 4
│   └── common/mod.rs              # Test helpers
```

## Dependencies

No new external dependencies. Uses existing:
- `ratatui` for terminal rendering (already in Cargo.toml)
- `crossterm` for input handling (already in Cargo.toml)

Note: Epic spec mentions "iocraft or crossterm" - we use ratatui (built on crossterm) which is already working.

---

## Phase 1: Wire Scenario Config to TUI

**Goal**: Pass scenario configuration through to TuiApp so it can access `trusted`, `user_name`, `default_model`.

**Update TuiApp** (`src/tui/app.rs`):

```rust
/// Configuration from scenario for TUI behavior
#[derive(Clone, Debug)]
pub struct TuiConfig {
    pub trusted: bool,
    pub user_name: String,
    pub model: String,
    pub working_directory: PathBuf,
    pub permission_mode: Option<String>,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            trusted: true,
            user_name: "Alfred".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            working_directory: std::env::current_dir().unwrap_or_default(),
            permission_mode: None,
        }
    }
}

impl TuiConfig {
    pub fn from_scenario(config: &ScenarioConfig, cli_model: Option<&str>) -> Self {
        Self {
            trusted: config.trusted,
            user_name: config.user_name.clone()
                .unwrap_or_else(|| DEFAULT_USER_NAME.to_string()),
            model: cli_model.map(|s| s.to_string())
                .or_else(|| config.default_model.clone())
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            working_directory: config.working_directory.as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default()),
            permission_mode: config.permission_mode.clone(),
        }
    }
}

pub struct TuiApp {
    // ... existing fields ...

    /// Configuration from scenario
    pub(crate) config: TuiConfig,

    /// Whether trust has been granted (for untrusted dirs)
    pub(crate) trust_granted: bool,
}
```

**Update app constructor**:

```rust
impl TuiApp {
    pub fn new(
        scenario: Scenario,
        sessions: SessionManager,
        clock: ClockHandle,
        config: TuiConfig,
    ) -> io::Result<Self> {
        // ... existing setup ...

        Ok(Self {
            // ... existing fields ...
            config,
            trust_granted: config.trusted, // Start granted if already trusted
            status: StatusInfo {
                model: config.model.clone(),
                ..Default::default()
            },
        })
    }
}
```

**Update main.rs**:

```rust
fn run_tui_mode(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let scenario = /* load scenario as before */;

    let config = TuiConfig::from_scenario(
        scenario.config(),
        cli.model.as_deref(),
    );

    let mut app = TuiApp::new(scenario, sessions, clock, config)?;
    // ...
}
```

**Verification**:
- TuiApp accepts TuiConfig parameter
- Config fields are accessible from TuiApp methods
- `cargo test -p claudeless tui` unit tests pass

---

## Phase 2: Trust Prompt Dialog

**Goal**: Show trust prompt when `trusted: false` in scenario, block input until user responds.

**New widget** (`src/tui/widgets/trust.rs`):

```rust
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrustChoice {
    Yes,
    No,
}

pub struct TrustPrompt {
    pub working_directory: String,
    pub selected: TrustChoice,
}

impl TrustPrompt {
    pub fn new(working_directory: String) -> Self {
        Self {
            working_directory,
            selected: TrustChoice::Yes,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Clear the background
        frame.render_widget(Clear, area);

        // Center the dialog
        let dialog_width = 60.min(area.width.saturating_sub(4));
        let dialog_height = 10;
        let dialog_x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Trust Folder ")
            .style(Style::default().fg(Color::Yellow));

        let yes_style = if self.selected == TrustChoice::Yes {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let no_style = if self.selected == TrustChoice::No {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let text = Text::from(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Do you trust the files in this folder?",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::raw(&self.working_directory)),
            Line::from(""),
            Line::from(Span::styled(
                "Trusting a folder allows Claude to read and modify files.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "This has security risks if the folder contains malicious code.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("[Yes]", yes_style),
                Span::raw("  "),
                Span::styled("[No/Exit]", no_style),
            ]),
        ]);

        let paragraph = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, dialog_area);
    }
}
```

**New AppMode variant** (`src/tui/app.rs`):

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppMode {
    Input,
    Responding,
    Permission,
    Thinking,
    Trust,  // NEW: Showing trust prompt
}
```

**Update app startup** to show trust prompt:

```rust
impl TuiApp {
    pub fn new(/* ... */) -> io::Result<Self> {
        let initial_mode = if config.trusted {
            AppMode::Input
        } else {
            AppMode::Trust
        };

        let trust_prompt = if !config.trusted {
            Some(TrustPrompt::new(
                config.working_directory.to_string_lossy().to_string()
            ))
        } else {
            None
        };

        Ok(Self {
            mode: initial_mode,
            trust_prompt,
            // ...
        })
    }
}
```

**Handle trust prompt keys** (`src/tui/input.rs`):

```rust
impl TuiApp {
    pub(crate) fn handle_trust_key(&mut self, key: KeyEvent) {
        match key.code {
            // Left/Right - Toggle selection
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                if let Some(ref mut prompt) = self.trust_prompt {
                    prompt.selected = match prompt.selected {
                        TrustChoice::Yes => TrustChoice::No,
                        TrustChoice::No => TrustChoice::Yes,
                    };
                }
            }

            // Enter - Confirm
            KeyCode::Enter => {
                if let Some(ref prompt) = self.trust_prompt {
                    match prompt.selected {
                        TrustChoice::Yes => {
                            self.trust_granted = true;
                            self.trust_prompt = None;
                            self.mode = AppMode::Input;
                        }
                        TrustChoice::No => {
                            self.exit(ExitReason::UserQuit);
                        }
                    }
                }
            }

            // Y/y - Yes
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.trust_granted = true;
                self.trust_prompt = None;
                self.mode = AppMode::Input;
            }

            // N/n or Escape - No/Exit
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.exit(ExitReason::UserQuit);
            }

            _ => {}
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        match self.mode {
            AppMode::Trust => self.handle_trust_key(key),
            AppMode::Input => self.handle_input_key(key),
            // ... rest unchanged
        }
    }
}
```

**Update layout.rs** to render trust prompt:

```rust
pub fn render(frame: &mut Frame, state: &RenderState) {
    // If in trust mode, render trust prompt over everything
    if state.mode == AppMode::Trust {
        if let Some(ref prompt) = state.trust_prompt {
            prompt.render(frame, frame.size());
            return;
        }
    }

    // ... existing layout rendering ...
}
```

**Verification**:
- `test_shows_trust_prompt_in_new_directory` passes
- `test_trust_prompt_mentions_files` passes
- `test_trust_prompt_shows_directory_path` passes
- `test_trust_prompt_has_yes_no_options` passes
- `test_trust_prompt_mentions_security` passes
- `test_trust_prompt_yes_proceeds` passes
- `test_trust_prompt_escape_cancels` passes

---

## Phase 3: Model Display Format

**Goal**: Display model name in status bar matching real Claude Code format.

**Model name mapping** (`src/tui/widgets/status.rs`):

```rust
/// Map model ID to display name
fn model_display_name(model: &str) -> String {
    // Handle short aliases (from --model flag)
    let base_name = match model.to_lowercase().as_str() {
        "haiku" | "claude-haiku" => "Haiku",
        "sonnet" | "claude-sonnet" => "Sonnet",
        "opus" | "claude-opus" => "Opus",
        _ => {
            // Parse full model ID like "claude-sonnet-4-20250514"
            if model.contains("haiku") {
                "Haiku"
            } else if model.contains("opus") {
                "Opus"
            } else if model.contains("sonnet") {
                "Sonnet"
            } else {
                // Unknown model, show as-is
                return model.to_string();
            }
        }
    };

    // Extract version if present (e.g., "4.5" from "claude-opus-4-5-...")
    let version = extract_model_version(model);

    match version {
        Some(v) => format!("{} {}", base_name, v),
        None => base_name.to_string(),
    }
}

fn extract_model_version(model: &str) -> Option<String> {
    // Pattern: claude-{name}-{major}-{minor?}-{date}
    // e.g., "claude-opus-4-5-20251101" -> "4.5"
    // e.g., "claude-sonnet-4-20250514" -> "4"
    let parts: Vec<&str> = model.split('-').collect();
    if parts.len() >= 4 && parts[0] == "claude" {
        let major = parts[2];
        if major.chars().all(|c| c.is_ascii_digit()) {
            let minor = parts.get(3);
            if let Some(m) = minor {
                if m.chars().all(|c| c.is_ascii_digit()) && m.len() <= 2 {
                    return Some(format!("{}.{}", major, m));
                }
            }
            return Some(major.to_string());
        }
    }
    None
}

pub fn render_status_bar(frame: &mut Frame, area: Rect, status: &StatusInfo) {
    let display_name = model_display_name(&status.model);

    // Format: "Model · Info"
    let status_text = format!(
        "{} · {} input · {} output",
        display_name,
        status.input_tokens,
        status.output_tokens
    );

    let paragraph = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(paragraph, area);
}
```

**Verification**:
- `test_tui_shows_haiku_model_name` passes
- `test_tui_shows_sonnet_model_name` passes
- `test_tui_shows_opus_model_name` passes
- `test_tui_model_display_format` passes

---

## Phase 4: Thinking Toggle Dialog

**Goal**: Ctrl+T opens thinking mode toggle dialog matching real Claude Code.

**New widget** (`src/tui/widgets/thinking.rs`):

```rust
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThinkingMode {
    Enabled,
    Disabled,
}

pub struct ThinkingDialog {
    pub selected: ThinkingMode,
}

impl ThinkingDialog {
    pub fn new(current: ThinkingMode) -> Self {
        Self { selected: current }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let dialog_width = 40.min(area.width.saturating_sub(4));
        let dialog_height = 6;
        let dialog_x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Extended Thinking ");

        let enabled_style = if self.selected == ThinkingMode::Enabled {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let disabled_style = if self.selected == ThinkingMode::Disabled {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let text = Text::from(vec![
            Line::from(""),
            Line::from("Toggle extended thinking mode:"),
            Line::from(""),
            Line::from(vec![
                Span::styled("[Enabled]", enabled_style),
                Span::raw("  "),
                Span::styled("[Disabled]", disabled_style),
            ]),
        ]);

        let paragraph = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, dialog_area);
    }
}
```

**Add state and handlers**:

```rust
// In app.rs
pub struct TuiApp {
    // ...
    pub(crate) thinking_enabled: bool,
    pub(crate) thinking_dialog: Option<ThinkingDialog>,
}

// New mode variant
pub enum AppMode {
    // ... existing ...
    ThinkingToggle,  // Showing thinking toggle dialog
}

// In input.rs
impl TuiApp {
    pub(crate) fn handle_input_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            // Ctrl+T - Toggle thinking dialog
            (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
                self.show_thinking_dialog();
            }
            // ... existing handlers ...
        }
    }

    fn show_thinking_dialog(&mut self) {
        let current = if self.thinking_enabled {
            ThinkingMode::Enabled
        } else {
            ThinkingMode::Disabled
        };
        self.thinking_dialog = Some(ThinkingDialog::new(current));
        self.mode = AppMode::ThinkingToggle;
    }

    pub(crate) fn handle_thinking_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                if let Some(ref mut dialog) = self.thinking_dialog {
                    dialog.selected = match dialog.selected {
                        ThinkingMode::Enabled => ThinkingMode::Disabled,
                        ThinkingMode::Disabled => ThinkingMode::Enabled,
                    };
                }
            }

            KeyCode::Enter => {
                if let Some(ref dialog) = self.thinking_dialog {
                    self.thinking_enabled = dialog.selected == ThinkingMode::Enabled;
                }
                self.thinking_dialog = None;
                self.mode = AppMode::Input;
            }

            KeyCode::Esc => {
                self.thinking_dialog = None;
                self.mode = AppMode::Input;
            }

            _ => {}
        }
    }
}
```

**Verification**:
- `test_thinking_toggle_opens_on_ctrl_t` passes
- `test_thinking_toggle_has_options` passes
- `test_thinking_toggle_enter_confirms` passes

---

## Phase 5: User Name in Input Prompt

**Goal**: Display user name from scenario in input prompt area.

**Update input widget** (`src/tui/widgets/input.rs`):

```rust
pub fn render_input(frame: &mut Frame, area: Rect, state: &RenderState) {
    let user_name = &state.user_name;

    // Format: "{user_name}> {input}"
    let prompt = format!("{} > ", user_name);
    let input = &state.input_buffer;

    let text = Line::from(vec![
        Span::styled(&prompt, Style::default().fg(Color::Cyan)),
        Span::raw(input),
    ]);

    let paragraph = Paragraph::new(text)
        .style(Style::default());

    frame.render_widget(paragraph, area);

    // Position cursor
    let cursor_x = area.x + prompt.len() as u16 + state.cursor_pos as u16;
    let cursor_y = area.y;
    frame.set_cursor_position((cursor_x, cursor_y));
}
```

**Update RenderState** to include user_name:

```rust
#[derive(Clone, Debug)]
pub struct RenderState {
    // ... existing fields ...
    pub user_name: String,
}

impl TuiApp {
    fn render_state(&self) -> RenderState {
        RenderState {
            // ... existing fields ...
            user_name: self.config.user_name.clone(),
        }
    }
}
```

**Verification**:
- Input prompt shows configured user name
- Default "Alfred" when not specified
- `cargo test -p claudeless --test tui_interaction` passes

---

## Phase 6: Integration & Test Verification

**Goal**: Ensure all TUI tests pass and behavior matches real Claude Code.

**Update test helpers** (`tests/common/mod.rs`):

Ensure tmux-based tests can reliably capture TUI state. Increase timeouts if needed for CI.

**Run full test suite**:

```bash
# Run all TUI tests sequentially (they use tmux sessions)
cargo test -p claudeless --test tui_trust -- --test-threads=1
cargo test -p claudeless --test tui_model -- --test-threads=1
cargo test -p claudeless --test tui_thinking -- --test-threads=1
cargo test -p claudeless --test tui_layout -- --test-threads=1
cargo test -p claudeless --test tui_interaction -- --test-threads=1
cargo test -p claudeless --test tui_permission -- --test-threads=1
cargo test -p claudeless --test tui_exit -- --test-threads=1
cargo test -p claudeless --test tui_snapshot -- --test-threads=1
cargo test -p claudeless --test tui_compacting -- --test-threads=1
```

**Fix any failing tests** by adjusting:
- Timing (sleep durations in tests)
- Text matching (case sensitivity, partial matches)
- Layout rendering (ensure text fits in capture area)

**Verification**:
- All `tui_*.rs` tests pass
- `make check` passes
- Manual verification: run `--tui` mode and interact

---

## Key Implementation Details

### Trust Prompt Behavior

| Scenario Field | TUI Behavior |
|----------------|--------------|
| `trusted: true` (default) | Skip trust prompt, start in Input mode |
| `trusted: false` | Show trust prompt, block until Yes/No |

Trust prompt renders as centered modal dialog over the main TUI.

### Model Name Mapping

| CLI / Scenario | Display Name |
|----------------|--------------|
| `--model haiku` | "Haiku 4.5" |
| `--model sonnet` | "Sonnet 4" |
| `--model opus` | "Opus 4.5" |
| `claude-sonnet-4-20250514` | "Sonnet 4" |
| Custom model ID | Model ID as-is |

### App Mode State Machine

```
                    [trusted=false]
                          │
                          ▼
                    ┌─────────┐
                    │  Trust  │
                    └────┬────┘
                         │ Yes
                         ▼
┌─────────┐         ┌─────────┐         ┌────────────┐
│ Thinking│◀───────▶│  Input  │◀───────▶│ThinkingTgl │
└────┬────┘  Ctrl+T └────┬────┘  Ctrl+T └────────────┘
     │                   │ Enter
     │ response          ▼
     │              ┌──────────┐
     └─────────────▶│Responding│
                    └────┬─────┘
                         │ tool needs permission
                         ▼
                    ┌──────────┐
                    │Permission│
                    └──────────┘
```

### Keyboard Shortcuts

| Mode | Key | Action |
|------|-----|--------|
| Trust | Enter | Confirm selection |
| Trust | Escape | Exit (deny trust) |
| Trust | Y/y | Yes (trust) |
| Trust | N/n | No (exit) |
| Trust | Left/Right/Tab | Toggle Yes/No |
| Input | Ctrl+T | Open thinking toggle |
| ThinkingTgl | Enter | Confirm |
| ThinkingTgl | Escape | Cancel |

---

## Verification Plan

### Unit Tests

| Module | Key Tests |
|--------|-----------|
| `widgets/trust.rs` | Dialog rendering, selection toggle |
| `widgets/thinking.rs` | Dialog rendering, mode toggle |
| `widgets/status.rs` | Model name mapping, format |
| `widgets/input.rs` | User name display, cursor position |
| `app.rs` | Mode transitions, trust state |
| `input.rs` | Key handlers for new modes |

### Integration Tests

| Test File | Description |
|-----------|-------------|
| `tui_trust.rs` | Trust prompt appearance, behavior |
| `tui_model.rs` | Model display in status bar |
| `tui_thinking.rs` | Thinking toggle dialog |
| `tui_layout.rs` | Overall layout structure |
| `tui_interaction.rs` | Input handling, submission |
| `tui_permission.rs` | Permission dialogs |
| `tui_exit.rs` | Exit behavior (Ctrl+C, Ctrl+D) |
| `tui_snapshot.rs` | Screenshot capture |
| `tui_compacting.rs` | History compaction |

### Test Commands

```bash
# Run all TUI tests
cargo test -p claudeless --test 'tui_*' -- --test-threads=1

# Run specific test file
cargo test -p claudeless --test tui_trust -- --test-threads=1

# Full CI check
make check

# Manual verification
cargo run -p claudeless -- --scenario scenarios/deterministic.toml --tui

# Test untrusted directory
cargo run -p claudeless -- --scenario <(echo '{"trusted": false}') --tui
```

### Manual Verification Checklist

- [ ] TUI starts in Input mode when `trusted: true`
- [ ] TUI shows trust prompt when `trusted: false`
- [ ] Trust prompt shows working directory path
- [ ] Trust prompt has Yes/No options
- [ ] Trust prompt mentions security risks
- [ ] Enter on Yes proceeds to Input mode
- [ ] Escape on trust prompt exits
- [ ] Status bar shows model name (e.g., "Haiku 4.5")
- [ ] Status bar uses `·` separator
- [ ] Input prompt shows user name
- [ ] Ctrl+T opens thinking toggle
- [ ] Thinking toggle has Enabled/Disabled options
- [ ] All `tui_*.rs` tests pass
- [ ] `make check` passes
