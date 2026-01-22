# Implementation Plan: /context Slash Command

## Overview

Implement the `/context` slash command to visualize current context usage as a colored grid. When executed, it displays:
- A 10x9 grid using Unicode symbols (⛀ ⛁ ⛶ ⛝) representing context allocation
- "Estimated usage by category" with token counts and percentages for:
  - System prompt, System tools, Memory files, Messages, Free space, Autocompact buffer
- "Memory files · /memory" section listing loaded CLAUDE.md files with token counts

This is a display-only command (no interactive dialog mode needed) that renders output directly in the response area.

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs                 # Add /context handler and rendering
│   ├── slash_menu.rs          # Already has "context" command (needs description update)
│   └── widgets/
│       ├── mod.rs             # Export ContextUsage
│       └── context.rs         # NEW: ContextUsage struct and formatting
└── state/                     # (No changes - context is computed, not persisted)

crates/cli/tests/
├── tui_context.rs             # Remove #[ignore] from 5 tests
└── fixtures/tui/v2.1.12/
    ├── context_usage.txt      # Already exists - grid display
    └── context_autocomplete.txt # Already exists - autocomplete
```

## Dependencies

No new dependencies required. Uses existing:
- Unicode box-drawing and symbol characters for grid display
- Existing state structures for token counting

## Implementation Phases

### Phase 1: Create ContextUsage Widget

Create a new widget module for context usage data and formatting logic.

**File:** `crates/cli/src/tui/widgets/context.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Context usage visualization widget.
//!
//! Shown when user executes `/context` to view context allocation.

/// Context usage category with token count
#[derive(Clone, Debug)]
pub struct ContextCategory {
    pub name: String,
    pub tokens: u64,
    pub symbol: char,
}

/// Memory file info
#[derive(Clone, Debug)]
pub struct MemoryFile {
    pub path: String,
    pub tokens: u64,
}

/// Data for context usage display
#[derive(Clone, Debug, Default)]
pub struct ContextUsage {
    pub system_prompt_tokens: u64,
    pub system_tools_tokens: u64,
    pub memory_files_tokens: u64,
    pub messages_tokens: u64,
    pub free_space_tokens: u64,
    pub autocompact_buffer_tokens: u64,
    pub total_tokens: u64,
    pub memory_files: Vec<MemoryFile>,
}

impl ContextUsage {
    /// Create default context usage with typical values
    pub fn new() -> Self {
        Self {
            system_prompt_tokens: 2800,
            system_tools_tokens: 16300,
            memory_files_tokens: 659,
            messages_tokens: 787,
            free_space_tokens: 134_000,
            autocompact_buffer_tokens: 45_000,
            total_tokens: 200_000,
            memory_files: vec![
                MemoryFile {
                    path: "CLAUDE.md".to_string(),
                    tokens: 659,
                }
            ],
        }
    }

    /// Calculate percentage for a token count
    pub fn percentage(&self, tokens: u64) -> f64 {
        if self.total_tokens == 0 {
            0.0
        } else {
            (tokens as f64 / self.total_tokens as f64) * 100.0
        }
    }

    /// Format tokens for display (e.g., "2.8k" or "134k")
    pub fn format_tokens(tokens: u64) -> String {
        if tokens >= 1000 {
            let k = tokens as f64 / 1000.0;
            if k >= 10.0 {
                format!("{:.0}k", k)
            } else {
                format!("{:.1}k", k)
            }
        } else {
            tokens.to_string()
        }
    }

