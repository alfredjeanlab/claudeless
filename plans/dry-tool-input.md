# Plan: DRY Tool Input Extraction

## Problem

The builtin tool executors duplicate input extraction logic across 5 files:

| Function | Files |
|----------|-------|
| `extract_path` | `read.rs`, `write.rs`, `edit.rs`, `glob.rs`, `grep.rs` |
| `extract_pattern` | `glob.rs`, `grep.rs` |

Each tool implements its own version checking `"file_path"`, `"path"`, `"directory"`, or `"pattern"` keys.

## Files to Modify

- `crates/cli/src/tools/builtin/mod.rs` - Add shared input helpers
- `crates/cli/src/tools/builtin/read.rs` - Use shared helper
- `crates/cli/src/tools/builtin/write.rs` - Use shared helper
- `crates/cli/src/tools/builtin/edit.rs` - Use shared helper
- `crates/cli/src/tools/builtin/glob.rs` - Use shared helper
- `crates/cli/src/tools/builtin/grep.rs` - Use shared helper
- `crates/cli/src/tools/builtin/bash.rs` - Use shared helper (for `extract_command`)

## Implementation

### Step 1: Add shared input module

Create `crates/cli/src/tools/builtin/input.rs`:

```rust
//! Shared input extraction helpers for builtin tools.

use serde_json::Value;

/// Extract a file path from tool input.
/// Checks "file_path" first, then "path" as fallback.
pub fn extract_file_path(input: &Value) -> Option<&str> {
    input
        .get("file_path")
        .or_else(|| input.get("path"))
        .and_then(|v| v.as_str())
}

/// Extract a directory/path from tool input.
/// Checks "path" first, then "directory" as fallback.
pub fn extract_directory(input: &Value) -> Option<&str> {
    input
        .get("path")
        .or_else(|| input.get("directory"))
        .and_then(|v| v.as_str())
}

/// Extract a pattern from tool input.
pub fn extract_pattern(input: &Value) -> Option<&str> {
    input.get("pattern").and_then(|v| v.as_str())
}

/// Extract a command from tool input.
pub fn extract_command(input: &Value) -> Option<&str> {
    input.get("command").and_then(|v| v.as_str())
}

/// Extract a string field by name.
pub fn extract_str<'a>(input: &'a Value, key: &str) -> Option<&'a str> {
    input.get(key).and_then(|v| v.as_str())
}

/// Extract a boolean field with default.
pub fn extract_bool(input: &Value, key: &str, default: bool) -> bool {
    input.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}
```

### Step 2: Update mod.rs

Add to `mod.rs`:
```rust
mod input;
pub use input::*;
```

### Step 3: Update each tool

Example for `read.rs`:
```rust
// Before
fn extract_path(input: &serde_json::Value) -> Option<&str> {
    input.get("file_path").or_else(|| input.get("path")).and_then(|v| v.as_str())
}

// After
use super::input::extract_file_path;
// Remove extract_path method entirely, use extract_file_path(&call.input)
```

Similar changes for `write.rs`, `edit.rs`, `glob.rs`, `grep.rs`, `bash.rs`.

### Step 4: Update edit.rs specific helpers

`edit.rs` also has `extract_old_string`, `extract_new_string`, `replace_all` - keep these as they're edit-specific, but use the generic `extract_str` and `extract_bool`:

```rust
let old_string = extract_str(&call.input, "old_string");
let new_string = extract_str(&call.input, "new_string");
let replace_all = extract_bool(&call.input, "replace_all", false);
```

## Testing

- Existing tests in `*_tests.rs` files should pass unchanged
- Add unit tests for the new `input.rs` module

## Lines Changed

- ~60 lines removed (duplicate functions)
- ~40 lines added (shared module)
- Net: ~20 lines reduced
