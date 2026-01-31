// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for Runtime.
//!
//! These tests construct Runtime directly without going through main(),
//! validating the orchestration layer in isolation.

mod common;

use clap::Parser;
use claudeless::cli::Cli;
use claudeless::runtime::RuntimeBuilder;
use common::write_scenario;

/// Test that RuntimeBuilder can be constructed with valid CLI args.
#[test]
fn test_runtime_builder_construction() {
    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let builder = RuntimeBuilder::new(cli);
    assert!(
        builder.is_ok(),
        "RuntimeBuilder should construct with valid CLI"
    );
}

/// Test that RuntimeBuilder validates CLI args.
#[test]
fn test_runtime_builder_validation() {
    // --no-session-persistence without -p should fail validation
    let cli = Cli::try_parse_from(["claude", "--no-session-persistence"]).unwrap();
    let builder = RuntimeBuilder::new(cli);
    assert!(
        builder.is_err(),
        "RuntimeBuilder should fail with invalid CLI args"
    );
}

/// Test that Runtime can be built with a scenario.
#[tokio::test]
async fn test_runtime_with_scenario() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello from test!"
        "#,
    );

    let cli = Cli::try_parse_from([
        "claude",
        "-p",
        "test prompt",
        "--scenario",
        scenario.path().to_str().unwrap(),
        "--no-session-persistence",
    ])
    .unwrap();

    let builder = RuntimeBuilder::new(cli).unwrap();
    let runtime = builder.build_from_cli().await;
    assert!(runtime.is_ok(), "Runtime should build successfully");

    let runtime = runtime.unwrap();
    assert!(runtime.scenario.is_some(), "Runtime should have a scenario");
    assert!(!runtime.should_use_tui(), "Print mode should not use TUI");
}

/// Test that Runtime respects session ID from CLI.
#[tokio::test]
async fn test_runtime_session_id() {
    let session_id = "12345678-1234-1234-1234-123456789abc";

    let cli = Cli::try_parse_from([
        "claude",
        "-p",
        "test",
        "--session-id",
        session_id,
        "--no-session-persistence",
    ])
    .unwrap();

    let builder = RuntimeBuilder::new(cli).unwrap();
    let runtime = builder.build_from_cli().await.unwrap();

    assert_eq!(runtime.session_id(), session_id);
}

/// Test that Runtime detects TUI mode correctly.
#[tokio::test]
async fn test_runtime_tui_detection() {
    // With -p flag, should not use TUI
    let cli = Cli::try_parse_from(["claude", "-p", "test", "--no-session-persistence"]).unwrap();
    let builder = RuntimeBuilder::new(cli).unwrap();
    let runtime = builder.build_from_cli().await.unwrap();
    assert!(!runtime.should_use_tui());

    // Without -p and in non-terminal, should still not use TUI
    // (this test runs in a non-terminal environment)
}

/// Test RuntimeBuilder with_settings loads settings.
#[test]
fn test_runtime_builder_with_settings() {
    let cli = Cli::try_parse_from(["claude", "-p", "test", "--no-session-persistence"]).unwrap();
    let _builder = RuntimeBuilder::new(cli).unwrap().with_settings();
    // with_settings should not fail - if it did, we'd have panicked above
}

/// Test that permission bypass validation works.
#[test]
fn test_runtime_builder_permission_bypass() {
    // --dangerously-skip-permissions without --allow-dangerously-skip-permissions should fail
    let cli =
        Cli::try_parse_from(["claude", "-p", "test", "--dangerously-skip-permissions"]).unwrap();

    let builder = RuntimeBuilder::new(cli);
    assert!(
        builder.is_err(),
        "Should fail without allow-dangerously-skip-permissions"
    );

    // With both flags, should succeed
    let cli = Cli::try_parse_from([
        "claude",
        "-p",
        "test",
        "--allow-dangerously-skip-permissions",
        "--dangerously-skip-permissions",
        "--no-session-persistence",
    ])
    .unwrap();

    let builder = RuntimeBuilder::new(cli);
    assert!(builder.is_ok(), "Should succeed with both flags");
}
