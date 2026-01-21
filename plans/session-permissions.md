# Implementation Plan: Session-Level Permission Persistence

## Overview

Implement session-level permission persistence so that when a user selects "Yes, allow for session" (`PermissionSelection::YesSession`), the permission grant is remembered for the remainder of the session. Currently, the UI option exists but selecting it only prints a message without actually remembering the grant.

Key changes:
1. Add a `SessionPermissionGrants` data structure to track granted permissions
2. Store grants in `TuiAppStateInner` when `YesSession` is selected
3. Check against session grants before showing permission dialogs
4. Clear session grants on `/clear` command or session end

## Project Structure

Key files to modify:

```
crates/cli/src/tui/
├── app.rs                    # Main implementation location
│   ├── TuiAppStateInner      # Add session_grants field
│   ├── confirm_permission()  # Store grants when YesSession selected
│   ├── show_permission_request() # Check grants before showing dialog
│   └── handle_command_inner() # Clear grants on /clear
├── app_tests.rs              # Add unit tests for session grants
│
crates/cli/src/tui/widgets/
├── permission.rs             # Add SessionPermissionKey type
└── permission_tests.rs       # Add tests for key extraction
```

## Dependencies

No new external dependencies required. Uses existing standard library types:
- `std::collections::HashSet` for storing granted permission keys

## Implementation Phases

### Phase 1: Define Session Permission Key Type

**Goal:** Create a type to uniquely identify permission requests for session-level matching.

**Files:**
- `crates/cli/src/tui/widgets/permission.rs`
- `crates/cli/src/tui/widgets/permission_tests.rs`

**Implementation:**

Add a new type that can be used as a key for tracking granted permissions:

```rust
/// Key for identifying session-level permission grants
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SessionPermissionKey {
    /// Bash command matching by prefix (e.g., "cat /etc/" grants all "cat /etc/*")
    BashPrefix(String),
    /// Edit permission for all files (session-level edit grants apply to all edits)
    EditAll,
    /// Write permission for all files (session-level write grants apply to all writes)
    WriteAll,
}

impl RichPermissionDialog {
    /// Extract the session permission key for this dialog
    pub fn session_key(&self) -> SessionPermissionKey {
        match &self.permission_type {
            PermissionType::Bash { command, .. } => {
                // Extract meaningful prefix for bash commands
                // For "cat /etc/passwd | head -5", extract "cat /etc/"
                SessionPermissionKey::BashPrefix(extract_bash_prefix(command))
            }
            PermissionType::Edit { .. } => SessionPermissionKey::EditAll,
            PermissionType::Write { .. } => SessionPermissionKey::WriteAll,
        }
    }
}

/// Extract a prefix from a bash command for permission matching
fn extract_bash_prefix(command: &str) -> String {
    // For now, use the first "word path" segment
    // e.g., "cat /etc/passwd" -> "cat /etc/"
    // e.g., "npm test" -> "npm"
    // This matches the option 2 text pattern
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return command.to_string();
    }

    // If second part looks like a path, include the directory
    if parts.len() > 1 && parts[1].starts_with('/') {
        if let Some(dir_end) = parts[1].rfind('/') {
            return format!("{} {}", parts[0], &parts[1][..=dir_end]);
        }
    }

    parts[0].to_string()
}
```

**Verification:**
- [ ] `extract_bash_prefix("cat /etc/passwd | head -5")` returns `"cat /etc/"`
- [ ] `extract_bash_prefix("npm test")` returns `"npm"`
- [ ] `extract_bash_prefix("rm -rf /tmp/foo")` returns `"rm"`
- [ ] `session_key()` returns `EditAll` for edit permissions
- [ ] `session_key()` returns `WriteAll` for write permissions

---

### Phase 2: Add Session Grants Storage to TuiAppStateInner

**Goal:** Add storage for session-granted permissions in the TUI state.

**Files:**
- `crates/cli/src/tui/app.rs`

**Changes:**

