// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

// =========================================================================
// ToolPattern Parsing Tests
// =========================================================================

#[test]
fn test_parse_simple_tool() {
    let pattern = ToolPattern::parse("Read").unwrap();
    assert_eq!(pattern.tool, "Read");
    assert!(pattern.argument.is_none());
}

#[test]
fn test_parse_tool_with_exact_arg() {
    let pattern = ToolPattern::parse("Bash(npm test)").unwrap();
    assert_eq!(pattern.tool, "Bash");
    assert!(matches!(
        pattern.argument,
        Some(CompiledPattern::Exact(ref s)) if s == "npm test"
    ));
}

#[test]
fn test_parse_tool_with_prefix() {
    let pattern = ToolPattern::parse("Bash(npm:*)").unwrap();
    assert_eq!(pattern.tool, "Bash");
    assert!(matches!(
        pattern.argument,
        Some(CompiledPattern::Prefix(ref s)) if s == "npm"
    ));
}

#[test]
fn test_parse_tool_with_glob() {
    let pattern = ToolPattern::parse("Write(*.md)").unwrap();
    assert_eq!(pattern.tool, "Write");
    assert!(matches!(pattern.argument, Some(CompiledPattern::Glob(_))));
}

#[test]
fn test_parse_empty_string() {
    assert!(ToolPattern::parse("").is_none());
    assert!(ToolPattern::parse("   ").is_none());
}

#[test]
fn test_parse_whitespace_trimmed() {
    let pattern = ToolPattern::parse("  Read  ").unwrap();
    assert_eq!(pattern.tool, "Read");
}

#[test]
fn test_parse_incomplete_parens() {
    // "Bash(" without closing paren should parse as tool name "Bash("
    let pattern = ToolPattern::parse("Bash(").unwrap();
    assert_eq!(pattern.tool, "Bash(");
    assert!(pattern.argument.is_none());
}

#[test]
fn test_parse_glob_with_question_mark() {
    let pattern = ToolPattern::parse("Bash(npm ?)").unwrap();
    assert!(matches!(pattern.argument, Some(CompiledPattern::Glob(_))));
}

#[test]
fn test_parse_glob_with_brackets() {
    let pattern = ToolPattern::parse("Bash([abc]*)").unwrap();
    assert!(matches!(pattern.argument, Some(CompiledPattern::Glob(_))));
}

// =========================================================================
// ToolPattern Matching Tests
// =========================================================================

#[test]
fn test_matches_simple_tool() {
    let pattern = ToolPattern::parse("Read").unwrap();
    assert!(pattern.matches("Read", None));
    assert!(pattern.matches("Read", Some("/path/to/file")));
    assert!(!pattern.matches("Write", None));
}

#[test]
fn test_matches_case_insensitive() {
    let pattern = ToolPattern::parse("Read").unwrap();
    assert!(pattern.matches("read", None));
    assert!(pattern.matches("READ", None));
    assert!(pattern.matches("rEaD", None));
}

#[test]
fn test_matches_exact_arg() {
    let pattern = ToolPattern::parse("Bash(npm test)").unwrap();
    assert!(pattern.matches("Bash", Some("npm test")));
    assert!(!pattern.matches("Bash", Some("npm install")));
    assert!(!pattern.matches("Bash", None));
}

#[test]
fn test_matches_prefix() {
    let pattern = ToolPattern::parse("Bash(npm:*)").unwrap();
    assert!(pattern.matches("Bash", Some("npm")));
    assert!(pattern.matches("Bash", Some("npm test")));
    assert!(pattern.matches("Bash", Some("npm install")));
    assert!(pattern.matches("Bash", Some("npm run build")));
    assert!(!pattern.matches("Bash", Some("cargo test")));
    // Prefix doesn't match partial word matches
    assert!(!pattern.matches("Bash", Some("npx")));
    assert!(!pattern.matches("Bash", Some("pnpm")));
}

#[test]
fn test_matches_prefix_with_space() {
    // Prefix can include space for more specific matching
    let pattern = ToolPattern::parse("Bash(npm run :*)").unwrap();
    assert!(pattern.matches("Bash", Some("npm run build")));
    assert!(pattern.matches("Bash", Some("npm run test")));
    assert!(!pattern.matches("Bash", Some("npm test")));
    assert!(!pattern.matches("Bash", Some("npm install")));
}

#[test]
fn test_matches_glob_question() {
    let pattern = ToolPattern::parse("Bash(npm ?est)").unwrap();
    assert!(pattern.matches("Bash", Some("npm test")));
    assert!(pattern.matches("Bash", Some("npm best")));
    assert!(!pattern.matches("Bash", Some("npm tests")));
}

#[test]
fn test_matches_tool_name_case_with_arg() {
    let pattern = ToolPattern::parse("Bash(npm test)").unwrap();
    assert!(pattern.matches("bash", Some("npm test")));
    assert!(pattern.matches("BASH", Some("npm test")));
}

// =========================================================================
// PermissionPatterns Tests
// =========================================================================

