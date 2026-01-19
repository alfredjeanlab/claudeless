# Epic 12d: Fix TUI Visual Fidelity

## Overview

Fix TUI visual divergences from real Claude CLI. This epic replaces `ratatui`/`crossterm` with `iocraft`, rewrites all TUI components, and updates the test suite to use fixture comparison.

**Documentation:** https://docs.rs/iocraft/0.7.16/iocraft/

## Execution Model

**IMPORTANT:** Each phase MUST be executed using Task agents. After each phase:
1. Validate the work was completed as specified
2. If validation fails, dispatch additional Task agents to fix discrepancies
3. Do not proceed to the next phase until validation passes

```
Phase N
   ↓
Task Agent executes phase
   ↓
Validation check
   ↓ (fail)
Dispatch fix agent → retry validation
   ↓ (pass)
Phase N+1
```

---

## Phase 1: Dependency Migration

**Goal:** Replace ratatui/crossterm with iocraft in Cargo.toml.

### Task Agent Instructions

```
Update crates/cli/Cargo.toml:
1. Remove dependency: crossterm = { version = "0.28", features = ["event-stream"] }
2. Remove dependency: ratatui = "0.29"
3. Add dependency: iocraft = "0.7"
4. Run: cargo check -p claudeless (expect compile errors - that's ok)
```

### Validation

- [ ] `Cargo.toml` contains `iocraft = "0.7"`
- [ ] `Cargo.toml` does NOT contain `ratatui` or `crossterm`
- [ ] `cargo metadata` shows iocraft in dependency tree

**If validation fails:** Dispatch agent to fix Cargo.toml entries.

---

## Phase 2: Core App Component

**Goal:** Rewrite `src/tui/app.rs` using iocraft component model.

### Task Agent Instructions

```
Rewrite src/tui/app.rs:

1. Remove all ratatui/crossterm imports
2. Add iocraft imports:
   use iocraft::prelude::*;

3. Create main App component with iocraft:
   - Use #[component] macro
   - Use hooks.use_state() for: mode, input_buffer, response_content, etc.
   - Use hooks.use_terminal_events() for keyboard handling
   - Return element! macro tree

4. App modes to support:
   - Trust (show TrustPrompt when trusted=false)
   - Input (normal input mode)
   - Responding (showing response)
   - ThinkingToggle (modal dialog)

5. Remove TuiApp struct - state lives in hooks
6. Remove separate input.rs - keyboard handling is in use_terminal_events
7. Remove layout.rs - iocraft handles layout declaratively

Reference iocraft docs: https://docs.rs/iocraft/0.7.16/iocraft/
```

### Validation

- [ ] `src/tui/app.rs` compiles with `cargo check -p claudeless`
- [ ] No imports from `ratatui` or `crossterm`
- [ ] Contains `#[component]` macro usage
- [ ] Contains `hooks.use_state()` calls
- [ ] Contains `hooks.use_terminal_events()` for input
- [ ] Contains `element!` macro for rendering

**If validation fails:** Dispatch agent with specific compile errors to fix.

---

## Phase 3: Trust Prompt Component

**Goal:** Rewrite trust prompt to match real Claude CLI format.

### Task Agent Instructions

```
Rewrite src/tui/widgets/trust.rs:

1. Create TrustPrompt component matching EXACTLY this format:

   ┌─ Trust Folder ─────────────────────────────────────────┐
   │                                                        │
   │  Do you trust the files in this folder?                │
   │                                                        │
   │  /path/to/working/directory                            │
   │                                                        │
   │  Trusting a folder allows Claude to read and modify    │
   │  files. This has security risks if the folder          │
   │  contains malicious code.                              │
   │                                                        │
   │  ❯ 1. Yes, proceed                                     │
   │    2. No, exit                                         │
   │                                                        │
   │  Enter to confirm · Esc to cancel                      │
   └────────────────────────────────────────────────────────┘

2. Component props:
   - working_directory: String
   - on_accept: Handler
   - on_reject: Handler

3. State:
   - selected: 0 (Yes) or 1 (No)

4. Keyboard handling:
   - Tab/Left/Right/Up/Down: toggle selection
   - Enter: confirm selection (call on_accept or on_reject)
   - y/Y: select Yes and confirm
   - n/N/Esc: select No and confirm

5. Visual requirements:
   - ❯ cursor on selected option
   - Two spaces indent on unselected option
   - Numbered options (1. and 2.)
   - Hint text at bottom
```