1. Add import for `HashSet`:
```rust
use std::collections::HashSet;
use super::widgets::permission::SessionPermissionKey;
```

2. Add field to `TuiAppStateInner`:
```rust
struct TuiAppStateInner {
    // ... existing fields ...

    /// Session-level permission grants
    /// Permissions granted with "Yes, allow for session" are stored here
    pub session_grants: HashSet<SessionPermissionKey>,
}
```

3. Initialize in `TuiAppState::new()`:
```rust
TuiAppStateInner {
    // ... existing initialization ...
    session_grants: HashSet::new(),
}
```

**Verification:**
- [ ] `TuiAppStateInner` compiles with new field
- [ ] New `TuiAppState` instances have empty `session_grants`

---

### Phase 3: Store Grants When YesSession Selected

**Goal:** When user selects "Yes, allow for session", store the permission grant.

**Files:**
- `crates/cli/src/tui/app.rs`

**Changes:**

Update `confirm_permission()` method (around line 1050):

```rust
fn confirm_permission(&self) {
    let mut inner = self.inner.lock();
    let perm = inner.pending_permission.take();
    inner.mode = AppMode::Input;

    if let Some(perm) = perm {
        let tool_name = match &perm.dialog.permission_type {
            PermissionType::Bash { command, .. } => format!("Bash: {}", command),
            PermissionType::Edit { file_path, .. } => format!("Edit: {}", file_path),
            PermissionType::Write { file_path, .. } => format!("Write: {}", file_path),
        };

        match perm.dialog.selected {
            PermissionSelection::Yes => {
                // Continue with tool execution (single request only)
                inner
                    .response_content
                    .push_str(&format!("\n[Permission granted for {}]\n", tool_name));
            }
            PermissionSelection::YesSession => {
                // Store session-level grant
                let key = perm.dialog.session_key();
                inner.session_grants.insert(key);

                // Continue with tool execution
                inner.response_content.push_str(&format!(
                    "\n[Permission granted for session: {}]\n",
                    tool_name
                ));
            }
            PermissionSelection::No => {
                inner
                    .response_content
                    .push_str(&format!("\n[Permission denied for {}]\n", tool_name));
            }
        }
    }
}
```

**Verification:**
- [ ] Selecting "Yes" does NOT add to session_grants
- [ ] Selecting "Yes, allow for session" adds key to session_grants
- [ ] Selecting "No" does NOT add to session_grants
- [ ] Correct session key is extracted for each permission type

---

### Phase 4: Check Session Grants Before Showing Dialog

**Goal:** Skip permission dialog if a matching session grant exists.

**Files:**
- `crates/cli/src/tui/app.rs`

**Changes:**

1. Add helper method to check if permission is granted:
```rust
impl TuiAppState {
    /// Check if a permission is already granted for this session
    fn is_session_granted(&self, permission_type: &PermissionType) -> bool {
        let inner = self.inner.lock();

        // Create a temporary dialog to extract the key
        let dialog = RichPermissionDialog::new(permission_type.clone());
        let key = dialog.session_key();

        inner.session_grants.contains(&key)
    }
}
```

2. Update `show_permission_request()` to check grants first:
```rust
pub fn show_permission_request(&self, permission_type: PermissionType) {
    // Check if this permission type is already granted for the session
    if self.is_session_granted(&permission_type) {
        // Auto-approve without showing dialog
        let mut inner = self.inner.lock();
        let tool_name = match &permission_type {
            PermissionType::Bash { command, .. } => format!("Bash: {}", command),
            PermissionType::Edit { file_path, .. } => format!("Edit: {}", file_path),
            PermissionType::Write { file_path, .. } => format!("Write: {}", file_path),
        };
        inner
            .response_content
            .push_str(&format!("\n[Permission auto-granted (session): {}]\n", tool_name));
        return;
    }

    // Show dialog as normal
    let mut inner = self.inner.lock();
    inner.pending_permission = Some(PermissionRequest {
        dialog: RichPermissionDialog::new(permission_type),
    });
    inner.mode = AppMode::Permission;
}
```

