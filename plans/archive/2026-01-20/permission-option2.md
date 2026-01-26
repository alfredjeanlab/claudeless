# Permission Option 2 Context-Sensitive Text

**Root Feature:** `cl-5246`

## Overview

Make the Bash permission dialog's option 2 text context-sensitive based on the actual command being executed. Currently, it hardcodes "Yes, allow reading from etc/ from this project" for all Bash commands. After this change, the text will reflect what the command actually does (e.g., "npm commands", "rm commands", "reading from etc/").

## Project Structure

```
crates/cli/src/tui/widgets/
├── permission.rs         # Main implementation (lines 89-97)
├── permission_tests.rs   # Unit tests for permission dialogs
└── mod.rs               # Module exports
```

**Key Files:**
- `crates/cli/src/tui/widgets/permission.rs` - Contains `RichPermissionDialog::option2_text()` method that needs modification

## Dependencies

No external dependencies required. This is a pure Rust string manipulation feature using standard library functions.

## Implementation Phases

### Phase 1: Add Command Category Detection

Add a helper function to categorize Bash commands and extract the relevant context.

**Location:** `crates/cli/src/tui/widgets/permission.rs`

```rust
/// Categories of bash commands for permission text generation
#[derive(Debug, Clone, PartialEq, Eq)]
enum BashCommandCategory {
    /// Command reads from /etc/ directory
    ReadingEtc,
    /// Named command (npm, cargo, git, rm, etc.)
    NamedCommand(String),
    /// Fallback for complex or unrecognized commands
    Generic,
}

/// Categorize a bash command for permission text generation.
///
/// Priority:
/// 1. If command contains `/etc/` path, categorize as ReadingEtc
/// 2. Extract first word as the command name
/// 3. Fallback to Generic for empty or unrecognizable commands
fn categorize_bash_command(command: &str) -> BashCommandCategory {
    // Check for /etc/ access first (highest priority)
    if command.contains("/etc/") {
        return BashCommandCategory::ReadingEtc;
    }

    // Extract the first word (command name)
    let first_word = command
        .split_whitespace()
        .next()
        .unwrap_or("");

    // Handle commands with paths (e.g., /usr/bin/npm -> npm)
    let command_name = first_word
        .rsplit('/')
        .next()
        .unwrap_or(first_word);

    if command_name.is_empty() {
        BashCommandCategory::Generic
    } else {
        BashCommandCategory::NamedCommand(command_name.to_string())
    }
}
```

### Phase 2: Update `option2_text()` Method

Modify the `option2_text()` method to use the new categorization.

**Location:** `crates/cli/src/tui/widgets/permission.rs:89-97`

```rust
/// Get the option 2 text based on permission type
fn option2_text(&self) -> String {
    match &self.permission_type {
        PermissionType::Bash { command, .. } => {
            match categorize_bash_command(command) {
                BashCommandCategory::ReadingEtc => {
                    "Yes, allow reading from etc/ from this project".to_string()
                }
                BashCommandCategory::NamedCommand(name) => {
                    format!("Yes, allow {} commands from this project", name)
                }
                BashCommandCategory::Generic => {
                    "Yes, allow this command from this project".to_string()
                }
            }
        }
        PermissionType::Edit { .. } | PermissionType::Write { .. } => {
            "Yes, allow all edits during this session (shift+tab)".to_string()
        }
    }
}
```

**Note:** Return type changes from `&'static str` to `String` to accommodate dynamic text.

### Phase 3: Update Render Method Signature

The `render()` method already uses `self.option2_text()` correctly, but verify it handles the `String` return type (it does, via `format!`).

### Phase 4: Add Unit Tests

Add comprehensive tests for the new categorization logic.

**Location:** `crates/cli/src/tui/widgets/permission_tests.rs`

