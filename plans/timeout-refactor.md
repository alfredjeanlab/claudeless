# Plan: Consolidate Timeout Configuration

## Goal
Remove `--delay-ms` CLI argument and add unified timeout configuration via:
1. Scenario `[timeouts]` section
2. Environment variables

## Timeouts to Configure

| Timeout | Current Location | Default | New Env Var |
|---------|-----------------|---------|-------------|
| Exit hint | `EXIT_HINT_TIMEOUT_MS` constant | 2000ms | `CLAUDELESS_EXIT_HINT_TIMEOUT_MS` |
| Compact delay | `compact_delay_ms` field | 20ms | `CLAUDELESS_COMPACT_DELAY_MS` |
| Hook timeout | `HookConfig::new()` | 5000ms | `CLAUDELESS_HOOK_TIMEOUT_MS` |
| MCP server timeout | `default_timeout()` | 30000ms | `CLAUDELESS_MCP_TIMEOUT_MS` |
| Response delay | `--delay-ms` CLI arg | 0ms | `CLAUDELESS_RESPONSE_DELAY_MS` |

---

## Implementation Steps

### Step 1: Add `TimeoutConfig` and `ResolvedTimeouts` (`config.rs`)

Add new types at end of file:

```rust
/// Timeout configuration (scenario [timeouts] section)
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimeoutConfig {
    pub exit_hint_ms: Option<u64>,
    pub compact_delay_ms: Option<u64>,
    pub hook_timeout_ms: Option<u64>,
    pub mcp_timeout_ms: Option<u64>,
    pub response_delay_ms: Option<u64>,
}

/// Resolved timeouts with defaults applied
#[derive(Clone, Debug)]
pub struct ResolvedTimeouts {
    pub exit_hint_ms: u64,
    pub compact_delay_ms: u64,
    pub hook_timeout_ms: u64,
    pub mcp_timeout_ms: u64,
    pub response_delay_ms: u64,
}

impl ResolvedTimeouts {
    pub const DEFAULT_EXIT_HINT_MS: u64 = 2000;
    pub const DEFAULT_COMPACT_DELAY_MS: u64 = 20;
    pub const DEFAULT_HOOK_TIMEOUT_MS: u64 = 5000;
    pub const DEFAULT_MCP_TIMEOUT_MS: u64 = 30000;
    pub const DEFAULT_RESPONSE_DELAY_MS: u64 = 0;

    /// Resolve from optional config with precedence: scenario > env > default
    pub fn resolve(config: Option<&TimeoutConfig>) -> Self {
        let cfg = config.cloned().unwrap_or_default();
        Self {
            exit_hint_ms: cfg.exit_hint_ms
                .or_else(|| Self::env_u64("CLAUDELESS_EXIT_HINT_TIMEOUT_MS"))
                .unwrap_or(Self::DEFAULT_EXIT_HINT_MS),
            compact_delay_ms: cfg.compact_delay_ms
                .or_else(|| Self::env_u64("CLAUDELESS_COMPACT_DELAY_MS"))
                .unwrap_or(Self::DEFAULT_COMPACT_DELAY_MS),
            hook_timeout_ms: cfg.hook_timeout_ms
                .or_else(|| Self::env_u64("CLAUDELESS_HOOK_TIMEOUT_MS"))
                .unwrap_or(Self::DEFAULT_HOOK_TIMEOUT_MS),
            mcp_timeout_ms: cfg.mcp_timeout_ms
                .or_else(|| Self::env_u64("CLAUDELESS_MCP_TIMEOUT_MS"))
                .unwrap_or(Self::DEFAULT_MCP_TIMEOUT_MS),
            response_delay_ms: cfg.response_delay_ms
                .or_else(|| Self::env_u64("CLAUDELESS_RESPONSE_DELAY_MS"))
                .unwrap_or(Self::DEFAULT_RESPONSE_DELAY_MS),
        }
    }

    fn env_u64(name: &str) -> Option<u64> {
        std::env::var(name).ok().and_then(|v| v.parse().ok())
    }
}
```

### Step 2: Update `ScenarioConfig` (`config.rs`)

- Remove: `pub compact_delay_ms: Option<u64>` field
- Add: `pub timeouts: Option<TimeoutConfig>` field

### Step 3: Remove CLI argument (`cli.rs`)

Remove these lines:
```rust
/// Response delay in milliseconds
#[arg(long, env = "CLAUDELESS_DELAY_MS")]
pub delay_ms: Option<u64>,
```

