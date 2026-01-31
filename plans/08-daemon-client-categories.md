# Daemon Client Command Categories

## Problem

The `DaemonClient::connect_or_start()` method auto-restarts the daemon on version mismatch. This causes issues:

1. **Infinite loop risk**: `oj emit agent:done` retry calls `connect_or_start()`, which may restart daemon, which may fail again, triggering another restart
2. **Pointless restarts**: Query commands like `oj status` shouldn't restart—if daemon is wrong version, there's nothing useful to query
3. **Lost context**: Signal commands from agents shouldn't restart—the new daemon has no knowledge of the agent

## Solution

Categorize commands and provide appropriate client constructors:

| Category | Auto-restart | Max restarts | Methods |
|----------|--------------|--------------|---------|
| **Action** | Yes | 1 | `for_action()` |
| **Query** | No | 0 | `for_query()` |
| **Signal** | No | 0 | `for_signal()` |

### Command Classification

**Action** (mutates state, user-initiated):
- `oj run <command>`
- `oj pipeline resume`
- `oj pipeline cancel`
- `oj workspace drop`
- `oj workspace prune`
- `oj daemon stop`

**Query** (reads state, user-initiated):
- `oj status`
- `oj pipeline list`
- `oj pipeline get`
- `oj pipeline peek`
- `oj pipeline logs`
- `oj session list`
- `oj workspace list`

**Signal** (operational, agent-initiated):
- `oj emit agent:done`

## Implementation

### 1. Add client constructors in `crates/cli/src/client.rs`

```rust
impl DaemonClient {
    /// For action commands - auto-start with version check, max 1 restart
    pub fn for_action() -> Result<Self, ClientError> {
        Self::connect_or_start_once()
    }

    /// For query commands - connect only, no restart
    pub fn for_query() -> Result<Self, ClientError> {
        Self::connect()
    }

    /// For signal commands - connect only, no restart
    /// Semantic alias for for_query() to document intent
    pub fn for_signal() -> Result<Self, ClientError> {
        Self::connect()
    }

    /// Internal: connect_or_start with restart limit
    fn connect_or_start_once() -> Result<Self, ClientError> {
        static RESTARTED: AtomicBool = AtomicBool::new(false);

        // If we already restarted this process, don't do it again
        if RESTARTED.load(Ordering::SeqCst) {
            return Self::connect();
        }

        // Check version and restart if needed
        let daemon_dir = daemon_dir()?;
        let version_path = daemon_dir.join("daemon.version");
        if let Ok(daemon_version) = std::fs::read_to_string(&version_path) {
            let cli_version = concat!(env!("CARGO_PKG_VERSION"), "+", env!("BUILD_GIT_HASH"));
            if daemon_version.trim() != cli_version {
                // Mark that we're restarting (before actually doing it)
                RESTARTED.store(true, Ordering::SeqCst);
                eprintln!(
                    "warn: daemon version {} does not match cli version {}, restarting daemon",
                    daemon_version.trim(),
                    cli_version
                );
                stop_daemon_sync();
            }
        }

        // Now connect or start
        match Self::connect() {
            Ok(client) => {
                if probe_socket(&client.socket_path) {
                    Ok(client)
                } else {
                    cleanup_stale_socket()?;
                    let child = start_daemon_background()?;
                    Self::connect_with_retry(timeout_connect(), child)
                }
            }
            Err(ClientError::DaemonNotRunning) => {
                let child = start_daemon_background()?;
                Self::connect_with_retry(timeout_connect(), child)
            }
            Err(e) => Err(wrap_with_startup_error(e)),
        }
    }
}
```

### 2. Update emit_event retry in `crates/cli/src/client.rs`

```rust
pub async fn emit_event(&self, event: oj_core::Event) -> Result<(), ClientError> {
    match self.send_simple(Request::Event { event: event.clone() }).await {
        Ok(()) => Ok(()),
        Err(ClientError::Io(_)) | Err(ClientError::Protocol(_)) => {
            // Retry with signal semantics - no restart
            let new_client = DaemonClient::for_signal()?;
            new_client.send_simple(Request::Event { event }).await
        }
        Err(e) => Err(e),
    }
}
```

### 3. Update command handlers

In `crates/cli/src/main.rs` or individual command files:

```rust
// Action commands
Commands::Run { .. } => {
    let client = DaemonClient::for_action()?;
    // ...
}
Commands::Pipeline { cmd: PipelineCmd::Resume { .. } } => {
    let client = DaemonClient::for_action()?;
    // ...
}

// Query commands
Commands::Status => {
    let client = DaemonClient::for_query()?;
    // ...
}
Commands::Pipeline { cmd: PipelineCmd::List } => {
    let client = DaemonClient::for_query()?;
    // ...
}

// Signal commands
Commands::Emit { .. } => {
    let client = DaemonClient::for_signal()?;
    // ...
}
```

## Files to Modify

1. `crates/cli/src/client.rs` - Add constructors, update emit_event
2. `crates/cli/src/main.rs` - Update command dispatch to use appropriate constructor
3. `crates/cli/src/commands/emit.rs` - Use for_signal()

## Testing

1. Version mismatch with action command → restarts once, then connects
2. Version mismatch with query command → fails with "daemon not running" or connects to existing
3. Signal from agent after daemon restart → fails cleanly (no loop)
4. Multiple action commands in same process → only first triggers restart

## Acceptance Criteria

- [ ] `for_action()`, `for_query()`, `for_signal()` methods exist
- [ ] Action commands use `for_action()`
- [ ] Query commands use `for_query()`
- [ ] `oj emit` uses `for_signal()`
- [ ] `emit_event()` retry uses `for_signal()`
- [ ] Max 1 restart per process enforced via AtomicBool
- [ ] No infinite restart loop possible
