# Epic 12f: Audit Permission Pattern Matching

**Scope:** Settings-based permission patterns (`permissions.allow`, `permissions.deny`)
**Claimed validation:** None (implementation based on documentation assumptions)

## Summary

| Aspect | Status |
|--------|--------|
| Pattern parsing implemented | ✅ |
| Pattern matching implemented | ✅ |
| Validated against real Claude | ❌ |
| Known syntax bugs | ✅ Yes - colon vs space |

## Known Bug: Wildcard Syntax

**Our implementation uses glob-style wildcards with spaces:**
```json
{
  "permissions": {
    "allow": ["Bash(npm *)"],
    "deny": ["Bash(rm -rf *)"]
  }
}
```

**Real Claude uses colon-prefix syntax:**
```json
{
  "permissions": {
    "allow": ["Bash(npm:*)"],
    "deny": ["Bash(rm -rf:*)"]
  }
}
```

### Pattern Syntax Comparison

| Pattern Type | Our Implementation | Real Claude | Match? |
|--------------|-------------------|-------------|--------|
| Exact match | `Bash(npm test)` | `Bash(npm test)` | ✅ |
| Starts with | `Bash(npm *)` | `Bash(npm:*)` | ❌ |
| Starts with | `Bash(rm -rf *)` | `Bash(rm -rf:*)` | ❌ |
| File pattern | `Write(*.md)` | `Write(*.md)` | ⚠️ Unverified |

### Semantic Difference

**Our glob approach:**
```rust
// Bash(npm *) uses shell glob matching
glob::Pattern::new("npm *").matches("npm test")  // true
glob::Pattern::new("npm *").matches("npm")       // false (no space match)
```

**Real Claude's prefix approach:**
```
// Bash(npm:*) means "starts with npm"
"npm test".starts_with("npm")  // true
"npm".starts_with("npm")       // true
"npmfoo".starts_with("npm")    // true (probably?)
```

## Other Unvalidated Behaviors

| Behavior | Our Assumption | Real Claude | Validated? |
|----------|---------------|-------------|------------|
| Pattern syntax | Glob (`*`, `?`, `[a-z]`) | Prefix (`:*`) | ❌ |
| Case sensitivity | Tool names case-insensitive | ? | ❌ |
| Priority order | deny > allow | ? | ❌ |
| Argument extraction | Full command string | ? | ❌ |
| Multiple wildcards | Supported (`*foo*`) | ? | ❌ |

## Files Affected

```
crates/cli/src/permission/pattern.rs    # ToolPattern parsing
crates/cli/src/permission/check.rs      # PermissionChecker integration
crates/cli/tests/settings_permissions.rs # All pattern tests
```

## Validation Approach

The permission rules are straightforward. Rather than running real Claude, we:

1. **Build on prior validation work** - TUI, CLI, and dotclaude epics validate output formats against real Claude
2. **Unit test the permission logic** - pattern matching, priority order
3. **Integration tests** - combine settings files with expected TUI/dotclaude output
4. **Purpose** - document and verify simulator behavior to prevent regressions

### Unit Tests Required

| Behavior | Test |
|----------|------|
| Prefix matching | `Bash(npm:*)` matches `npm test`, `npm`, but not `npx` |
| Exact matching | `Bash(npm test)` matches only `npm test` |
| Deny beats allow | Both match → denied |
| Priority order | bypass > scenario > deny > allow > mode |
| Case insensitivity | `Bash` matches `bash`, `BASH` |

### Integration Tests Required

Combine permissions with validated TUI/dotclaude output:

```toml
# Scenario: test-permission-allow.toml
[tool_execution]
mode = "simulated"

# Settings file has: {"permissions": {"allow": ["Bash(npm:*)"]}}
# Expected: tool auto-approved, no permission prompt in TUI
# Expected: dotclaude session shows tool execution without prompt
```

```toml
# Scenario: test-permission-deny.toml
[tool_execution]
mode = "simulated"

# Settings file has: {"permissions": {"deny": ["Bash(rm:*)"]}}
# Expected: tool denied, TUI shows denial message
# Expected: dotclaude session shows denial
```

These tests verify the simulator's behavior is consistent and documented.

## Recommendations

### 1. Fix pattern syntax

Update `src/permission/pattern.rs`:

```rust
impl ToolPattern {
    pub fn parse(s: &str) -> Option<Self> {
        // ... existing tool name extraction ...

        // Check for :* suffix (prefix matching)
        if arg.ends_with(":*") {
            let prefix = &arg[..arg.len() - 2];
            return Some(Self {
                tool,
                argument: Some(CompiledPattern::Prefix(prefix.to_string())),
            });
        }

        // Exact match (no wildcard)
        Some(Self {
            tool,
            argument: Some(CompiledPattern::Exact(arg.to_string())),
        })
    }
}

pub enum CompiledPattern {
    Exact(String),
    Prefix(String),  // New: for :* patterns
    // Remove Glob variant or keep for file patterns only
}
```