### Step 4: Update `TuiConfig` (`tui/app/types.rs`)

- Remove `EXIT_HINT_TIMEOUT_MS` constant
- Remove `compact_delay_ms` field
- Add `timeouts: ResolvedTimeouts` field
- Update `TuiConfig::from_scenario()` to resolve timeouts

### Step 5: Update input handling (`tui/app/input.rs`)

Replace all uses of `EXIT_HINT_TIMEOUT_MS` with `inner.config.timeouts.exit_hint_ms`

### Step 6: Update state checks (`tui/app/state/mod.rs`)

- `check_exit_hint_timeout()`: Use `inner.config.timeouts.exit_hint_ms`
- `check_compacting()`: Use `inner.config.timeouts.compact_delay_ms`

### Step 7: Update hook executor (`hooks/executor.rs`)

Change `HookConfig::new()` default from hardcoded 5000 to accept parameter:
```rust
pub fn new(script_path: impl Into<PathBuf>, default_timeout_ms: u64) -> Self
```

### Step 8: Update hook registry (`hooks/registry.rs`)

Pass resolved timeout when creating `HookConfig`

### Step 9: Update MCP config (`mcp/config.rs`)

Change `default_timeout()` to read from env var with fallback:
```rust
fn default_timeout() -> u64 {
    std::env::var("CLAUDELESS_MCP_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30000)
}
```

### Step 10: Update main.rs

Replace:
```rust
if let Some(delay_ms) = cli.delay_ms {
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
}
```

With:
```rust
let timeouts = ResolvedTimeouts::resolve(scenario.as_ref().and_then(|s| s.config().timeouts.as_ref()));
if timeouts.response_delay_ms > 0 {
    tokio::time::sleep(Duration::from_millis(timeouts.response_delay_ms)).await;
}
```

### Step 11: Update api.rs

- Remove `delay_ms` field from `SimulatorBuilder`
- Remove `delay_ms()` builder method
- Remove `delay_ms` from `SimulatorHandle::InProcess`
- Remove `delay_ms` from `BinarySimulatorHandle`
- Remove env var setting in `BinarySimulatorHandle::env_vars()`

### Step 12: Migrate scenario files

Update all `scenarios/*.toml` files:
- Remove top-level `compact_delay_ms`
- Add `[timeouts]` section where needed

### Step 13: Update documentation

**docs/USAGE.md**: Remove `--delay-ms` flag documentation

**docs/SCENARIOS.md**: Add `[timeouts]` section:
```markdown
### Timeouts

Configure various timeout values:

```toml
[timeouts]
exit_hint_ms = 2000      # "Press Ctrl-C again" hint duration
compact_delay_ms = 20    # /compact spinner delay
hook_timeout_ms = 5000   # Hook script execution limit
mcp_timeout_ms = 30000   # MCP server response timeout
response_delay_ms = 100  # Delay before sending response
```

All timeouts can also be set via environment variables:
- `CLAUDELESS_EXIT_HINT_TIMEOUT_MS`
- `CLAUDELESS_COMPACT_DELAY_MS`
- `CLAUDELESS_HOOK_TIMEOUT_MS`
- `CLAUDELESS_MCP_TIMEOUT_MS`
- `CLAUDELESS_RESPONSE_DELAY_MS`

Precedence: scenario config > environment variable > default
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `config.rs` | Add `TimeoutConfig`, `ResolvedTimeouts`; update `ScenarioConfig` |
| `cli.rs` | Remove `delay_ms` arg |
| `tui/app/types.rs` | Remove constant; add `timeouts` to `TuiConfig` |
| `tui/app/input.rs` | Use config timeout |
| `tui/app/state/mod.rs` | Use config timeouts |
| `hooks/executor.rs` | Parameterize timeout |
| `hooks/registry.rs` | Pass resolved timeout |
| `mcp/config.rs` | Read env var in default |
| `main.rs` | Use resolved response delay |
| `api.rs` | Remove delay_ms builder/fields |
| `scenarios/*.toml` | Migrate to `[timeouts]` |
| `docs/USAGE.md` | Remove --delay-ms |
| `docs/SCENARIOS.md` | Document [timeouts] |

---

## Verification

1. `cargo build --all` - Compiles
2. `cargo test --all` - Tests pass
3. Test env var: `CLAUDELESS_EXIT_HINT_TIMEOUT_MS=500 cargo run`
4. Test scenario: Create file with `[timeouts]` section
5. `make check` - Full suite passes