### Validation

- [ ] Component compiles
- [ ] Renders `❯ 1. Yes, proceed` (not `[Yes]`)
- [ ] Renders `  2. No, exit` (not `[No/Exit]`)
- [ ] Shows hint text `Enter to confirm · Esc to cancel`
- [ ] Keyboard navigation works (Tab toggles)

**If validation fails:** Dispatch agent with screenshot of current output vs expected.

---

## Phase 4: Remaining Widget Components

**Goal:** Rewrite all other widgets using iocraft.

### Task Agent Instructions

```
Rewrite each widget in src/tui/widgets/:

1. thinking.rs - ThinkingDialog component
   - Title: "Extended Thinking"
   - Options: [Enabled] / [Disabled]
   - Same keyboard handling pattern as TrustPrompt

2. input.rs - InputPrompt component
   - Format: "{user_name} > {input_text}"
   - Use iocraft TextInput if appropriate, or custom
   - Cursor positioning

3. response.rs - ResponseArea component
   - Prefix responses with ⏺ marker
   - Handle streaming indicator

4. status.rs - StatusBar component
   - Model name display (e.g., "Haiku 4.5")
   - Token counts
   - Permission mode indicator

5. mod.rs - Export all components

Delete these files (functionality moved to components):
- src/tui/input.rs (keyboard handling now in components)
- src/tui/layout.rs (iocraft handles layout)
```

### Validation

- [ ] All widget files compile
- [ ] `cargo check -p claudeless` succeeds with no ratatui/crossterm references
- [ ] Each component uses `#[component]` macro
- [ ] `src/tui/input.rs` and `src/tui/layout.rs` are deleted

**If validation fails:** Dispatch agent per failing component.

---

## Phase 5: Test Infrastructure

**Goal:** Create fixture comparison helpers for TUI tests.

### Task Agent Instructions

```
Create test infrastructure:

1. Create crates/cli/src/testing/mod.rs (if not exists)
   - Add: pub mod normalize_tui;

2. Create crates/cli/src/testing/normalize_tui.rs:

   pub fn normalize_tui_capture(capture: &str) -> String {
       // Rules:
       // - Replace /var/folders/... and /tmp/... paths with <TEMPDIR>
       // - Replace ISO timestamps with <TIME>
       // - Replace UUIDs with <UUID>
       // - Strip trailing whitespace per line (preserve leading/interior)
       // - Do NOT collapse multiple spaces
   }

3. Create crates/cli/tests/common/tui_fixtures.rs:

   pub fn load_tui_fixture(version: &str, name: &str) -> String {
       let path = format!("tests/fixtures/tui/{}/{}", version, name);
       std::fs::read_to_string(path).expect("fixture exists")
   }

   pub fn assert_tui_matches_fixture(actual: &str, version: &str, name: &str) {
       let expected = load_tui_fixture(version, name);
       let norm_actual = normalize_tui_capture(actual);
       let norm_expected = normalize_tui_capture(&expected);
       assert_eq!(norm_actual, norm_expected,
           "TUI output does not match fixture {}/{}",
           version, name);
   }

4. Update tests/common/mod.rs to export tui_fixtures
```

### Validation

- [ ] `normalize_tui_capture` handles temp paths, timestamps, UUIDs
- [ ] `normalize_tui_capture` preserves interior whitespace
- [ ] `load_tui_fixture` loads from correct path
- [ ] `assert_tui_matches_fixture` compares normalized strings
- [ ] `cargo test -p claudeless normalize` passes (add unit tests)

**If validation fails:** Dispatch agent with failing test output.

---

## Phase 6: Update Test Suite

**Goal:** Replace keyword-matching tests with fixture comparison tests.

### Task Agent Instructions

