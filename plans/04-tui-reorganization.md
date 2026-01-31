# TUI Module Reorganization

## Problem

TUI has deep nesting inconsistent with rest of codebase:

```
tui/
├── app/
│   ├── state/
│   │   ├── dialog.rs
│   │   ├── input.rs
│   │   └── display.rs
│   ├── render/
│   │   ├── format.rs
│   │   ├── content.rs
│   │   └── dialogs.rs
│   ├── commands.rs
│   ├── dialogs.rs
│   └── types.rs
├── widgets/
│   └── (12 files)
└── (6 other files)
```

Other modules are flat (`state/`, `permission/`, `tools/`). The 3-level nesting (`tui/app/state/`) is harder to navigate.

## Plan

1. **Flatten `app/state/` into `app/`**:
   - `app/state/dialog.rs` → `app/dialog_state.rs`
   - `app/state/input.rs` → `app/input_state.rs`
   - `app/state/display.rs` → `app/display_state.rs`
   - Or consolidate into single `app/state.rs` if they're small

2. **Flatten `app/render/` into `app/`**:
   - `app/render/format.rs` → `app/format.rs`
   - `app/render/content.rs` → `app/content.rs`
   - `app/render/dialogs.rs` → `app/render_dialogs.rs` (avoid collision with `app/dialogs.rs`)

3. **Consider consolidating small widget files**:
   - Group related widgets: `widgets/dialogs.rs` (permission, trust, export)
   - Keep larger widgets separate: `widgets/scrollable.rs`, `widgets/model_picker.rs`

4. **Target structure**:
   ```
   tui/
   ├── app.rs (or app/ with ~6 files max)
   ├── widgets/
   │   └── (6-8 files)
   ├── colors.rs
   ├── shortcuts.rs
   └── spinner.rs
   ```