#[test]
fn test_patterns_from_settings() {
    let settings = PermissionSettings {
        allow: vec!["Read".to_string(), "Bash(npm test)".to_string()],
        deny: vec!["Bash(rm:*)".to_string()],
        additional_directories: vec![],
    };

    let patterns = PermissionPatterns::from_settings(&settings);

    assert_eq!(patterns.allow.len(), 2);
    assert_eq!(patterns.deny.len(), 1);
}

#[test]
fn test_patterns_is_allowed() {
    let settings = PermissionSettings {
        allow: vec!["Read".to_string(), "Bash(npm:*)".to_string()],
        deny: vec![],
        additional_directories: vec![],
    };

    let patterns = PermissionPatterns::from_settings(&settings);

    assert!(patterns.is_allowed("Read", None));
    assert!(patterns.is_allowed("Read", Some("/any/path")));
    assert!(patterns.is_allowed("Bash", Some("npm test")));
    assert!(patterns.is_allowed("Bash", Some("npm")));
    assert!(!patterns.is_allowed("Write", None));
    assert!(!patterns.is_allowed("Bash", Some("cargo build")));
}

#[test]
fn test_patterns_is_denied() {
    let settings = PermissionSettings {
        allow: vec![],
        deny: vec!["Bash(rm:*)".to_string(), "Bash(sudo:*)".to_string()],
        additional_directories: vec![],
    };

    let patterns = PermissionPatterns::from_settings(&settings);

    assert!(patterns.is_denied("Bash", Some("rm -rf /")));
    assert!(patterns.is_denied("Bash", Some("sudo apt install")));
    assert!(!patterns.is_denied("Bash", Some("npm test")));
    assert!(!patterns.is_denied("Read", None));
}

#[test]
fn test_patterns_allow_and_deny_both_checked() {
    let settings = PermissionSettings {
        allow: vec!["Bash".to_string()],
        deny: vec!["Bash(rm:*)".to_string()],
        additional_directories: vec![],
    };

    let patterns = PermissionPatterns::from_settings(&settings);

    // Generic Bash is allowed
    assert!(patterns.is_allowed("Bash", Some("echo hello")));
    // But rm commands are also denied
    assert!(patterns.is_denied("Bash", Some("rm -rf /")));

    // Both can be true - caller must check deny first
    assert!(patterns.is_allowed("Bash", Some("rm file")));
    assert!(patterns.is_denied("Bash", Some("rm file")));
}

#[test]
fn test_patterns_empty() {
    let patterns = PermissionPatterns::default();
    assert!(patterns.is_empty());
    assert!(!patterns.is_allowed("Read", None));
    assert!(!patterns.is_denied("Read", None));
}

#[test]
fn test_patterns_skips_invalid() {
    let settings = PermissionSettings {
        allow: vec!["".to_string(), "Read".to_string(), "   ".to_string()],
        deny: vec![],
        additional_directories: vec![],
    };

    let patterns = PermissionPatterns::from_settings(&settings);

    // Only "Read" should be parsed
    assert_eq!(patterns.allow.len(), 1);
}

// =========================================================================
// Edge Cases
// =========================================================================

#[test]
fn test_empty_arg_pattern() {
    // Bash() with empty arg
    let pattern = ToolPattern::parse("Bash()").unwrap();
    assert_eq!(pattern.tool, "Bash");
    // Empty string inside parens is an exact match for empty string
    assert!(matches!(
        pattern.argument,
        Some(CompiledPattern::Exact(ref s)) if s.is_empty()
    ));
}

#[test]
fn test_pattern_with_special_chars() {
    // Pattern with path-like content
    let pattern = ToolPattern::parse("Read(/path/to/file.txt)").unwrap();
    assert_eq!(pattern.tool, "Read");
    assert!(matches!(
        pattern.argument,
        Some(CompiledPattern::Exact(ref s)) if s == "/path/to/file.txt"
    ));
}

#[test]
fn test_pattern_with_nested_parens() {
    // This is tricky - we only look for the first ( and last )
    let pattern = ToolPattern::parse("Bash(echo (hello))").unwrap();
    assert_eq!(pattern.tool, "Bash");
    // The argument should be "echo (hello)"
    assert!(matches!(
        pattern.argument,
        Some(CompiledPattern::Exact(ref s)) if s == "echo (hello)"
    ));
}

#[test]
fn test_glob_with_path() {
    let pattern = ToolPattern::parse("Write(*.md)").unwrap();
    assert!(pattern.matches("Write", Some("README.md")));
    assert!(pattern.matches("Write", Some("CHANGELOG.md")));
    assert!(!pattern.matches("Write", Some("main.rs")));
}

#[test]
fn test_prefix_complex_pattern() {
    let pattern = ToolPattern::parse("Bash(npm run :*)").unwrap();
    assert!(pattern.matches("Bash", Some("npm run build")));
    assert!(pattern.matches("Bash", Some("npm run test")));
    assert!(pattern.matches("Bash", Some("npm run dev")));
    assert!(!pattern.matches("Bash", Some("npm install")));
    assert!(!pattern.matches("Bash", Some("npm test")));
}
