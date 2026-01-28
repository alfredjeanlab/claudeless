# Implementation Plan: Remove --tui and --no-tui Flags

## Overview

Remove the simulator-specific `--tui` and `--no-tui` CLI flags from claudeless to improve compatibility with the real Claude CLI. These flags are unnecessary because:

- TUI mode auto-detects based on stdin being a TTY
- `--print` mode already implies non-TUI operation

## Project Structure

Files requiring changes:

```
crates/cli/src/cli.rs          # Remove flags and simplify should_use_tui()
docs/USAGE.md                  # Remove documentation for these flags
scripts/capture-tui.sh         # Update comment example
scripts/compare-tui.sh         # Remove --tui from simulator command
crates/cli/tests/common/mod.rs # Update test helper functions
crates/cli/tests/tui_setup.rs  # Remove --tui from test commands
crates/cli/tests/tui_permission.rs
crates/cli/tests/tui_snapshot.rs
```

## Dependencies

No new dependencies required. This is a removal-only change.

## Implementation Phases

### Phase 1: Remove Flags from CLI Struct

Update `crates/cli/src/cli.rs`:

1. Remove the `tui` field (lines 135-137):
   ```rust
   /// Enable TUI mode (interactive terminal interface)
   #[arg(long, env = "CLAUDELESS_TUI")]
   pub tui: bool,
   ```

2. Remove the `no_tui` field (lines 139-141):
   ```rust
   /// Force non-TUI mode even if stdin is a TTY
   #[arg(long)]
   pub no_tui: bool,
   ```

3. Simplify `should_use_tui()` method (lines 174-186) to:
   ```rust
   impl Cli {
       /// Determine if TUI mode should be used
       pub fn should_use_tui(&self) -> bool {
           use std::io::IsTerminal;
           !self.print && std::io::stdin().is_terminal()
       }
   }
   ```

**Verification:** `cargo check -p claudeless` succeeds.

---

### Phase 2: Update Documentation

Update `docs/USAGE.md`:

Remove lines 27-28 from the CLI Flags table:
```diff
 | `--delay-ms <MS>` | `CLAUDELESS_DELAY_MS` | Response delay in milliseconds |
-| `--tui` | `CLAUDELESS_TUI` | Force TUI mode |
-| `--no-tui` | — | Force non-TUI mode |
 | `--tool-mode <MODE>` | `CLAUDELESS_TOOL_MODE` | Tool execution mode |
```

**Verification:** Documentation is accurate and complete.

---

### Phase 3: Update Helper Scripts

**`scripts/compare-tui.sh` (line 110):**

Remove `--tui` flag from the simulator command:
```diff
-        SIM_CMD="$SIM_BIN --scenario $SCENARIO --tui"
+        SIM_CMD="$SIM_BIN --scenario $SCENARIO"
```

**`scripts/capture-tui.sh` (line 27):**

Update comment example:
```diff
-#   capture-tui.sh -o sim_state.txt 'claudeless --scenario test.json --tui'
+#   capture-tui.sh -o sim_state.txt 'claudeless --scenario test.json'
```

**Verification:** Scripts run without argument errors.

---

### Phase 4: Update Integration Tests

Update test files to remove `--tui` flag. These tests run in tmux which provides a PTY, so stdin is automatically detected as a terminal.

**`crates/cli/tests/common/mod.rs`:**

Update `start_tui_ext()` (line 70):
```diff
-        "{} --scenario {} --tui",
+        "{} --scenario {}",
```

Update `capture_tui_initial()` (line 96):
```diff
-        "{} --scenario {} --tui {}",
+        "{} --scenario {} {}",
```

Update `start_tui_with_env()` (line 358):
```diff
-        "{} --scenario {} --tui",
+        "{} --scenario {}",
```

**`crates/cli/tests/tui_setup.rs`:**

Remove `--tui` from all test commands (lines 58, 132, 182, 234, 289, 356, 503, 553):
```diff
-        "{} --scenario {} --tui",
+        "{} --scenario {}",
```

**`crates/cli/tests/tui_permission.rs` (line 303):**
```diff
-        "{} --scenario {} --tui",
+        "{} --scenario {}",
```

**`crates/cli/tests/tui_snapshot.rs` (line 70):**
```diff
-        "{} --scenario {} --tui",
+        "{} --scenario {}",
```

**Verification:** `cargo test --all` passes.

---

### Phase 5: Full Build Verification

Run the complete verification suite:

```bash
make check
```

This validates:
- Formatting (`cargo fmt --check`)
- Linting (`cargo clippy`)
- Tests (`cargo test --all`)
- Build (`cargo build --all`)
- Publish dry-run (`cargo publish --dry-run`)

**Verification:** All checks pass with no errors.

## Key Implementation Details

### Why Tests Still Work

The integration tests run claudeless inside tmux sessions. Tmux allocates a PTY (pseudo-terminal) for each session, which means:
- The child process's stdin is connected to the PTY
- `std::io::stdin().is_terminal()` returns `true`
- TUI mode activates automatically without needing `--tui`

### Simplified Logic

Before:
```rust
pub fn should_use_tui(&self) -> bool {
    if self.no_tui {
        return false;
    }
    if self.tui {
        return true;
    }
    !self.print && std::io::stdin().is_terminal()
}
```

After:
```rust
pub fn should_use_tui(&self) -> bool {
    use std::io::IsTerminal;
    !self.print && std::io::stdin().is_terminal()
}
```

### Removed Environment Variable

`CLAUDELESS_TUI` environment variable is removed as it was only used by the `--tui` flag.

## Verification Plan

1. **Compile check:** `cargo check -p claudeless`
2. **Unit tests:** `cargo test -p claudeless`
3. **Integration tests:** `cargo test --all` (includes tmux-based TUI tests)
4. **Manual TUI test:** Run `claudeless --scenario scenarios/simple.toml` in a terminal to verify TUI activates
5. **Manual print mode test:** Run `claudeless --scenario scenarios/simple.toml -p "hello"` to verify non-TUI mode
6. **Full CI check:** `make check`