**Verification:**
- [ ] First bash permission request shows dialog
- [ ] After "Yes, allow for session", same-prefix bash command auto-approves
- [ ] After "Yes, allow for session" for edit, subsequent edits auto-approve
- [ ] "Yes" (single grant) does NOT auto-approve subsequent requests
- [ ] Different permission types are tracked independently

---

### Phase 5: Clear Session Grants on /clear

**Goal:** Clear session grants when the user runs `/clear` command.

**Files:**
- `crates/cli/src/tui/app.rs`

**Changes:**

Update `handle_command_inner()` for the `/clear` command:

```rust
"/clear" => {
    // Clear session turns
    {
        let mut sessions = inner.sessions.lock();
        sessions.current_session().turns.clear();
    }

    // Reset token counts
    inner.status.input_tokens = 0;
    inner.status.output_tokens = 0;

    // Clear session-level permission grants
    inner.session_grants.clear();

    // Set response content (will be rendered with elbow connector)
    inner.response_content = "(no content)".to_string();
}
```

**Verification:**
- [ ] After `/clear`, previously granted session permissions require re-prompting
- [ ] Session grants are empty after `/clear`

---

### Phase 6: Add Unit Tests

**Goal:** Comprehensive test coverage for session permission grants.

**Files:**
- `crates/cli/src/tui/app_tests.rs`
- `crates/cli/src/tui/widgets/permission_tests.rs`

**Permission Widget Tests (`permission_tests.rs`):**

```rust
#[test]
fn test_bash_prefix_extraction_with_path() {
    let result = extract_bash_prefix("cat /etc/passwd | head -5");
    assert_eq!(result, "cat /etc/");
}

#[test]
fn test_bash_prefix_extraction_simple() {
    let result = extract_bash_prefix("npm test");
    assert_eq!(result, "npm");
}

#[test]
fn test_session_key_bash() {
    let dialog = RichPermissionDialog::new(PermissionType::Bash {
        command: "cat /etc/passwd".to_string(),
        description: None,
    });
    assert!(matches!(dialog.session_key(), SessionPermissionKey::BashPrefix(_)));
}

#[test]
fn test_session_key_edit() {
    let dialog = RichPermissionDialog::new(PermissionType::Edit {
        file_path: "foo.txt".to_string(),
        diff_lines: vec![],
    });
    assert_eq!(dialog.session_key(), SessionPermissionKey::EditAll);
}

#[test]
fn test_session_key_write() {
    let dialog = RichPermissionDialog::new(PermissionType::Write {
        file_path: "foo.txt".to_string(),
        content_lines: vec![],
    });
    assert_eq!(dialog.session_key(), SessionPermissionKey::WriteAll);
}
```

**App Tests (`app_tests.rs`):**

```rust
#[test]
fn test_session_grant_not_stored_for_single_yes() {
    let state = create_test_state();
    state.show_bash_permission("cat /etc/passwd".to_string(), None);

    // Select "Yes" (single grant)
    {
        let mut inner = state.inner.lock();
        inner.pending_permission.as_mut().unwrap().dialog.selected = PermissionSelection::Yes;
    }
    state.confirm_permission();

    // Verify no session grant stored
    let inner = state.inner.lock();
    assert!(inner.session_grants.is_empty());
}

#[test]
fn test_session_grant_stored_for_yes_session() {
    let state = create_test_state();
    state.show_bash_permission("cat /etc/passwd".to_string(), None);

    // Select "Yes, allow for session"
    {
        let mut inner = state.inner.lock();
        inner.pending_permission.as_mut().unwrap().dialog.selected = PermissionSelection::YesSession;
    }
    state.confirm_permission();

    // Verify session grant stored
    let inner = state.inner.lock();
    assert!(!inner.session_grants.is_empty());
}

#[test]
fn test_session_grant_auto_approves_subsequent_request() {
    let state = create_test_state();

    // First request: grant for session
    state.show_bash_permission("cat /etc/passwd".to_string(), None);
    {
        let mut inner = state.inner.lock();
        inner.pending_permission.as_mut().unwrap().dialog.selected = PermissionSelection::YesSession;
    }
    state.confirm_permission();

    // Second request: should auto-approve (no pending permission)
    state.show_bash_permission("cat /etc/hosts".to_string(), None);

    let inner = state.inner.lock();
    assert!(inner.pending_permission.is_none()); // Auto-approved, no dialog
    assert!(inner.response_content.contains("auto-granted"));
}

#[test]
fn test_clear_command_clears_session_grants() {
    let state = create_test_state();

    // Grant session permission
    state.show_bash_permission("cat /etc/passwd".to_string(), None);
    {
        let mut inner = state.inner.lock();
        inner.pending_permission.as_mut().unwrap().dialog.selected = PermissionSelection::YesSession;
    }
    state.confirm_permission();

    // Run /clear
    {
        let mut inner = state.inner.lock();
        inner.input_buffer = "/clear".to_string();
    }
    state.submit_input();

    // Verify grants cleared
    let inner = state.inner.lock();
    assert!(inner.session_grants.is_empty());
}
```

