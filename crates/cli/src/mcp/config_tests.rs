// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]
use super::*;

#[test]
fn test_parse_basic_config() {
    let json = r#"
    {
        "mcpServers": {
            "test": {
                "command": "node",
                "args": ["server.js"]
            }
        }
    }
    "#;

    let config = McpConfig::parse(json).unwrap();
    assert!(config.mcp_servers.contains_key("test"));
    assert_eq!(config.mcp_servers["test"].command, "node");
    assert_eq!(config.mcp_servers["test"].args, vec!["server.js"]);
}

#[test]
fn test_parse_json5_with_comments() {
    let json5 = r#"
    {
        // This is a comment
        "mcpServers": {
            "test": {
                "command": "node",
                "args": ["server.js"], // trailing comma OK
            }
        }
    }
    "#;

    let config = McpConfig::parse(json5).unwrap();
    assert!(config.mcp_servers.contains_key("test"));
}

#[test]
fn test_merge_configs() {
    let config1 = McpConfig::parse(r#"{"mcpServers": {"a": {"command": "a"}}}"#).unwrap();
    let config2 = McpConfig::parse(r#"{"mcpServers": {"b": {"command": "b"}}}"#).unwrap();

    let merged = McpConfig::merge([config1, config2]);
    assert_eq!(merged.mcp_servers.len(), 2);
    assert!(merged.mcp_servers.contains_key("a"));
    assert!(merged.mcp_servers.contains_key("b"));
}

#[test]
fn test_merge_override() {
    let config1 = McpConfig::parse(r#"{"mcpServers": {"a": {"command": "old"}}}"#).unwrap();
    let config2 = McpConfig::parse(r#"{"mcpServers": {"a": {"command": "new"}}}"#).unwrap();

    let merged = McpConfig::merge([config1, config2]);
    assert_eq!(merged.mcp_servers.len(), 1);
    assert_eq!(merged.mcp_servers["a"].command, "new");
}

#[test]
fn test_env_var_in_config() {
    let json = r#"
    {
        "mcpServers": {
            "github": {
                "command": "npx",
                "env": {
                    "GITHUB_TOKEN": "${GITHUB_TOKEN}"
                }
            }
        }
    }
    "#;

    let config = McpConfig::parse(json).unwrap();
    assert_eq!(
        config.mcp_servers["github"].env.get("GITHUB_TOKEN"),
        Some(&"${GITHUB_TOKEN}".to_string())
    );
}

#[test]
fn test_default_timeout() {
    let json = r#"{"mcpServers": {"test": {"command": "node"}}}"#;
    let config = McpConfig::parse(json).unwrap();
    assert_eq!(config.mcp_servers["test"].timeout_ms, 30000);
}

#[test]
fn test_custom_timeout() {
    let json = r#"{"mcpServers": {"test": {"command": "node", "timeoutMs": 60000}}}"#;
    let config = McpConfig::parse(json).unwrap();
    assert_eq!(config.mcp_servers["test"].timeout_ms, 60000);
}

#[test]
fn test_server_names() {
    let json = r#"{"mcpServers": {"a": {"command": "a"}, "b": {"command": "b"}}}"#;
    let config = McpConfig::parse(json).unwrap();
    let names = config.server_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"a"));
    assert!(names.contains(&"b"));
}

#[test]
fn test_empty_config() {
    let json = r#"{}"#;
    let config = McpConfig::parse(json).unwrap();
    assert!(!config.has_servers());
}

#[test]
fn test_load_inline_json() {
    let json = r#"{"mcpServers": {"test": {"command": "echo"}}}"#;
    let config = load_mcp_config(json).unwrap();
    assert!(config.has_servers());
}

#[test]
fn test_invalid_json_error() {
    let result = McpConfig::parse("not valid json");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("parse"));
}