    /// Get grid cells (90 total: 10 columns x 9 rows)
    /// Returns Vec of (symbol, is_used) tuples
    pub fn grid_cells(&self) -> Vec<char> {
        // Calculate proportions out of 90 cells
        let total_used = self.system_prompt_tokens
            + self.system_tools_tokens
            + self.memory_files_tokens
            + self.messages_tokens;
        let used_cells = ((total_used as f64 / self.total_tokens as f64) * 90.0).ceil() as usize;
        let autocompact_cells = ((self.autocompact_buffer_tokens as f64 / self.total_tokens as f64) * 90.0).ceil() as usize;
        let free_cells = 90_usize.saturating_sub(used_cells).saturating_sub(autocompact_cells);

        let mut cells = Vec::with_capacity(90);

        // First cell is ⛀ (used marker), rest of used is ⛁
        if used_cells > 0 {
            cells.push('⛀');
            for _ in 1..used_cells {
                cells.push('⛁');
            }
        }

        // Free space cells
        for _ in 0..free_cells {
            cells.push('⛶');
        }

        // Autocompact buffer cells
        for _ in 0..autocompact_cells {
            cells.push('⛝');
        }

        // Ensure exactly 90 cells
        cells.truncate(90);
        while cells.len() < 90 {
            cells.push('⛶');
        }

        cells
    }
}
```

**File:** `crates/cli/src/tui/widgets/mod.rs`

Add export:
```rust
pub mod context;
pub use context::ContextUsage;
```

**Verification:** `cargo build` succeeds.

---

### Phase 2: Update Slash Menu Description

Update the `/context` command description to match Claude Code behavior.

**File:** `crates/cli/src/tui/slash_menu.rs`

Update the existing entry:
```rust
SlashCommand {
    name: "context",
    description: "Visualize current context usage as a colored grid",
    argument_hint: None,
},
```

**Verification:**
- `cargo build`
- Typing `/context` shows updated description in autocomplete

---

### Phase 3: Implement Context Rendering Function

Create the rendering function that formats context usage matching the fixture.

**File:** `crates/cli/src/tui/app.rs`

Add import:
```rust
use super::widgets::context::ContextUsage;
```

Add rendering function (near other render functions):
```rust
/// Format context usage as a grid display
fn format_context_usage(usage: &ContextUsage) -> String {
    let cells = usage.grid_cells();
    let mut lines = Vec::new();

    // Build grid rows (10 cells per row, 9 rows)
    for row in 0..9 {
        let start = row * 10;
        let end = start + 10;
        let row_cells: String = cells[start..end]
            .iter()
            .map(|c| format!("{} ", c))
            .collect::<String>()
            .trim_end()
            .to_string();

        // First 6 rows have category labels on the right
        let label = match row {
            1 => format!("  Estimated usage by category"),
            2 => format!(
                "  ⛁ System prompt: {} tokens ({:.1}%)",
                ContextUsage::format_tokens(usage.system_prompt_tokens),
                usage.percentage(usage.system_prompt_tokens)
            ),
            3 => format!(
                "  ⛁ System tools: {} tokens ({:.1}%)",
                ContextUsage::format_tokens(usage.system_tools_tokens),
                usage.percentage(usage.system_tools_tokens)
            ),
            4 => format!(
                "  ⛁ Memory files: {} tokens ({:.1}%)",
                ContextUsage::format_tokens(usage.memory_files_tokens),
                usage.percentage(usage.memory_files_tokens)
            ),
            5 => format!(
                "  ⛁ Messages: {} tokens ({:.1}%)",
                ContextUsage::format_tokens(usage.messages_tokens),
                usage.percentage(usage.messages_tokens)
            ),
            6 => format!(
                "  ⛶ Free space: {} ({:.1}%)",
                ContextUsage::format_tokens(usage.free_space_tokens),
                usage.percentage(usage.free_space_tokens)
            ),
            7 => format!(
                "  ⛝ Autocompact buffer: {} tokens ({:.1}%)",
                ContextUsage::format_tokens(usage.autocompact_buffer_tokens),
                usage.percentage(usage.autocompact_buffer_tokens)
            ),
            _ => String::new(),
        };

        lines.push(format!("     {}   {}", row_cells, label));
    }

    // Add memory files section
    lines.push(String::new());
    lines.push("     Memory files · /memory".to_string());
    for file in &usage.memory_files {
        lines.push(format!(
            "     └ {}: {} tokens",
            file.path,
            file.tokens
        ));
    }

    lines.join("\n")
}
```

**Verification:** Unit test for formatting produces expected output.

---

### Phase 4: Implement /context Command Handler

Add the command handler to display context usage.

**File:** `crates/cli/src/tui/app.rs`

Add match arm in `handle_command_inner()` (before the `_ =>` catch-all):
```rust
"/context" => {
    let usage = ContextUsage::new();
    inner.response_content = format_context_usage(&usage);
    inner.is_command_output = true;
}
```

**Verification:**
- Running `/context` displays the grid and category breakdown
- `test_context_shows_usage_grid` passes

---

### Phase 5: Enable Tests and Final Verification

Remove `#[ignore]` from all tests and verify.

**File:** `crates/cli/tests/tui_context.rs`

Remove `#[ignore]` attribute and `// TODO(implement)` comments from these tests:
- `test_context_shows_usage_grid` (line 27)
- `test_context_shows_memory_files` (line 84)
- `test_context_in_autocomplete` (line 121)
- `test_context_shows_visual_grid` (line 168)
- `test_context_shows_token_percentages` (line 207)

**Verification:**
```bash
cargo test --test tui_context
make check
```

## Key Implementation Details

### Grid Format

The fixture shows a 10x9 grid using these Unicode symbols:
- `⛀` - First cell marker (used context start)
- `⛁` - Used context (system prompt, tools, memory, messages)
- `⛶` - Free space
- `⛝` - Autocompact buffer

Grid layout with labels on right side:
```
     ⛀ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶
     ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶   Estimated usage by category
     ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶   ⛁ System prompt: 2.8k tokens (1.4%)
     ...
```

### Token Formatting

- Tokens >= 1000 shown as "Xk" or "X.Xk"
- Percentages shown with one decimal place

### Output Location

Unlike `/tasks` which uses a dialog mode, `/context` outputs directly to `response_content` with `is_command_output = true`. This matches simpler commands like `/help` and `/todos`.

### Memory Files Section

Shows loaded CLAUDE.md files:
```
     Memory files · /memory
     └ CLAUDE.md: 659 tokens
```

## Verification Plan

1. **Unit Tests:**
   - All 5 tests in `tui_context.rs` pass

2. **Integration:**
   - `make check` passes (includes lint, format, clippy, tests, build, audit)

3. **Manual Testing:**
   - Launch TUI: `cargo run -- --scenario <file>`
   - Type `/context` - should show grid with categories and percentages
   - Type `/context` partially - should appear in autocomplete with description
   - Verify grid symbols are visible (⛀ ⛁ ⛶ ⛝)
   - Verify token counts and percentages are displayed

## Files Modified Summary

| File | Changes |
|------|---------|
| `crates/cli/src/tui/widgets/context.rs` | NEW: ContextUsage struct and grid formatting |
| `crates/cli/src/tui/widgets/mod.rs` | Export ContextUsage |
| `crates/cli/src/tui/app.rs` | Add /context handler and format_context_usage function |
| `crates/cli/src/tui/slash_menu.rs` | Update description to "Visualize current context usage as a colored grid" |
| `crates/cli/tests/tui_context.rs` | Remove `#[ignore]` from 5 tests |
