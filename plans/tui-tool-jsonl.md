# Implementation Plan: TUI Tool Call JSONL Recording

## Overview

Implement Phase 5 from the tui-jsonl plan: record `tool_use` and `tool_result` messages to JSONL when tools execute in TUI mode. Currently, the TUI records user messages and assistant text responses but not tool execution, which is needed for external watchers (otters) to detect tool activity.

## Project Structure

```
crates/cli/src/
├── state/
│   ├── mod.rs                    # StateWriter (already has record_assistant_tool_use, record_tool_result)
│   └── session/
│       └── jsonl.rs              # ContentBlock::ToolUse, tool result types (existing)
├── tui/
│   └── app/
│       ├── state/
│       │   ├── mod.rs            # TuiAppStateInner (has state_writer)
│       │   └── display.rs        # DisplayState (add pending_assistant_uuid)
│       ├── commands.rs           # Permission handling (main changes here)
│       └── types.rs              # PermissionRequest
└── tui/widgets/
    └── permission.rs             # PermissionType (Bash, Edit, Write)
```

## Dependencies

No new dependencies. Uses existing:
- `StateWriter::record_assistant_tool_use()` and `record_tool_result()` from `state/mod.rs`
- `ContentBlock::ToolUse` from `state/session/jsonl.rs`
- `PermissionType` from `tui/widgets/permission.rs`

## Implementation Phases

### Phase 1: Add Pending Assistant UUID Tracking

**File:** `crates/cli/src/tui/app/state/display.rs`

Add a field to track the assistant message UUID when a tool_use is recorded, so we can link tool_result messages back to it.

```rust
/// Display/rendering state
#[derive(Clone, Debug, Default)]
pub struct DisplayState {
    // ... existing fields ...

    /// Pending user message UUID for linking assistant responses in JSONL
    pub pending_user_uuid: Option<String>,
    /// Pending assistant UUID for linking tool results in JSONL
    pub pending_assistant_uuid: Option<String>,
}
```

**Verification:** `cargo build` succeeds.

### Phase 2: Record tool_use on Permission Request

**File:** `crates/cli/src/tui/app/commands.rs`

When `show_permission_request()` triggers a permission dialog, record the assistant message with the `tool_use` block. This is when Claude "decides" to use a tool.

```rust
impl TuiAppState {
    /// Show a permission request with rich dialog
    pub fn show_permission_request(&self, permission_type: PermissionType) {
        // ... existing bypass/session grant checks ...

        // Record assistant tool_use message to JSONL
        let mut inner = self.inner.lock();
        if let (Some(ref writer), Some(ref user_uuid)) =
            (&inner.state_writer, &inner.display.pending_user_uuid)
        {
            // Build tool_use content block
            let (tool_use_id, content) = build_tool_use_content(&permission_type);

            // Record and store assistant UUID for linking tool result
            if let Ok(assistant_uuid) = writer.write().record_assistant_tool_use(user_uuid, content) {
                inner.display.pending_assistant_uuid = Some(assistant_uuid);
                // Store tool_use_id for result linking
                // (stored in permission request for later use)
            }
        }

        // ... show dialog as normal ...
    }
}

/// Build a tool_use content block from a permission type
fn build_tool_use_content(permission_type: &PermissionType) -> (String, Vec<ContentBlock>) {
    let tool_use_id = format!("toolu_{}", uuid::Uuid::new_v4().simple());

    let content = match permission_type {
        PermissionType::Bash { command, description } => {
            let mut input = serde_json::json!({ "command": command });
            if let Some(desc) = description {
                input["description"] = serde_json::json!(desc);
            }
            ContentBlock::ToolUse {
                id: tool_use_id.clone(),
                name: "Bash".to_string(),
                input,
            }
        }
        PermissionType::Edit { file_path, diff_lines } => {
            ContentBlock::ToolUse {
                id: tool_use_id.clone(),
                name: "Edit".to_string(),
                input: serde_json::json!({
                    "file_path": file_path,
                    "changes": diff_lines.len()
                }),
            }
        }
        PermissionType::Write { file_path, content_lines } => {
            ContentBlock::ToolUse {
                id: tool_use_id.clone(),
                name: "Write".to_string(),
                input: serde_json::json!({
                    "file_path": file_path,
                    "content": content_lines.join("\n")
                }),
            }
        }
    };

    (tool_use_id, vec![content])
}
```

