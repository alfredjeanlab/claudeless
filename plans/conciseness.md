# Conciseness Plan: Remove Ceremony, Keep Boundaries

## Goal
Reduce LOC by 250+ lines while maintaining (and improving) the clean module separation achieved in plans 01, 04, and 07. Focus on eliminating wrapper structs, duplicate validation boilerplate, and excessive documentation while preserving clear architectural boundaries.

## Principles
- **Separate files**: Keep the file organization (paths.rs, persistence.rs, index.rs, etc.)
- **Clear boundaries**: Maintain module responsibilities and public APIs
- **Zero ceremony**: Remove wrappers that add no value
- **Inline trivia**: Replace one-line delegation methods with direct access
- **Documentation**: Keep essential docs, remove redundant examples

---

## Part 1: Eliminate StatePaths Wrapper (~100 LOC reduction)

### Problem
`state/paths.rs` (184 lines) wraps simple path concatenation in a struct with 6 trivial methods. Each method is just `self.root.join("subdir")`. StateDirectory then delegates to all these methods (8 pass-through methods).

### Solution
**Replace StatePaths struct with free functions:**

```rust
// state/paths.rs - AFTER (55 lines, down from 184)
pub fn todos_dir(root: &Path) -> PathBuf { root.join("todos") }
pub fn projects_dir(root: &Path) -> PathBuf { root.join("projects") }
pub fn plans_dir(root: &Path) -> PathBuf { root.join("plans") }
pub fn sessions_dir(root: &Path) -> PathBuf { root.join("sessions") }
pub fn settings_path(root: &Path) -> PathBuf { root.join("settings.json") }

pub fn project_dir(root: &Path, project_path: &Path) -> PathBuf {
    let dir_name = project_dir_name(project_path);
    projects_dir(root).join(&dir_name)
}

pub fn session_path(root: &Path, session_id: &str) -> PathBuf {
    sessions_dir(root).join(format!("{}.json", session_id))
}

pub fn todo_path(root: &Path, context: &str) -> PathBuf {
    todos_dir(root).join(format!("{}.json", context))
}

pub fn normalize_project_path(path: &Path) -> String { /* keep */ }
pub fn project_dir_name(path: &Path) -> String { /* keep */ }
```

**Update StateDirectory to use free functions:**

```rust
// state/directory.rs
pub struct StateDirectory {
    root: PathBuf,  // Remove paths: StatePaths field
    initialized: bool,
}

impl StateDirectory {
    pub fn root(&self) -> &Path { &self.root }

    pub fn todos_dir(&self) -> PathBuf {
        paths::todos_dir(&self.root)
    }

    pub fn projects_dir(&self) -> PathBuf {
        paths::projects_dir(&self.root)
    }

    // ... etc for other paths (still delegates, but to functions not methods)
}
```

**Impact:**
- paths.rs: 184 → 55 lines (-129 lines)
- directory.rs: Remove `paths: StatePaths` field and `.paths` indirection
- Same public API for StateDirectory users
- StatePaths struct eliminated from exports

---

## Part 2: Inline MessageIds and WriteContext (~60 LOC reduction)

### Problem
- **MessageIds** (39 lines): Struct with no methods, just UUID generation, never exported
- **WriteContext** (18 lines): Tuple-like struct with 6 fields, never exported
- Both are immediately destructured at every use site

### Solution
**Replace MessageIds with inline generation:**

```rust
// state/mod.rs - REMOVE MessageIds struct entirely

// In StateWriter methods, generate UUIDs inline:
pub fn record_turn(&mut self, prompt: &str, response: &str) -> std::io::Result<()> {
    let ctx = self.write_context()?;
    let user_uuid = Uuid::new_v4().to_string();
    let assistant_uuid = Uuid::new_v4().to_string();
    let request_id = format!("req_{}", Uuid::new_v4().simple());
    let message_id = format!("msg_{}", Uuid::new_v4().simple());

    let params = TurnParams {
        session_id: &ctx.session_id,
        user_uuid: &user_uuid,
        assistant_uuid: &assistant_uuid,
        request_id: &request_id,
        // ...
    };
    // ...
}
```

