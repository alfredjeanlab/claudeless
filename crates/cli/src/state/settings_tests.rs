// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]
use super::*;

#[test]
fn test_new_settings() {
    let settings = Settings::new();
    assert!(settings.is_empty());
}

#[test]
fn test_set_get() {
    let mut settings = Settings::new();

    settings.set("name", "test");
    settings.set("count", 42);
    settings.set("enabled", true);

    assert_eq!(settings.get_str("name"), Some("test"));
    assert_eq!(settings.get_i64("count"), Some(42));
    assert_eq!(settings.get_bool("enabled"), Some(true));
}

#[test]
fn test_contains_remove() {
    let mut settings = Settings::new();
    settings.set("key", "value");

    assert!(settings.contains("key"));
    assert!(!settings.contains("other"));

    let removed = settings.remove("key");
    assert!(removed.is_some());
    assert!(!settings.contains("key"));
}

#[test]
fn test_save_load() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.json");

    let mut settings = Settings::new();
    settings.set("theme", "dark");
    settings.set("maxTokens", 4096);
    settings.save(&path).unwrap();

    let loaded = Settings::load(&path).unwrap();
    assert_eq!(loaded.get_str("theme"), Some("dark"));
    assert_eq!(loaded.get_i64("maxTokens"), Some(4096));
}

#[test]
fn test_clear() {
    let mut settings = Settings::new();
    settings.set("a", 1);
    settings.set("b", 2);

    assert!(!settings.is_empty());
    settings.clear();
    assert!(settings.is_empty());
}

#[test]
fn test_keys() {
    let mut settings = Settings::new();
    settings.set("alpha", 1);
    settings.set("beta", 2);

    let keys: Vec<_> = settings.keys().collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"alpha"));
    assert!(keys.contains(&"beta"));
}

#[test]
fn test_nested_values() {
    let mut settings = Settings::new();
    settings.set(
        "nested",
        serde_json::json!({
            "inner": {
                "value": 123
            }
        }),
    );

    let nested = settings.get("nested").unwrap();
    assert_eq!(nested["inner"]["value"], 123);
}

// ClaudeSettings tests

#[test]
fn test_claude_settings_default() {
    let settings = ClaudeSettings::default();
    assert!(settings.permissions.allow.is_empty());
    assert!(settings.permissions.deny.is_empty());
    assert!(settings.permissions.additional_directories.is_empty());
    assert!(settings.mcp_servers.is_empty());
    assert!(settings.env.is_empty());
}

#[test]
fn test_claude_settings_parse_permissions() {
    let json = r#"{
        "permissions": {
            "allow": ["Read", "Bash(npm test)"],
            "deny": ["Bash(rm *)"],
            "additionalDirectories": ["/tmp/workspace"]
        }
    }"#;

    let settings: ClaudeSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.permissions.allow, vec!["Read", "Bash(npm test)"]);
    assert_eq!(settings.permissions.deny, vec!["Bash(rm *)"]);
    assert_eq!(
        settings.permissions.additional_directories,
        vec!["/tmp/workspace"]
    );
}

#[test]
fn test_claude_settings_parse_mcp_servers() {
    let json = r#"{
        "mcpServers": {
            "filesystem": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-filesystem"],
                "cwd": "/home/user"
            }
        }
    }"#;

    let settings: ClaudeSettings = serde_json::from_str(json).unwrap();
    let server = settings.mcp_servers.get("filesystem").unwrap();
    assert_eq!(server.command.as_deref(), Some("npx"));
    assert_eq!(
        server.args,
        vec!["-y", "@modelcontextprotocol/server-filesystem"]
    );
    assert_eq!(server.cwd.as_deref(), Some("/home/user"));
}

#[test]
fn test_claude_settings_parse_env() {
    let json = r#"{
        "env": {
            "API_KEY": "secret",
            "DEBUG": "true"
        }
    }"#;

    let settings: ClaudeSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.env.get("API_KEY"), Some(&"secret".to_string()));
    assert_eq!(settings.env.get("DEBUG"), Some(&"true".to_string()));
}

