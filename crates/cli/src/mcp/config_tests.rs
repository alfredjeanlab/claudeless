// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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

// =============================================================================
// MCP Tool Naming Convention Tests
// =============================================================================
// These tests document the real Claude CLI naming convention for MCP tools.

/// Real Claude CLI uses `mcp__<server>__<tool>` format for MCP tools.
///
/// Observed from `claude --mcp-config ... --output-format stream-json --verbose`:
/// - "mcp__filesystem__read_file"
/// - "mcp__filesystem__write_file"
/// - "mcp__filesystem__list_directory"
#[test]
fn test_qualified_name_format() {
    let tool = McpToolDef {
        name: "read_file".into(),
        description: "Read a file".into(),
        input_schema: serde_json::json!({}),
        server_name: "filesystem".into(),
    };

    assert_eq!(tool.qualified_name(), "mcp__filesystem__read_file");
}

#[test]
fn test_qualified_name_preserves_underscores_in_tool_name() {
    let tool = McpToolDef {
        name: "read_text_file".into(),
        description: "Read a text file".into(),
        input_schema: serde_json::json!({}),
        server_name: "filesystem".into(),
    };

    assert_eq!(tool.qualified_name(), "mcp__filesystem__read_text_file");
}

#[test]
fn test_qualified_name_preserves_underscores_in_server_name() {
    let tool = McpToolDef {
        name: "query".into(),
        description: "Run a query".into(),
        input_schema: serde_json::json!({}),
        server_name: "my_database".into(),
    };

    assert_eq!(tool.qualified_name(), "mcp__my_database__query");
}

#[test]
fn test_parse_qualified_name_success() {
    let parsed = McpToolDef::parse_qualified_name("mcp__filesystem__read_file");
    assert_eq!(
        parsed,
        Some(("filesystem".to_string(), "read_file".to_string()))
    );
}

#[test]
fn test_parse_qualified_name_with_underscores() {
    let parsed = McpToolDef::parse_qualified_name("mcp__filesystem__read_text_file");
    assert_eq!(
        parsed,
        Some(("filesystem".to_string(), "read_text_file".to_string()))
    );
}

#[test]
fn test_parse_qualified_name_rejects_builtin_tool() {
    assert_eq!(McpToolDef::parse_qualified_name("Read"), None);
    assert_eq!(McpToolDef::parse_qualified_name("Write"), None);
    assert_eq!(McpToolDef::parse_qualified_name("Bash"), None);
}

#[test]
fn test_parse_qualified_name_rejects_malformed() {
    // Missing mcp__ prefix
    assert_eq!(
        McpToolDef::parse_qualified_name("filesystem__read_file"),
        None
    );

    // Missing second separator
    assert_eq!(
        McpToolDef::parse_qualified_name("mcp__filesystemread"),
        None
    );

    // Empty parts
    assert_eq!(McpToolDef::parse_qualified_name("mcp____read_file"), None);
}

#[test]
fn test_qualified_name_roundtrip() {
    let tool = McpToolDef {
        name: "list_directory".into(),
        description: "List directory".into(),
        input_schema: serde_json::json!({}),
        server_name: "filesystem".into(),
    };

    let qualified = tool.qualified_name();
    let parsed = McpToolDef::parse_qualified_name(&qualified);

    assert_eq!(parsed, Some((tool.server_name.clone(), tool.name.clone())));
}