```rust
// =========================================================================
// Bash Command Categorization Tests
// =========================================================================

#[test]
fn test_categorize_etc_reading() {
    assert_eq!(
        categorize_bash_command("cat /etc/passwd"),
        BashCommandCategory::ReadingEtc
    );
    assert_eq!(
        categorize_bash_command("cat /etc/passwd | head -5"),
        BashCommandCategory::ReadingEtc
    );
    assert_eq!(
        categorize_bash_command("ls /etc/"),
        BashCommandCategory::ReadingEtc
    );
}

#[test]
fn test_categorize_named_commands() {
    assert_eq!(
        categorize_bash_command("npm test"),
        BashCommandCategory::NamedCommand("npm".to_string())
    );
    assert_eq!(
        categorize_bash_command("rm -rf /tmp/foo"),
        BashCommandCategory::NamedCommand("rm".to_string())
    );
    assert_eq!(
        categorize_bash_command("cargo build --release"),
        BashCommandCategory::NamedCommand("cargo".to_string())
    );
    assert_eq!(
        categorize_bash_command("git status"),
        BashCommandCategory::NamedCommand("git".to_string())
    );
}

#[test]
fn test_categorize_commands_with_paths() {
    assert_eq!(
        categorize_bash_command("/usr/bin/npm test"),
        BashCommandCategory::NamedCommand("npm".to_string())
    );
    assert_eq!(
        categorize_bash_command("./scripts/build.sh"),
        BashCommandCategory::NamedCommand("build.sh".to_string())
    );
}

#[test]
fn test_categorize_empty_or_whitespace() {
    assert_eq!(
        categorize_bash_command(""),
        BashCommandCategory::Generic
    );
    assert_eq!(
        categorize_bash_command("   "),
        BashCommandCategory::Generic
    );
}

// =========================================================================
// Option 2 Text Integration Tests
// =========================================================================

#[test]
fn test_option2_text_etc_reading() {
    let dialog = RichPermissionDialog::new(PermissionType::Bash {
        command: "cat /etc/passwd".to_string(),
        description: None,
    });
    let output = dialog.render(120);
    assert!(output.contains("Yes, allow reading from etc/ from this project"));
}

#[test]
fn test_option2_text_npm_command() {
    let dialog = RichPermissionDialog::new(PermissionType::Bash {
        command: "npm test".to_string(),
        description: None,
    });
    let output = dialog.render(120);
    assert!(output.contains("Yes, allow npm commands from this project"));
}

#[test]
fn test_option2_text_rm_command() {
    let dialog = RichPermissionDialog::new(PermissionType::Bash {
        command: "rm -rf /tmp/test".to_string(),
        description: None,
    });
    let output = dialog.render(120);
    assert!(output.contains("Yes, allow rm commands from this project"));
}
```

### Phase 5: Update Existing Tests

Update the existing test that checks for hardcoded text.

**Location:** `crates/cli/src/tui/widgets/permission_tests.rs:59`

```rust
// Change from:
assert!(output.contains("2. Yes, allow reading from etc/"));
// To:
assert!(output.contains("2. Yes, allow reading from etc/ from this project"));
```

This test already uses a command with `/etc/passwd`, so it should still pass.

### Phase 6: Run Verification

Run `make check` to ensure all tests pass and code meets project standards.

## Key Implementation Details

### Command Parsing Strategy

The categorization uses a simple but effective approach:

1. **Path Detection First**: Check for `/etc/` anywhere in the command string. This catches both direct commands (`cat /etc/passwd`) and piped commands (`cat /etc/passwd | head`).

2. **First Word Extraction**: Extract the first whitespace-separated token as the command name.

3. **Path Stripping**: Handle commands invoked with full paths by taking everything after the last `/`.

### Edge Cases Handled

| Input | Category | Option 2 Text |
|-------|----------|---------------|
| `cat /etc/passwd` | ReadingEtc | "Yes, allow reading from etc/ from this project" |
| `npm test` | NamedCommand("npm") | "Yes, allow npm commands from this project" |
| `/usr/bin/rm -rf foo` | NamedCommand("rm") | "Yes, allow rm commands from this project" |
| `./build.sh` | NamedCommand("build.sh") | "Yes, allow build.sh commands from this project" |
| `""` (empty) | Generic | "Yes, allow this command from this project" |

### Return Type Change

The `option2_text()` method return type changes from `&'static str` to `String` because we now generate dynamic text. This is a minor performance consideration but acceptable for UI code that doesn't run in tight loops.

## Verification Plan

1. **Unit Tests**: Run new and updated tests in `permission_tests.rs`
   ```bash
   cargo test -p cli permission
   ```

2. **Full Test Suite**: Run all project tests
   ```bash
   cargo test --all
   ```

3. **Linting**: Verify code style and quality
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   ```

4. **Complete Check**: Run full project validation
   ```bash
   make check
   ```

5. **Manual Verification**: Create a test scenario with different commands:
   - `cat /etc/passwd` → should show "reading from etc/"
   - `npm test` → should show "npm commands"
   - `rm -rf /tmp/foo` → should show "rm commands"
   - `cargo build` → should show "cargo commands"