### 3. Update tests

All tests using `Bash(npm *)` syntax need to change to `Bash(npm:*)`:

```rust
// BEFORE (wrong)
let pattern = ToolPattern::parse("Bash(npm *)").unwrap();
assert!(pattern.matches("Bash", Some("npm test")));

// AFTER (correct)
let pattern = ToolPattern::parse("Bash(npm:*)").unwrap();
assert!(pattern.matches("Bash", Some("npm test")));
```

## Regression Prevention

Integration tests that combine permissions with TUI/dotclaude output:

```
tests/
├── permission_integration.rs     # Tests combining settings + scenarios
└── fixtures/
    └── scenarios/
        ├── permission-allow.toml     # Auto-approve scenario
        ├── permission-deny.toml      # Denial scenario
        └── permission-priority.toml  # Priority order scenario
```

Each test verifies:
1. Permission decision matches expected (allowed/denied/prompted)
2. TUI output matches expected format (from TUI validation work)
3. dotclaude output matches expected format (from dotclaude validation work)

## Required Fixes

See [epic-05x-fix-permissions.md](epic-05x-fix-permissions.md) for implementation.

1. **Change wildcard syntax** from glob (`npm *`) to prefix (`npm:*`)
2. **Update CompiledPattern enum** to use `Prefix` instead of `Glob` for Bash
3. **Verify file patterns** - does `Write(*.md)` use glob or different syntax?
4. **Update all tests** with correct syntax
5. **Add integration tests** combining permissions with TUI/dotclaude output

## Deliverables

1. Fix pattern syntax (`:*` prefix matching)
2. Unit tests for pattern matching behavior
3. Unit tests for priority order
4. Integration tests combining:
   - Settings files with permission patterns
   - Scenario-driven tool calls
   - Expected TUI output (validated in epic-05x-audit-tui)
   - Expected dotclaude output (validated in epic-05x-audit-dotclaude)
5. All tests pass and document expected simulator behavior

---

# Fix Permission Pattern Syntax

## Overview

Fix permission pattern syntax to match real Claude Code. The primary bug is wildcard syntax: we use glob-style `Bash(npm *)` but real Claude uses prefix-style `Bash(npm:*)`.

## Known Bugs

1. **Wildcard syntax:** `npm *` should be `npm:*`
2. **Glob matching:** We use shell glob, Claude uses prefix matching

## Phase 1: Update Pattern Parsing

**File:** `crates/cli/src/permission/pattern.rs`

### Task Agent Instructions

```
Update ToolPattern parsing to handle :* prefix syntax:

1. Change CompiledPattern enum:
   - Keep: Exact(String)
   - Add: Prefix(String)     // For :* patterns
   - Keep: Glob(Pattern)     // Only for file patterns like *.md

2. Update parse() method:
   - If argument ends with ":*", extract prefix and use Prefix variant
   - If argument contains glob chars (*, ?, [) but NOT :*, treat as file glob
   - Otherwise, exact match

3. Update matches() method:
   - Prefix: input.starts_with(&prefix)
   - Exact: input == exact
   - Glob: glob.matches(input)

Example transformations:
  "Bash(npm:*)"     → Prefix("npm")
  "Bash(npm test)"  → Exact("npm test")
  "Write(*.md)"     → Glob("*.md")
  "Bash(rm -rf:*)"  → Prefix("rm -rf")
```

### Validation

- [ ] `ToolPattern::parse("Bash(npm:*)")` returns `Prefix("npm")`
- [ ] `ToolPattern::parse("Bash(npm test)")` returns `Exact("npm test")`
- [ ] Pattern with `Prefix("npm")` matches "npm test", "npm install", "npm"
- [ ] Pattern with `Prefix("npm")` does NOT match "npx", "pnpm"
- [ ] `cargo check -p claudeless` passes

## Phase 2: Update Unit Tests

**File:** `crates/cli/src/permission/pattern.rs` (tests module)

### Task Agent Instructions

```
Update all pattern tests to use :* syntax:

BEFORE:
  ToolPattern::parse("Bash(npm *)").unwrap();
  assert!(pattern.matches("Bash", Some("npm test")));

AFTER:
  ToolPattern::parse("Bash(npm:*)").unwrap();
  assert!(pattern.matches("Bash", Some("npm test")));

Tests to update:
- test_glob_star → test_prefix_wildcard
- test_tool_pattern_with_glob → test_tool_pattern_with_prefix
- Any test using "npm *", "rm *", "sudo *", etc.

Add new tests:
- test_prefix_matches_exact_prefix ("npm" matches "npm")
- test_prefix_does_not_match_partial ("npm" does not match "npx")
- test_prefix_with_spaces ("rm -rf:*" matches "rm -rf /tmp")
```