```
Update all tui_*.rs test files:

1. For each test file (tui_trust.rs, tui_model.rs, tui_thinking.rs, etc.):

   a. Add import:
      use common::tui_fixtures::assert_tui_matches_fixture;

   b. Replace keyword assertions with fixture assertions:

      BEFORE:
      assert!(capture.to_lowercase().contains("trust"));
      assert!(capture.contains("files") && capture.contains("folder"));

      AFTER:
      assert_tui_matches_fixture(&capture, "v2.1.12", "trust_prompt.txt");

2. Update tmux session dimensions to 120x40:
   tmux new-session -d -s $SESSION -x 120 -y 40

3. Remove tests that are now redundant (multiple keyword tests for same fixture)

4. Keep behavioral tests that can't be fixture-compared:
   - test_trust_prompt_yes_proceeds (tests interaction, not visual)
   - test_trust_prompt_escape_cancels (tests interaction, not visual)

5. Ensure fixtures exist in tests/fixtures/tui/v2.1.12/ for each comparison
```

### Validation

- [ ] No test file contains `contains("trust")` style assertions for visual checks
- [ ] All visual tests use `assert_tui_matches_fixture`
- [ ] tmux sessions use 120x40 dimensions
- [ ] `cargo test -p claudeless --test 'tui_*' -- --test-threads=1` passes

**If validation fails:** Dispatch agent per failing test file with error output.

---

## Phase 7: Integration Validation

**Goal:** Verify complete TUI matches real Claude CLI.

### Task Agent Instructions

```
Run full validation:

1. Run comparison script:
   ./crates/cli/scripts/compare-tui.sh

2. If script reports differences:
   - Document each difference
   - Dispatch fix agents for each divergence
   - Re-run comparison

3. Run full test suite:
   cargo test -p claudeless --test 'tui_*' -- --test-threads=1

4. Run make check:
   make check

5. Manual verification (describe what to check):
   - Start TUI: cargo run -p claudeless -- --tui
   - Verify trust prompt appears correctly (if untrusted)
   - Verify input prompt shows user name
   - Verify model name in status bar
   - Verify Ctrl+T opens thinking dialog
```

### Validation

- [ ] `scripts/compare-tui.sh` exits 0
- [ ] All TUI tests pass
- [ ] `make check` passes
- [ ] No `#[ignore]` tests with `FIXME: epic-05x-fix-tui` remain

**If validation fails:** Dispatch targeted fix agents based on specific failures.

---

## Architecture Reference

### iocraft Component Pattern

```rust
use iocraft::prelude::*;

#[component]
fn MyComponent(mut hooks: Hooks, prop1: String) -> impl Into<AnyElement<'static>> {
    // State
    let mut selected = hooks.use_state(|| 0);

    // Keyboard handling
    hooks.use_terminal_events(move |event| match event {
        TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
            match code {
                KeyCode::Tab => selected.set((selected + 1) % 2),
                KeyCode::Enter => { /* handle */ }
                _ => {}
            }
        }
        _ => {}
    });

    // Render
    element! {
        View(border_style: BorderStyle::Round) {
            Text(content: format!("Selected: {}", selected))
        }
    }
}
```

### Normalization Rules

| Content | Replacement |
|---------|-------------|
| `/var/folders/...` | `<TEMPDIR>` |
| `/tmp/...` | `<TEMPDIR>` |
| ISO 8601 timestamps | `<TIME>` |
| UUIDs | `<UUID>` |
| Trailing whitespace | Strip |
| Leading whitespace | **Preserve** |
| Interior spacing | **Preserve** |

### Terminal Dimensions

All fixtures and tests use **120 columns x 40 rows**.

---

## Final Checklist

- [ ] Phase 1: Dependencies migrated
- [ ] Phase 2: App component rewritten
- [ ] Phase 3: Trust prompt matches fixture
- [ ] Phase 4: All widgets migrated
- [ ] Phase 5: Test infrastructure created
- [ ] Phase 6: Test suite updated
- [ ] Phase 7: Integration validated
- [ ] No ratatui/crossterm imports remain
- [ ] All tests pass
- [ ] `make check` passes
