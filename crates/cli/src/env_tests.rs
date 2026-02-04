// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use serial_test::serial;

#[test]
#[serial]
fn config_dir_returns_none_when_unset() {
    std::env::remove_var(CLAUDELESS_CONFIG_DIR);
    assert_eq!(config_dir(), None);
}

#[test]
#[serial]
fn config_dir_returns_path_when_set() {
    std::env::set_var(CLAUDELESS_CONFIG_DIR, "/tmp/test-config");
    let result = config_dir();
    std::env::remove_var(CLAUDELESS_CONFIG_DIR);
    assert_eq!(result, Some(std::path::PathBuf::from("/tmp/test-config")));
}

#[test]
#[serial]
fn state_dir_returns_none_when_unset() {
    std::env::remove_var(CLAUDELESS_STATE_DIR);
    assert_eq!(state_dir(), None);
}

#[test]
#[serial]
fn state_dir_returns_path_when_set() {
    std::env::set_var(CLAUDELESS_STATE_DIR, "/tmp/test-state");
    let result = state_dir();
    std::env::remove_var(CLAUDELESS_STATE_DIR);
    assert_eq!(result, Some(std::path::PathBuf::from("/tmp/test-state")));
}

#[test]
#[serial]
fn claude_config_dir_returns_none_when_unset() {
    std::env::remove_var(CLAUDE_CONFIG_DIR);
    assert_eq!(claude_config_dir(), None);
}

#[test]
#[serial]
fn mcp_timeout_returns_none_when_unset() {
    std::env::remove_var(CLAUDELESS_MCP_TIMEOUT_MS);
    assert_eq!(mcp_timeout_ms(), None);
}

#[test]
#[serial]
fn mcp_timeout_parses_valid_u64() {
    std::env::set_var(CLAUDELESS_MCP_TIMEOUT_MS, "5000");
    let result = mcp_timeout_ms();
    std::env::remove_var(CLAUDELESS_MCP_TIMEOUT_MS);
    assert_eq!(result, Some(5000));
}

#[test]
#[serial]
fn mcp_timeout_returns_none_for_non_numeric() {
    std::env::set_var(CLAUDELESS_MCP_TIMEOUT_MS, "not-a-number");
    let result = mcp_timeout_ms();
    std::env::remove_var(CLAUDELESS_MCP_TIMEOUT_MS);
    assert_eq!(result, None);
}

#[test]
#[serial]
fn exit_hint_timeout_parses_valid_u64() {
    std::env::set_var(CLAUDELESS_EXIT_HINT_TIMEOUT_MS, "1000");
    let result = exit_hint_timeout_ms();
    std::env::remove_var(CLAUDELESS_EXIT_HINT_TIMEOUT_MS);
    assert_eq!(result, Some(1000));
}

#[test]
fn home_returns_some_value() {
    // HOME is always set in dev/CI environments
    assert!(home().is_some());
}

#[test]
fn cargo_bin_exe_returns_option() {
    // Just verify it doesn't panic; value depends on test context
    let _ = cargo_bin_exe();
}

#[test]
fn cargo_target_dir_returns_option() {
    let _ = cargo_target_dir();
}

#[test]
#[serial]
fn compact_delay_ms_returns_none_when_unset() {
    std::env::remove_var(CLAUDELESS_COMPACT_DELAY_MS);
    assert_eq!(compact_delay_ms(), None);
}

#[test]
#[serial]
fn hook_timeout_ms_returns_none_when_unset() {
    std::env::remove_var(CLAUDELESS_HOOK_TIMEOUT_MS);
    assert_eq!(hook_timeout_ms(), None);
}

#[test]
#[serial]
fn response_delay_ms_returns_none_when_unset() {
    std::env::remove_var(CLAUDELESS_RESPONSE_DELAY_MS);
    assert_eq!(response_delay_ms(), None);
}

#[test]
#[serial]
fn response_delay_ms_parses_valid_u64() {
    std::env::set_var(CLAUDELESS_RESPONSE_DELAY_MS, "100");
    let result = response_delay_ms();
    std::env::remove_var(CLAUDELESS_RESPONSE_DELAY_MS);
    assert_eq!(result, Some(100));
}