**Verification:**
- [ ] All new unit tests pass
- [ ] `cargo test -p claudeless -- session` passes

---

## Key Implementation Details

### Session Permission Matching Strategy

| Permission Type | Session Key | Matching Behavior |
|-----------------|-------------|-------------------|
| Bash | `BashPrefix(prefix)` | Matches commands with same prefix (e.g., "cat /etc/") |
| Edit | `EditAll` | All file edits share one session grant |
| Write | `WriteAll` | All file writes share one session grant |

This matches Claude Code's actual behavior where:
- Edit/Write option 2 says "allow all edits during this session"
- Bash option 2 says "allow [action] from this project" (path-based)

### Bash Prefix Extraction Algorithm

```rust
fn extract_bash_prefix(command: &str) -> String {
    // Split command into parts
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return command.to_string();
    }

    // Check if second part is a path
    if parts.len() > 1 && parts[1].starts_with('/') {
        // Extract directory portion: "/etc/passwd" -> "/etc/"
        if let Some(last_slash) = parts[1].rfind('/') {
            return format!("{} {}", parts[0], &parts[1][..=last_slash]);
        }
    }

    // Default: just the command name
    parts[0].to_string()
}
```

### Thread Safety

The `session_grants` field is protected by the existing `Mutex<TuiAppStateInner>` wrapper, so no additional synchronization is needed.

---

## Verification Plan

### Unit Tests

**Permission Widget:**
- [ ] `test_bash_prefix_extraction_with_path`
- [ ] `test_bash_prefix_extraction_simple`
- [ ] `test_bash_prefix_extraction_no_path`
- [ ] `test_session_key_bash`
- [ ] `test_session_key_edit`
- [ ] `test_session_key_write`

**TUI App:**
- [ ] `test_session_grant_not_stored_for_single_yes`
- [ ] `test_session_grant_stored_for_yes_session`
- [ ] `test_session_grant_auto_approves_subsequent_request`
- [ ] `test_different_permission_types_tracked_independently`
- [ ] `test_clear_command_clears_session_grants`
- [ ] `test_no_grant_stored_for_denied_permission`

### Integration Tests

- [ ] Create scenario that triggers multiple permission requests
- [ ] Verify second request of same type is auto-approved after session grant
- [ ] Verify `/clear` resets session state

### Manual Testing

1. Run `claudeless` with test scenario
2. Trigger bash permission, select "Yes, allow for session"
3. Trigger another bash permission with same prefix
4. Verify no dialog appears (auto-approved)
5. Run `/clear`
6. Trigger bash permission again
7. Verify dialog appears (grant was cleared)

### Final Checklist

- [ ] `make check` passes
- [ ] No new clippy warnings
- [ ] All new tests pass
- [ ] Session grants work correctly for Bash, Edit, Write
- [ ] `/clear` properly resets session state
- [ ] Update `plans/TODO.md` to remove completed item