**Verification:** After triggering a permission dialog in TUI, check JSONL file contains an assistant message with `stop_reason: "tool_use"` and a `tool_use` content block.

### Phase 3: Store tool_use_id in Permission Request

**File:** `crates/cli/src/tui/app/types.rs`

Extend `PermissionRequest` to store the `tool_use_id` generated when the request was created, so we can reference it when recording the result.

```rust
/// Permission request state using the rich permission dialog
#[derive(Clone, Debug)]
pub struct PermissionRequest {
    pub dialog: RichPermissionDialog,
    /// Tool use ID for JSONL recording
    pub tool_use_id: Option<String>,
}

impl PermissionRequest {
    pub fn new(dialog: RichPermissionDialog) -> Self {
        Self {
            dialog,
            tool_use_id: None,
        }
    }

    pub fn with_tool_use_id(dialog: RichPermissionDialog, tool_use_id: String) -> Self {
        Self {
            dialog,
            tool_use_id: Some(tool_use_id),
        }
    }
}
```

Update `show_permission_request()` to use the new constructor.

**Verification:** `cargo build` succeeds.

### Phase 4: Record tool_result on Permission Confirmation

**File:** `crates/cli/src/tui/app/commands.rs`

When `confirm_permission()` is called and the user grants permission (Yes or YesSession), record the tool result.

```rust
pub(super) fn confirm_permission(&self) {
    let mut inner = self.inner.lock();

    // Extract the permission from dialog state
    let perm = if let DialogState::Permission(p) = std::mem::take(&mut inner.dialog) {
        Some(p)
    } else {
        None
    };
    inner.mode = AppMode::Input;

    if let Some(perm) = perm {
        let tool_name = match &perm.dialog.permission_type {
            PermissionType::Bash { command, .. } => format!("Bash: {}", command),
            PermissionType::Edit { file_path, .. } => format!("Edit: {}", file_path),
            PermissionType::Write { file_path, .. } => format!("Write: {}", file_path),
        };

        let granted = match perm.dialog.selected {
            PermissionSelection::Yes | PermissionSelection::YesSession => true,
            PermissionSelection::No => false,
        };

        // Record tool result to JSONL
        if let (Some(ref writer), Some(ref assistant_uuid), Some(tool_use_id)) =
            (&inner.state_writer, &inner.display.pending_assistant_uuid, &perm.tool_use_id)
        {
            let (result_content, result_json) = if granted {
                let content = format!("[Permission granted for {}]", tool_name);
                (content, serde_json::json!({"success": true}))
            } else {
                let content = format!("[Permission denied for {}]", tool_name);
                (content, serde_json::json!({"success": false, "denied": true}))
            };

            let _ = writer.write().record_tool_result(
                tool_use_id,
                &result_content,
                assistant_uuid,
                result_json,
            );
        }
        inner.display.pending_assistant_uuid = None;

        // ... existing permission handling ...
    }
}
```

**Verification:** After confirming a permission in TUI, JSONL contains a user message with `tool_result` content and a `type: "result"` record.

### Phase 5: Handle Bypass Mode and Auto-Grants

**File:** `crates/cli/src/tui/app/commands.rs`

When permissions are auto-granted (bypass mode or session grants), also record tool_use and tool_result.

