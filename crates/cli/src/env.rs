// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Centralized environment variable access.
//!
//! All runtime environment variables used by claudeless are defined here.
//! Use these accessors instead of calling `std::env::var()` directly.

/// Generated env var name constants.
mod names {
    include!(concat!(env!("OUT_DIR"), "/env_names.rs"));
}

// Re-export name constants for callers that need the raw name string.
pub use names::*;

use std::path::PathBuf;

/// `CLAUDELESS_CONFIG_DIR` — Claudeless-specific config directory override.
pub fn config_dir() -> Option<PathBuf> {
    std::env::var(names::CLAUDELESS_CONFIG_DIR)
        .ok()
        .map(PathBuf::from)
}

/// `CLAUDELESS_STATE_DIR` — Legacy state directory (backwards compatibility).
pub fn state_dir() -> Option<PathBuf> {
    std::env::var(names::CLAUDELESS_STATE_DIR)
        .ok()
        .map(PathBuf::from)
}

/// `CLAUDE_CONFIG_DIR` — Standard Claude Code config directory.
pub fn claude_config_dir() -> Option<PathBuf> {
    std::env::var(names::CLAUDE_CONFIG_DIR)
        .ok()
        .map(PathBuf::from)
}

/// `CLAUDELESS_EXIT_HINT_TIMEOUT_MS` — Exit hint display duration.
pub fn exit_hint_timeout_ms() -> Option<u64> {
    var_u64(names::CLAUDELESS_EXIT_HINT_TIMEOUT_MS)
}

/// `CLAUDELESS_COMPACT_DELAY_MS` — Delay before compacting output.
pub fn compact_delay_ms() -> Option<u64> {
    var_u64(names::CLAUDELESS_COMPACT_DELAY_MS)
}

/// `CLAUDELESS_HOOK_TIMEOUT_MS` — Hook execution timeout.
pub fn hook_timeout_ms() -> Option<u64> {
    var_u64(names::CLAUDELESS_HOOK_TIMEOUT_MS)
}

/// `CLAUDELESS_MCP_TIMEOUT_MS` — MCP server timeout. Default 30000.
pub fn mcp_timeout_ms() -> Option<u64> {
    var_u64(names::CLAUDELESS_MCP_TIMEOUT_MS)
}

/// `CLAUDELESS_RESPONSE_DELAY_MS` — Response delay between messages.
pub fn response_delay_ms() -> Option<u64> {
    var_u64(names::CLAUDELESS_RESPONSE_DELAY_MS)
}

/// `CARGO_BIN_EXE_claudeless` — Path to compiled binary (set by cargo test).
pub fn cargo_bin_exe() -> Option<String> {
    std::env::var(names::CARGO_BIN_EXE_CLAUDELESS).ok()
}

/// `CARGO_TARGET_DIR` — Cargo build target directory.
pub fn cargo_target_dir() -> Option<PathBuf> {
    std::env::var(names::CARGO_TARGET_DIR)
        .ok()
        .map(PathBuf::from)
}

/// `HOME` — User's home directory.
pub fn home() -> Option<PathBuf> {
    std::env::var(names::HOME).ok().map(PathBuf::from)
}

fn var_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}

#[cfg(test)]
#[path = "env_tests.rs"]
mod tests;