**Replace WriteContext with inline struct construction:**

```rust
// state/mod.rs - REMOVE WriteContext struct

// In write_context(), return tuple or construct inline:
fn write_context(&self) -> std::io::Result<(PathBuf, String, DateTime<Utc>)> {
    let project_dir = self.project_dir();
    std::fs::create_dir_all(&project_dir)?;

    Ok((
        self.session_jsonl_path(),
        get_git_branch(),
        Utc::now(),
    ))
}

// Or inline at call sites since it's only used 6 times
```

**Impact:**
- Remove 57 lines (39 + 18)
- Clearer code flow - no struct creation/destruction ceremony
- Same functionality, less abstraction

---

## Part 3: Consolidate Config Validation (~25 LOC reduction)

### Problem
Three near-identical `validate()` methods with repeated boilerplate:
- `IdentityConfig::validate()` (10 LOC)
- `EnvironmentConfig::validate()` (25 LOC)
- `TimingConfig::validate()` (14 LOC)

Called from 3 separate lines in `scenario.rs` with `.map_err()` wrappers.

### Solution A: Add ScenarioConfig::validate() wrapper (Minimal)

```rust
// config.rs
impl ScenarioConfig {
    /// Validate all sub-configurations.
    pub fn validate(&self) -> Result<(), String> {
        self.identity.validate()?;
        self.environment.validate()?;
        self.timing.validate()?;
        Ok(())
    }
}
```

```rust
// scenario.rs - Replace 3 validate calls with 1
config.validate().map_err(ScenarioError::Validation)?;
```

**Impact:** -10 LOC in scenario.rs, +6 LOC in config.rs = net -4 LOC

### Solution B: Inline validators (Aggressive)

Move validation back to `Scenario::from_config()` but keep field organization:

```rust
// scenario.rs
pub fn from_config(config: ScenarioConfig) -> Result<Self, ScenarioError> {
    // Validate identity
    if let Some(ref id) = config.identity.session_id {
        if uuid::Uuid::parse_str(id).is_err() {
            return Err(ScenarioError::Validation(
                format!("Invalid session_id '{}': must be a valid UUID", id)
            ));
        }
    }

    // Validate environment
    if let Some(ref mode) = config.environment.permission_mode {
        if !EnvironmentConfig::VALID_PERMISSION_MODES.contains(&mode.as_str()) {
            return Err(ScenarioError::Validation(
                format!("Invalid permission_mode '{}': must be one of {:?}",
                    mode, EnvironmentConfig::VALID_PERMISSION_MODES)
            ));
        }
    }

    // Validate timing
    if let Some(ref ts) = config.timing.launch_timestamp {
        if chrono::DateTime::parse_from_rfc3339(ts).is_err() {
            return Err(ScenarioError::Validation(
                format!("Invalid launch_timestamp '{}': must be ISO 8601 format", ts)
            ));
        }
    }

    // ... continue with compilation
}
```

**Impact:**
- Remove 3 validate() methods from config.rs (-49 LOC)
- Add inline validation to scenario.rs (+30 LOC)
- Net: -19 LOC
- Tradeoff: Validation no longer near the data

**Recommendation:** Use Solution A (minimal). Keeps validation encapsulation.

---

## Part 4: Trim Documentation Overhead (~30 LOC reduction)

### Current State
paths.rs has 54 comment lines (30% of file) with verbose examples for trivial functions.

### Solution
**Reduce doc comments to essentials:**

```rust
// Before (5 lines of docs for a path.join):
/// Get the todos directory path.
pub fn todos_dir(root: &Path) -> PathBuf {
    root.join("todos")
}

// After (0 lines - self-documenting):
pub fn todos_dir(root: &Path) -> PathBuf {
    root.join("todos")
}
```

