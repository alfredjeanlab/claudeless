// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_parse_basic_prompt() {
    let cli = Cli::try_parse_from(["claude", "hello world"]).unwrap();
    assert_eq!(cli.prompt, Some("hello world".to_string()));
    assert!(!cli.print);
}

#[test]
fn test_parse_print_mode() {
    let cli = Cli::try_parse_from(["claude", "-p", "test prompt"]).unwrap();
    assert!(cli.print);
    assert_eq!(cli.prompt, Some("test prompt".to_string()));
}

#[test]
fn test_parse_output_format_json() {
    let cli = Cli::try_parse_from(["claude", "--output-format", "json", "-p", "test"]).unwrap();
    assert!(matches!(cli.output_format, OutputFormat::Json));
}

#[test]
fn test_parse_output_format_stream_json() {
    let cli =
        Cli::try_parse_from(["claude", "--output-format", "stream-json", "-p", "test"]).unwrap();
    assert!(matches!(cli.output_format, OutputFormat::StreamJson));
}

#[test]
fn test_parse_model() {
    let cli =
        Cli::try_parse_from(["claude", "--model", "claude-opus-4-20250514", "-p", "test"]).unwrap();
    assert_eq!(cli.model, "claude-opus-4-20250514");
}

#[test]
fn test_parse_allowed_tools() {
    let cli = Cli::try_parse_from([
        "claude",
        "--allowedTools",
        "Bash",
        "--allowedTools",
        "Read",
        "-p",
        "test",
    ])
    .unwrap();
    assert_eq!(cli.allowed_tools, vec!["Bash", "Read"]);
}

#[test]
fn test_parse_simulator_flags() {
    let cli = Cli::try_parse_from([
        "claude",
        "--scenario",
        "/path/to/scenario.toml",
        "--capture",
        "/tmp/capture.jsonl",
        "--failure",
        "rate-limit",
        "--delay-ms",
        "100",
        "-p",
        "test",
    ])
    .unwrap();
    assert_eq!(cli.scenario, Some("/path/to/scenario.toml".to_string()));
    assert_eq!(cli.capture, Some("/tmp/capture.jsonl".to_string()));
    assert!(matches!(cli.failure, Some(FailureMode::RateLimit)));
    assert_eq!(cli.delay_ms, Some(100));
}

#[test]
fn test_parse_continue_conversation() {
    let cli = Cli::try_parse_from(["claude", "-c", "-p", "continue"]).unwrap();
    assert!(cli.continue_conversation);
}

#[test]
fn test_parse_resume() {
    let cli = Cli::try_parse_from(["claude", "-r", "session-123", "-p", "resume"]).unwrap();
    assert_eq!(cli.resume, Some("session-123".to_string()));
}

#[test]
fn test_default_model() {
    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    assert_eq!(cli.model, "claude-opus-4-5-20251101");
}

#[test]
fn test_parse_max_tokens() {
    let cli = Cli::try_parse_from(["claude", "--max-tokens", "4096", "-p", "test"]).unwrap();
    assert_eq!(cli.max_tokens, Some(4096));
}

#[test]
fn test_parse_system_prompt() {
    let cli = Cli::try_parse_from([
        "claude",
        "--system-prompt",
        "You are a helpful assistant",
        "-p",
        "test",
    ])
    .unwrap();
    assert_eq!(
        cli.system_prompt,
        Some("You are a helpful assistant".to_string())
    );
}

#[test]
fn test_parse_cwd() {
    let cli = Cli::try_parse_from(["claude", "--cwd", "/home/user/project", "-p", "test"]).unwrap();
    assert_eq!(cli.cwd, Some("/home/user/project".to_string()));
}

#[test]
fn test_parse_input_format() {
    let cli =
        Cli::try_parse_from(["claude", "--input-format", "stream-json", "-p", "test"]).unwrap();
    assert_eq!(cli.input_format, "stream-json");
}

#[test]
fn test_default_input_format() {
    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    assert_eq!(cli.input_format, "text");
}

#[test]
fn test_parse_session_id() {
    let cli = Cli::try_parse_from([
        "claude",
        "-p",
        "--session-id",
        "01234567-89ab-cdef-0123-456789abcdef",
        "test",
    ])
    .unwrap();
    assert_eq!(
        cli.session_id,
        Some("01234567-89ab-cdef-0123-456789abcdef".to_string())
    );
}

#[test]
fn test_parse_verbose() {
    let cli = Cli::try_parse_from(["claude", "--verbose", "-p", "test"]).unwrap();
    assert!(cli.verbose);
}

#[test]
fn test_parse_debug() {
    // Debug flag without value
    let cli = Cli::try_parse_from(["claude", "-d", "-p", "test"]).unwrap();
    assert!(cli.debug.is_some());
}

#[test]
fn test_parse_include_partial_messages() {
    let cli = Cli::try_parse_from(["claude", "--include-partial-messages", "-p", "test"]).unwrap();
    assert!(cli.include_partial_messages);
}

#[test]
fn test_parse_fallback_model() {
    let cli =
        Cli::try_parse_from(["claude", "--fallback-model", "claude-haiku", "-p", "test"]).unwrap();
    assert_eq!(cli.fallback_model, Some("claude-haiku".to_string()));
}

#[test]
fn test_parse_max_budget_usd() {
    let cli = Cli::try_parse_from(["claude", "--max-budget-usd", "10.50", "-p", "test"]).unwrap();
    assert_eq!(cli.max_budget_usd, Some(10.50));
}