#[test]
fn test_claude_settings_preserves_unknown_fields() {
    let json = r#"{
        "permissions": {"allow": ["Read"]},
        "futureField": "some value",
        "nestedFuture": {"key": 123}
    }"#;

    let settings: ClaudeSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.permissions.allow, vec!["Read"]);
    assert!(settings.extra.contains_key("futureField"));
    assert!(settings.extra.contains_key("nestedFuture"));
}

#[test]
fn test_claude_settings_merge_permissions_replace() {
    let mut base = ClaudeSettings::default();
    base.permissions.allow = vec!["Read".to_string(), "Glob".to_string()];

    let override_settings = ClaudeSettings {
        permissions: PermissionSettings {
            allow: vec!["Write".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    base.merge(override_settings);

    // Arrays are replaced, not merged
    assert_eq!(base.permissions.allow, vec!["Write"]);
}

#[test]
fn test_claude_settings_merge_permissions_empty_doesnt_override() {
    let mut base = ClaudeSettings::default();
    base.permissions.allow = vec!["Read".to_string()];
    base.permissions.deny = vec!["Bash(rm *)".to_string()];

    let override_settings = ClaudeSettings {
        permissions: PermissionSettings {
            allow: vec![],                          // Empty - should not override
            deny: vec!["Bash(sudo *)".to_string()], // Non-empty - should override
            ..Default::default()
        },
        ..Default::default()
    };

    base.merge(override_settings);

    // Empty array doesn't override
    assert_eq!(base.permissions.allow, vec!["Read"]);
    // Non-empty array does override
    assert_eq!(base.permissions.deny, vec!["Bash(sudo *)"]);
}

#[test]
fn test_claude_settings_merge_env() {
    let mut base = ClaudeSettings::default();
    base.env.insert("A".to_string(), "1".to_string());
    base.env.insert("B".to_string(), "2".to_string());

    let mut override_settings = ClaudeSettings::default();
    override_settings
        .env
        .insert("B".to_string(), "3".to_string());
    override_settings
        .env
        .insert("C".to_string(), "4".to_string());

    base.merge(override_settings);

    // Maps are merged, later values win
    assert_eq!(base.env.get("A"), Some(&"1".to_string()));
    assert_eq!(base.env.get("B"), Some(&"3".to_string())); // Overridden
    assert_eq!(base.env.get("C"), Some(&"4".to_string())); // Added
}

#[test]
fn test_claude_settings_merge_mcp_servers() {
    let mut base = ClaudeSettings::default();
    base.mcp_servers.insert(
        "server1".to_string(),
        McpServerConfig {
            command: Some("cmd1".to_string()),
            ..Default::default()
        },
    );

    let mut override_settings = ClaudeSettings::default();
    override_settings.mcp_servers.insert(
        "server2".to_string(),
        McpServerConfig {
            command: Some("cmd2".to_string()),
            ..Default::default()
        },
    );

    base.merge(override_settings);

    // Both servers should be present
    assert!(base.mcp_servers.contains_key("server1"));
    assert!(base.mcp_servers.contains_key("server2"));
}

#[test]
fn test_claude_settings_load_save() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.json");

    // Create a settings file
    std::fs::write(
        &path,
        r#"{
            "permissions": {"allow": ["Read"]},
            "env": {"KEY": "value"}
        }"#,
    )
    .unwrap();

    let settings = ClaudeSettings::load(&path).unwrap();
    assert_eq!(settings.permissions.allow, vec!["Read"]);
    assert_eq!(settings.env.get("KEY"), Some(&"value".to_string()));
}

#[test]
fn test_settings_as_claude_settings() {
    let mut settings = Settings::new();
    settings.set(
        "permissions",
        serde_json::json!({
            "allow": ["Read"],
            "deny": []
        }),
    );
    settings.set("env", serde_json::json!({"KEY": "value"}));

    let claude_settings = settings.as_claude_settings().unwrap();
    assert_eq!(claude_settings.permissions.allow, vec!["Read"]);
    assert_eq!(claude_settings.env.get("KEY"), Some(&"value".to_string()));
}
