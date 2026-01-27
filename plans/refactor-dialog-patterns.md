# Plan: Refactor Dialog Patterns

## Problem

### 1. Repeated dialog rendering dispatch (render/mod.rs:37-101)

Eight nearly identical blocks:
```rust
if state.mode == AppMode::XXXDialog {
    if let Some(ref dialog) = state.xxx_dialog {
        return render_xxx_dialog(dialog, width);
    }
}
```

### 2. Repeated dialog dismissal (dialogs.rs)

Five dialogs have identical escape handling:
```rust
KeyCode::Esc => {
    inner.mode = AppMode::Input;
    inner.xxx_dialog = None;
    inner.response_content = "XXX dialog dismissed".to_string();
    inner.is_command_output = true;
}
```

## Files to Modify

- `crates/cli/src/tui/app/render/mod.rs` - Simplify dialog dispatch
- `crates/cli/src/tui/app/dialogs.rs` - Extract dismissal helper
- `crates/cli/src/tui/app/state.rs` - Add dialog helper methods

## Implementation

### Step 1: Add dialog dismissal helper

In `state.rs`, add to `TuiAppStateInner`:

```rust
impl TuiAppStateInner {
    /// Dismiss any active dialog and return to input mode.
    pub fn dismiss_dialog(&mut self, name: &str) {
        self.mode = AppMode::Input;
        self.response_content = format!("{} dismissed", name);
        self.is_command_output = true;

        // Clear all dialog states
        self.thinking_dialog = None;
        self.tasks_dialog = None;
        self.model_picker_dialog = None;
        self.export_dialog = None;
        self.help_dialog = None;
        self.hooks_dialog = None;
        self.memory_dialog = None;
        self.trust_prompt = None;
        self.pending_permission = None;
    }
}
```

### Step 2: Simplify dialog handlers

In `dialogs.rs`, replace repeated blocks:

```rust
// Before (repeated 5+ times)
KeyCode::Esc => {
    inner.mode = AppMode::Input;
    inner.help_dialog = None;
    inner.response_content = "Help dialog dismissed".to_string();
    inner.is_command_output = true;
}

// After
KeyCode::Esc => {
    inner.dismiss_dialog("Help dialog");
}
```

### Step 3: Create dialog rendering dispatcher

In `render/mod.rs`, replace the 8 if-blocks with a single dispatch:

```rust
/// Render modal dialog if one is active, otherwise return None.
fn render_active_dialog(state: &RenderState, width: usize) -> Option<AnyElement<'static>> {
    match state.mode {
        AppMode::Trust => state.trust_prompt.as_ref()
            .map(|p| render_trust_prompt(p, width)),
        AppMode::ThinkingToggle => state.thinking_dialog.as_ref()
            .map(|d| render_thinking_dialog(d, width)),
        AppMode::TasksDialog => state.tasks_dialog.as_ref()
            .map(|d| render_tasks_dialog(d, width)),
        AppMode::ExportDialog => state.export_dialog.as_ref()
            .map(|d| render_export_dialog(d, width)),
        AppMode::HelpDialog => state.help_dialog.as_ref()
            .map(|d| render_help_dialog(d, width)),
        AppMode::HooksDialog => state.hooks_dialog.as_ref()
            .map(|d| render_hooks_dialog(d, width)),
        AppMode::MemoryDialog => state.memory_dialog.as_ref()
            .map(|d| render_memory_dialog(d, width)),
        AppMode::ModelPicker => state.model_picker_dialog.as_ref()
            .map(|d| render_model_picker_dialog(d, width)),
        AppMode::Permission => state.pending_permission.as_ref()
            .map(|p| render_permission_dialog(p, width)),
        _ => None,
    }
}

pub(crate) fn render_main_content(state: &RenderState) -> AnyElement<'static> {
    let width = state.terminal_width as usize;

    // Modal dialogs take over the full screen
    if let Some(dialog) = render_active_dialog(state, width) {
        return dialog;
    }

    // Regular content rendering...
}
```

### Step 4: Consider enum-based dialog state (optional future improvement)

For a more robust solution, consider unifying dialog states:

```rust
pub enum ActiveDialog {
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
```

This would replace the 9 `Option<XxxDialog>` fields with a single field, but requires more refactoring.

## Testing

- Existing TUI tests should pass
- Dialog dismissal behavior unchanged

## Lines Changed

- ~60 lines removed (duplicate if-blocks and Esc handlers)
- ~30 lines added (helper function and match dispatch)
- Net: ~30 lines reduced
