// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_builder_respond_to() {
    let sim = SimulatorBuilder::new()
        .respond_to("hello", "Hello back!")
        .build_in_process()
        .unwrap();

    let response = sim.execute("hello world");
    assert_eq!(response, "Hello back!");
}

#[test]
fn test_builder_respond_to_exact() {
    let sim = SimulatorBuilder::new()
        .respond_to_exact("hello", "Exact match!")
        .respond_to("hello", "Contains match!")
        .build_in_process()
        .unwrap();

    // Exact match
    let response = sim.execute("hello");
    assert_eq!(response, "Exact match!");

    // Contains match (exact doesn't match)
    let response = sim.execute("hello world");
    assert_eq!(response, "Contains match!");
}

#[test]
fn test_builder_respond_to_regex() {
    let sim = SimulatorBuilder::new()
        .respond_to_regex(r"^test\d+$", "Matched number pattern!")
        .build_in_process()
        .unwrap();

    assert_eq!(sim.execute("test123"), "Matched number pattern!");
    assert_eq!(sim.execute("test"), ""); // No match
}

#[test]
fn test_builder_default_response() {
    let sim = SimulatorBuilder::new()
        .respond_to("specific", "Specific response")
        .default_response("Default fallback")
        .build_in_process()
        .unwrap();

    assert_eq!(sim.execute("specific"), "Specific response");
    assert_eq!(sim.execute("anything else"), "Default fallback");
}

#[test]
fn test_assertions() {
    let sim = SimulatorBuilder::new()
        .respond_to("test", "response")
        .build_in_process()
        .unwrap();

    sim.execute("test prompt");

    sim.assert_received("test");
    sim.assert_not_received("other");
    sim.assert_count(1);
    sim.assert_last_response_contains("response");
}

#[test]
fn test_reset() {
    let sim = SimulatorBuilder::new()
        .respond_to("test", "response")
        .build_in_process()
        .unwrap();

    sim.execute("test");
    sim.assert_count(1);

    sim.reset();
    sim.assert_count(0);
}

#[test]
fn test_capture_recorded() {
    let sim = SimulatorBuilder::new()
        .respond_to("hello", "Hi!")
        .build_in_process()
        .unwrap();

    sim.execute("hello world");

    let interactions = sim.capture().interactions();
    assert_eq!(interactions.len(), 1);
    assert_eq!(interactions[0].args.prompt, Some("hello world".to_string()));
}

#[test]
fn test_execute_with_args() {
    let sim = SimulatorBuilder::new()
        .respond_to("test", "ok")
        .build_in_process()
        .unwrap();

    sim.execute_with_args("test", Some("claude-opus"));

    let interactions = sim.capture().interactions();
    assert_eq!(interactions[0].args.model, "claude-opus");
}

#[test]
fn test_binary_handle_env_vars() {
    let handle = SimulatorBuilder::new()
        .respond_to("test", "ok")
        .delay_ms(100)
        .build_binary()
        .unwrap();

    let vars = handle.env_vars();
    assert!(vars.iter().any(|(k, _)| *k == "CLAUDELESS_SCENARIO"));
    assert!(vars.iter().any(|(k, _)| *k == "CLAUDELESS_CAPTURE"));
    assert!(vars
        .iter()
        .any(|(k, v)| *k == "CLAUDELESS_DELAY_MS" && v == "100"));
}

#[test]
fn test_binary_handle_paths() {
    let handle = SimulatorBuilder::new()
        .respond_to("test", "ok")
        .build_binary()
        .unwrap();

    assert!(handle.scenario_path().exists());
    // Capture file may not exist yet
}

#[test]
fn test_multiple_rules_order() {
    let sim = SimulatorBuilder::new()
        .respond_to("error", "Error response")
        .respond_to("warning", "Warning response")
        .respond_to("info", "Info response")
        .build_in_process()
        .unwrap();

    assert_eq!(sim.execute("an error occurred"), "Error response");
    assert_eq!(sim.execute("warning message"), "Warning response");
    assert_eq!(sim.execute("info log"), "Info response");
}

#[test]
fn test_scenario_from_config() {
    let config = ScenarioConfig {
        name: "test".to_string(),
        default_response: Some(ResponseSpec::Simple("default".to_string())),
        responses: vec![],
        tool_execution: None,
        ..Default::default()
    };

    let sim = SimulatorBuilder::new()
        .scenario(config)
        .build_in_process()
        .unwrap();

    assert_eq!(sim.execute("anything"), "default");
}