### Validation

- [ ] All pattern.rs unit tests pass
- [ ] No tests use old `npm *` syntax

## Phase 3: Update Integration Tests

**File:** `crates/cli/tests/settings_permissions.rs`

### Task Agent Instructions

```
Update all integration tests to use :* syntax:

Search and replace patterns:
  "Bash(npm *)"     → "Bash(npm:*)"
  "Bash(rm *)"      → "Bash(rm:*)"
  "Bash(sudo *)"    → "Bash(sudo:*)"
  "Bash(npm run *)" → "Bash(npm run:*)"
  "Bash(git log *)" → "Bash(git log:*)"
  etc.

Update test assertions to match new behavior:
- Prefix matching is more permissive than glob
- "npm:*" matches "npm" (glob "npm *" did not)

Keep exact match tests unchanged:
  "Bash(npm test)" stays as-is
  "Bash(npm publish)" stays as-is
```

### Validation

- [ ] `cargo test -p claudeless --test settings_permissions` passes
- [ ] No tests use old space-wildcard syntax

## Phase 4: Update Documentation

**Files:**
- `crates/cli/docs/ACCURACY.md`
- `plans/epic-05d7-settings-file.md`

### Task Agent Instructions

```
Update documentation to reflect correct syntax:

1. ACCURACY.md - Update pattern syntax examples:
   - "Bash(npm *)" → "Bash(npm:*)"
   - Document :* as "prefix match" not "glob"

2. epic-05d7-settings-file.md - Update code examples:
   - All pattern examples should use :* syntax
   - Clarify that :* means "starts with"
```

### Validation

- [ ] No documentation references old `npm *` syntax
- [ ] Pattern syntax is clearly documented as prefix matching

## Phase 5: Integration Tests

**Goal:** Add integration tests that combine permissions with TUI/dotclaude output.

### Task Agent Instructions

```
Create integration tests that verify end-to-end permission behavior:

1. Create test scenarios in tests/fixtures/scenarios/:
   - permission-allow.toml: settings allow Bash(npm:*), tool call "npm test"
   - permission-deny.toml: settings deny Bash(rm:*), tool call "rm -rf /tmp"
   - permission-priority.toml: settings allow + deny same tool, verify deny wins

2. Create tests/permission_integration.rs:

   #[test]
   fn test_allowed_tool_auto_approves() {
       // Setup: settings with allow pattern
       // Action: run scenario with matching tool call
       // Assert: tool was auto-approved (no prompt)
       // Assert: TUI output shows execution without prompt
       // Assert: dotclaude session shows approved tool call
   }

   #[test]
   fn test_denied_tool_blocked() {
       // Setup: settings with deny pattern
       // Action: run scenario with matching tool call
       // Assert: tool was denied
       // Assert: TUI output shows denial message
       // Assert: dotclaude session shows denial
   }

   #[test]
   fn test_deny_beats_allow() {
       // Setup: settings with both allow and deny matching
       // Action: run scenario with matching tool call
       // Assert: tool was denied (deny wins)
   }

3. These tests document expected simulator behavior and prevent regressions
```

### Validation

- [ ] Integration tests cover allow, deny, priority scenarios
- [ ] Tests verify TUI output format (from TUI validation work)
- [ ] Tests verify dotclaude output format (from dotclaude validation work)
- [ ] All tests pass

## Architecture Notes

### CompiledPattern Enum (Updated)

```rust
pub enum CompiledPattern {
    /// Exact string match: "npm test" matches only "npm test"
    Exact(String),

    /// Prefix match: "npm" matches "npm", "npm test", "npm install"
    /// Used for :* patterns like Bash(npm:*)
    Prefix(String),

    /// Glob pattern: "*.md" matches "README.md", "CHANGELOG.md"
    /// Used for file patterns like Write(*.md)
    Glob(glob::Pattern),
}
```

### Pattern Parsing Logic

```
Input               → Variant
────────────────────────────────────
"Bash(npm:*)"       → Prefix("npm")
"Bash(rm -rf:*)"    → Prefix("rm -rf")
"Bash(npm test)"    → Exact("npm test")
"Write(*.md)"       → Glob("*.md")
"Read"              → (no argument pattern)
```

## Final Checklist

- [ ] Phase 1: Pattern parsing updated (`:*` prefix syntax)
- [ ] Phase 2: Unit tests updated (pattern matching, priority order)
- [ ] Phase 3: Integration tests updated (settings_permissions.rs)
- [ ] Phase 4: Documentation updated
- [ ] Phase 5: End-to-end integration tests (permissions + TUI + dotclaude)
- [ ] All tests pass
- [ ] `make check` passes
