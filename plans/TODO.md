# TODO

Follow-up items from completed features.

## Permission Dialogs (from fix-permission-dialogs.md)

### Context-Sensitive Option 2 Text

The Bash permission dialog hardcodes "Yes, allow reading from etc/ from this project" regardless of the actual command. This should be context-sensitive based on what the command does.

**Current behavior:**
```
 Do you want to proceed?
 ❯ 1. Yes
   2. Yes, allow reading from etc/ from this project  <-- always this
   3. No
```

**Expected behavior:**
- For `cat /etc/passwd`: "Yes, allow reading from etc/ from this project"
- For `npm test`: "Yes, allow npm commands from this project"
- For `rm -rf`: "Yes, allow rm commands from this project"

**Location:** `crates/cli/src/tui/widgets/permission.rs:92-94`

### Session-Level Permission Persistence

The `PermissionSelection::YesSession` choice is implemented in the UI but the actual session-level grant isn't persisted. Currently it just prints "[Permission granted for session]" without actually remembering the grant.

**To implement:**
1. Track session-granted permissions in `TuiAppStateInner`
2. Check against session grants before showing permission dialog
3. Clear session grants when session ends

**Location:** `crates/cli/src/tui/app.rs:949-954`