```rust
// In show_permission_request(), bypass mode handling:
if inner.permission_mode.allows_all() {
    // Record tool_use and immediate result for bypass mode
    if let Some(ref writer) = inner.state_writer {
        if let Some(ref user_uuid) = inner.display.pending_user_uuid {
            let (tool_use_id, content) = build_tool_use_content(&permission_type);
            if let Ok(assistant_uuid) = writer.write().record_assistant_tool_use(user_uuid, content) {
                let result_content = format!("[Permission auto-granted (bypass): {}]", tool_name);
                let _ = writer.write().record_tool_result(
                    &tool_use_id,
                    &result_content,
                    &assistant_uuid,
                    serde_json::json!({"success": true, "auto_granted": true}),
                );
            }
        }
    }
    drop(inner);
    simulate_permission_accept(self, &permission_type, &tool_name);
    return;
}

// Similar for session grants:
if self.is_session_granted(&permission_type) {
    // Record tool_use and immediate result for session grant
    // ... similar pattern ...
}
```

**Verification:** In bypass mode or with session grants, JSONL contains paired tool_use (with `stop_reason: "tool_use"`) and tool_result messages.

### Phase 6: Unit Tests

**File:** `crates/cli/src/tui/app/commands_tests.rs` (new file)

Add tests for the tool JSONL recording functionality:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_build_tool_use_content_bash() {
    let (id, content) = build_tool_use_content(&PermissionType::Bash {
        command: "ls -la".to_string(),
        description: Some("List files".to_string()),
    });

    assert!(id.starts_with("toolu_"));
    assert_eq!(content.len(), 1);
    match &content[0] {
        ContentBlock::ToolUse { name, input, .. } => {
            assert_eq!(name, "Bash");
            assert_eq!(input["command"], "ls -la");
        }
        _ => panic!("Expected ToolUse content block"),
    }
}

#[test]
fn test_build_tool_use_content_edit() {
    let (id, content) = build_tool_use_content(&PermissionType::Edit {
        file_path: "test.rs".to_string(),
        diff_lines: vec![],
    });

    assert!(id.starts_with("toolu_"));
    match &content[0] {
        ContentBlock::ToolUse { name, input, .. } => {
            assert_eq!(name, "Edit");
            assert_eq!(input["file_path"], "test.rs");
        }
        _ => panic!("Expected ToolUse content block"),
    }
}
```

**Verification:** `cargo test --all` passes.

## Key Implementation Details

### Tool Use ID Generation

Tool use IDs follow the Claude API format: `toolu_{uuid}` (32-char UUID without hyphens).

```rust
let tool_use_id = format!("toolu_{}", uuid::Uuid::new_v4().simple());
```

### Message Linking Chain

The JSONL messages are linked via UUIDs:

1. **User message** (`pending_user_uuid`): User's prompt that triggers tool use
2. **Assistant tool_use** (`pending_assistant_uuid`): Claude's decision to use a tool, links to user via `parentUuid`
3. **Tool result** (user message): Result of tool execution, links to assistant via `parentUuid` and `sourceToolAssistantUUID`

### stop_reason Values

- `"tool_use"` - Assistant message with tool_use blocks (waiting for result)
- `"end_turn"` - Final assistant response (turn complete)

### Error Handling

JSONL write errors are logged but don't fail the TUI operation:

```rust
if let Err(e) = writer.write().record_tool_result(...) {
    tracing::warn!("Failed to write tool result JSONL: {}", e);
}
```

### ContentBlock Import

Import from the state module:

```rust
use crate::state::ContentBlock;
```

## Verification Plan

1. **Unit Tests**
   - `build_tool_use_content()` produces correct content blocks
   - Tool use ID format is correct (`toolu_*`)

2. **Integration Tests**
   - Permission dialog triggers tool_use message
   - Permission confirmation triggers tool_result message
   - Bypass mode produces paired tool_use/tool_result
   - Session grants produce paired tool_use/tool_result

3. **Manual Testing**
   ```bash
   # Start TUI with state dir
   export CLAUDE_LOCAL_STATE_DIR=/tmp/claudeless-test
   cargo run

   # In another terminal, watch JSONL
   tail -f /tmp/claudeless-test/projects/*/*.jsonl | jq .

   # In TUI: type "test bash permission" to trigger permission dialog
   # Confirm permission
   # Check JSONL has tool_use and tool_result entries
   ```

4. **Pre-commit Checks**
   - `make check` passes
   - `cargo clippy --all-targets --all-features -- -D warnings` clean
   - `cargo test --all` passes
