// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool permission pattern matching.
//!
//! Implements pattern matching for `permissions.allow` and `permissions.deny` settings.
//!
//! Claude Code uses patterns like:
//! - `"Read"` - matches all Read tool calls
//! - `"Bash(npm test)"` - matches Bash with specific command
//! - `"Bash(npm:*)"` - prefix pattern matching commands starting with "npm"
//! - `"Write(*.md)"` - glob pattern for file paths
//! - `"Edit"` - matches all Edit tool calls

use crate::state::PermissionSettings;
use glob::Pattern;

/// A compiled tool permission pattern.
#[derive(Clone, Debug)]
pub struct ToolPattern {
    /// The tool name (e.g., "Bash", "Read", "Edit")
    pub tool: String,
    /// Optional argument pattern (e.g., "npm test", "npm *")
    pub argument: Option<CompiledPattern>,
}

/// A compiled pattern for matching tool arguments.
#[derive(Clone, Debug)]
pub enum CompiledPattern {
    /// Exact string match
    Exact(String),
    /// Prefix match (for :* patterns like "Bash(npm:*)")
    Prefix(String),
    /// Glob pattern (for file patterns like "Write(*.md)")
    Glob(Pattern),
}

impl ToolPattern {
    /// Parse a pattern string like "Bash(npm test)" or "Read".
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }

        // Check for parentheses: "Tool(argument)"
        if let Some(paren_start) = s.find('(') {
            if s.ends_with(')') {
                let tool = s[..paren_start].to_string();
                let arg = &s[paren_start + 1..s.len() - 1];

                // Check for :* suffix (prefix matching) - Claude's actual syntax
                // e.g., "Bash(npm:*)" means match any command starting with "npm"
                let pattern = if let Some(prefix) = arg.strip_suffix(":*") {
                    Some(CompiledPattern::Prefix(prefix.to_string()))
                } else if arg.contains('*') || arg.contains('?') || arg.contains('[') {
                    // Glob pattern (for file paths like "*.md")
                    Pattern::new(arg).ok().map(CompiledPattern::Glob)
                } else {
                    // Exact match
                    Some(CompiledPattern::Exact(arg.to_string()))
                };

                return Some(Self {
                    tool,
                    argument: pattern,
                });
            }
        }

        // No parentheses - match all calls to this tool
        Some(Self {
            tool: s.to_string(),
            argument: None,
        })
    }

    /// Check if this pattern matches a tool call.
    ///
    /// # Arguments
    /// * `tool_name` - The tool being called (e.g., "Bash")
    /// * `tool_input` - The tool input as a string representation
    pub fn matches(&self, tool_name: &str, tool_input: Option<&str>) -> bool {
        // Tool name must match (case-insensitive)
        if !self.tool.eq_ignore_ascii_case(tool_name) {
            return false;
        }

        // If no argument pattern, match all calls to this tool
        let Some(ref arg_pattern) = self.argument else {
            return true;
        };

        // If pattern requires argument but none provided, no match
        let Some(input) = tool_input else {
            return false;
        };

        match arg_pattern {
            CompiledPattern::Exact(exact) => input == exact,
            CompiledPattern::Prefix(prefix) => input.starts_with(prefix),
            CompiledPattern::Glob(glob) => glob.matches(input),
        }
    }
}

/// A collection of allow/deny patterns for permission checking.
#[derive(Clone, Debug, Default)]
pub struct PermissionPatterns {
    /// Patterns that auto-approve tool calls
    pub allow: Vec<ToolPattern>,
    /// Patterns that deny tool calls
    pub deny: Vec<ToolPattern>,
}

impl PermissionPatterns {
    /// Create from permission settings.
    pub fn from_settings(settings: &PermissionSettings) -> Self {
        Self {
            allow: settings
                .allow
                .iter()
                .filter_map(|s| ToolPattern::parse(s))
                .collect(),
            deny: settings
                .deny
                .iter()
                .filter_map(|s| ToolPattern::parse(s))
                .collect(),
        }
    }

    /// Check if a tool call is explicitly allowed by settings.
    pub fn is_allowed(&self, tool: &str, input: Option<&str>) -> bool {
        self.allow.iter().any(|p| p.matches(tool, input))
    }

    /// Check if a tool call is explicitly denied by settings.
    pub fn is_denied(&self, tool: &str, input: Option<&str>) -> bool {
        self.deny.iter().any(|p| p.matches(tool, input))
    }

    /// Check if there are any patterns defined.
    pub fn is_empty(&self) -> bool {
        self.allow.is_empty() && self.deny.is_empty()
    }
}

#[cfg(test)]
#[path = "pattern_tests.rs"]
mod tests;