**Keep docs only for:**
- Non-obvious behavior (normalize_project_path)
- Public API types (StateDirectory, StateWriter)
- Complex logic (validation rules)

**Impact:** -30 lines of redundant documentation

---

## Part 5: StateWriter Method Cleanup (~15 LOC reduction)

### Problem
6 trivial delegation methods that could be direct field access or removed:

```rust
pub fn session_id(&self) -> &str { &self.session_id }  // Just expose field
pub fn state_dir(&self) -> &StateDirectory { &self.dir }  // Rarely used
pub fn project_dir(&self) -> PathBuf { self.dir.project_dir(&self.project_path) }
```

### Solution
**Make appropriate fields public:**

```rust
pub struct StateWriter {
    dir: StateDirectory,
    pub session_id: String,  // Make public
    project_path: PathBuf,
    // ...
}

// Remove session_id() method - users access field directly
```

**Inline rarely-used methods at call sites:**

```rust
// Instead of: self.state_writer.project_dir()
// Use: self.state_writer.dir.project_dir(&self.state_writer.project_path)
// Or: Keep method if used frequently (check usage)
```

**Impact:** -15 LOC of trivial accessors

---

## Part 6: CLI Structure Review (Already Good!)

### Current State (post-refactor)
CLI was already reorganized into focused sub-structs using `#[command(flatten)]`:
- OutputOptions
- SessionOptions
- PermissionOptions
- McpOptions
- SimulatorOptions

### Analysis
✅ **Good organization** - Related flags grouped semantically
✅ **Validation on sub-structs** - SessionOptions has validate methods
✅ **Default impls where needed** - No ceremony, just clap requirements
✅ **No bloat** - Each group is 3-5 fields, appropriate size

### Verdict
**No changes needed.** This refactoring already follows the conciseness principle - clear boundaries without ceremony.

---

## Summary of Changes

| File | Before | After | Reduction | Change |
|------|--------|-------|-----------|--------|
| state/paths.rs | 184 | 55 | -129 | Remove StatePaths struct, use free functions |
| state/mod.rs | 510 | 440 | -70 | Remove MessageIds, WriteContext, trim StateWriter |
| state/directory.rs | 294 | 270 | -24 | Use paths:: functions directly |
| config.rs | 439 | 445 | +6 | Add ScenarioConfig::validate() wrapper |
| scenario.rs | 310 | 300 | -10 | Single validate() call |
| **Total** | **1737** | **1510** | **-227** | **13% reduction** |

---

## Implementation Order

1. **paths.rs refactor** (biggest win, low risk)
   - Convert StatePaths to free functions
   - Update directory.rs to call functions
   - Update tests

2. **Remove MessageIds/WriteContext** (medium win, medium risk)
   - Inline UUID generation
   - Inline context construction
   - Verify no external usage

3. **Add ScenarioConfig::validate()** (small win, zero risk)
   - Single wrapper method
   - Update scenario.rs call site

4. **Trim documentation** (small win, zero risk)
   - Remove redundant examples
   - Keep essential docs

5. **StateWriter cleanup** (small win, low risk)
   - Make session_id public
   - Remove trivial accessors

---

## Verification

After each change:
1. **Run tests**: `cargo test --all`
2. **Check compilation**: `cargo check --all`
3. **Verify public API**: Ensure no breaking changes to lib.rs exports
4. **Run integration tests**: Scenario loading, state persistence

Final verification:
- All tests pass
- No new clippy warnings
- LOC reduction confirmed with `cloc`
- Public API unchanged (check dependent code)

---

## Non-Goals

This plan does NOT:
- Change file organization (keep paths.rs, persistence.rs, etc.)
- Merge modules (keep separation)
- Remove sub-config structs (keep IdentityConfig, etc.)
- Change TUI structure (already done in plan 04)

We're removing **ceremony**, not **structure**.
