// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! CLI flag comparison and validation.

use std::collections::BTreeMap;

/// Flag implementation status
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlagStatus {
    /// Fully implemented and tested
    Implemented,
    /// Partially implemented (some behaviors missing)
    Partial(String),
    /// Not implemented but needed
    MissingNeeded,
    /// Not implemented, low priority
    MissingLowPriority,
    /// Intentionally not supported
    NotSupported(String),
}

/// CLI flag definition
#[derive(Clone, Debug)]
pub struct FlagDef {
    pub name: &'static str,
    pub short: Option<char>,
    pub takes_value: bool,
    pub description: &'static str,
    pub status: FlagStatus,
}

/// Audit of CLI flag implementation
pub struct CliAudit {
    flags: BTreeMap<&'static str, FlagDef>,
}

impl CliAudit {
    /// Create audit with all known Claude CLI flags
    pub fn new() -> Self {
        let mut flags = BTreeMap::new();

        // Core flags - implemented
        flags.insert(
            "print",
            FlagDef {
                name: "print",
                short: Some('p'),
                takes_value: false,
                description: "Print response and exit",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "model",
            FlagDef {
                name: "model",
                short: None,
                takes_value: true,
                description: "Model for the session",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "output-format",
            FlagDef {
                name: "output-format",
                short: None,
                takes_value: true,
                description: "Output format (text/json/stream-json)",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "max-tokens",
            FlagDef {
                name: "max-tokens",
                short: None,
                takes_value: true,
                description: "Maximum tokens in response",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "system-prompt",
            FlagDef {
                name: "system-prompt",
                short: None,
                takes_value: true,
                description: "System prompt for conversation",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "continue",
            FlagDef {
                name: "continue",
                short: Some('c'),
                takes_value: false,
                description: "Continue previous conversation",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "resume",
            FlagDef {
                name: "resume",
                short: Some('r'),
                takes_value: true,
                description: "Resume by session ID",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "allowedTools",
            FlagDef {
                name: "allowedTools",
                short: None,
                takes_value: true,
                description: "Allowed tools list",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "disallowedTools",
            FlagDef {
                name: "disallowedTools",
                short: None,
                takes_value: true,
                description: "Disallowed tools list",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "permission-mode",
            FlagDef {
                name: "permission-mode",
                short: None,
                takes_value: true,
                description: "Permission mode for tool execution",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "cwd",
            FlagDef {
                name: "cwd",
                short: None,
                takes_value: true,
                description: "Working directory",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "input-format",
            FlagDef {
                name: "input-format",
                short: None,
                takes_value: true,
                description: "Input format (text/stream-json)",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "session-id",
            FlagDef {
                name: "session-id",
                short: None,
                takes_value: true,
                description: "Use specific session UUID",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "verbose",
            FlagDef {
                name: "verbose",
                short: None,
                takes_value: false,
                description: "Verbose output mode",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "debug",
            FlagDef {
                name: "debug",
                short: Some('d'),
                takes_value: true,
                description: "Debug mode with filter",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "include-partial-messages",
            FlagDef {
                name: "include-partial-messages",
                short: None,
                takes_value: false,
                description: "Include partial message chunks",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "fallback-model",
            FlagDef {
                name: "fallback-model",
                short: None,
                takes_value: true,
                description: "Fallback model on overload",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "max-budget-usd",
            FlagDef {
                name: "max-budget-usd",
                short: None,
                takes_value: true,
                description: "Maximum budget in USD",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "input-file",
            FlagDef {
                name: "input-file",
                short: None,
                takes_value: true,
                description: "Read prompt from file",
                status: FlagStatus::Implemented,
            },
        );

        // MCP flags - implemented
        flags.insert(
            "mcp-config",
            FlagDef {
                name: "mcp-config",
                short: None,
                takes_value: true,
                description: "MCP server configuration",
                status: FlagStatus::Partial("Config parsing only, no server execution".into()),
            },
        );

        flags.insert(
            "mcp-debug",
            FlagDef {
                name: "mcp-debug",
                short: None,
                takes_value: false,
                description: "MCP debug mode",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "strict-mcp-config",
            FlagDef {
                name: "strict-mcp-config",
                short: None,
                takes_value: false,
                description: "Strict MCP config mode",
                status: FlagStatus::Implemented,
            },
        );

        // Permission bypass flags - implemented
        flags.insert(
            "allow-dangerously-skip-permissions",
            FlagDef {
                name: "allow-dangerously-skip-permissions",
                short: None,
                takes_value: false,
                description: "Enable permission bypass option",
                status: FlagStatus::Implemented,
            },
        );

        flags.insert(
            "add-dir",
            FlagDef {
                name: "add-dir",
                short: None,
                takes_value: true,
                description: "Additional directories",
                status: FlagStatus::MissingLowPriority,
            },
        );

        flags.insert(
            "agent",
            FlagDef {
                name: "agent",
                short: None,
                takes_value: true,
                description: "Custom agent",
                status: FlagStatus::MissingLowPriority,
            },
        );

        flags.insert(
            "betas",
            FlagDef {
                name: "betas",
                short: None,
                takes_value: true,
                description: "Beta headers",
                status: FlagStatus::MissingLowPriority,
            },
        );

        flags.insert(
            "chrome",
            FlagDef {
                name: "chrome",
                short: None,
                takes_value: false,
                description: "Chrome integration",
                status: FlagStatus::NotSupported("Chrome integration out of scope".into()),
            },
        );

        flags.insert(
            "json-schema",
            FlagDef {
                name: "json-schema",
                short: None,
                takes_value: true,
                description: "Structured output schema",
                status: FlagStatus::MissingLowPriority,
            },
        );

        flags.insert(
            "tools",
            FlagDef {
                name: "tools",
                short: None,
                takes_value: true,
                description: "Built-in tool list",
                status: FlagStatus::MissingLowPriority,
            },
        );

        flags.insert(
            "dangerously-skip-permissions",
            FlagDef {
                name: "dangerously-skip-permissions",
                short: None,
                takes_value: false,
                description: "Bypass permissions",
                status: FlagStatus::Implemented,
            },
        );

        Self { flags }
    }

    /// Get all flags
    pub fn all_flags(&self) -> impl Iterator<Item = &FlagDef> {
        self.flags.values()
    }

    /// Get all flags with given status
    pub fn flags_with_status(&self, status: &FlagStatus) -> Vec<&FlagDef> {
        self.flags
            .values()
            .filter(|f| std::mem::discriminant(&f.status) == std::mem::discriminant(status))
            .collect()
    }

    /// Get a specific flag
    pub fn get(&self, name: &str) -> Option<&FlagDef> {
        self.flags.get(name)
    }

    /// Count flags by status
    pub fn count_by_status(&self) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        counts.insert("implemented", 0);
        counts.insert("partial", 0);
        counts.insert("missing_needed", 0);
        counts.insert("missing_low_priority", 0);
        counts.insert("not_supported", 0);

        for flag in self.flags.values() {
            let key = match &flag.status {
                FlagStatus::Implemented => "implemented",
                FlagStatus::Partial(_) => "partial",
                FlagStatus::MissingNeeded => "missing_needed",
                FlagStatus::MissingLowPriority => "missing_low_priority",
                FlagStatus::NotSupported(_) => "not_supported",
            };
            *counts.get_mut(key).unwrap() += 1;
        }

        counts
    }

    /// Generate markdown report
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# CLI Flag Audit\n\n");

        md.push_str("## Implemented\n\n");
        for flag in self.flags_with_status(&FlagStatus::Implemented) {
            md.push_str(&format!("- `--{}` - {}\n", flag.name, flag.description));
        }

        let partial_flags = self.flags_with_status(&FlagStatus::Partial(String::new()));
        if !partial_flags.is_empty() {
            md.push_str("\n## Partial\n\n");
            for flag in partial_flags {
                if let FlagStatus::Partial(note) = &flag.status {
                    md.push_str(&format!(
                        "- `--{}` - {} ({})\n",
                        flag.name, flag.description, note
                    ));
                }
            }
        }

        let missing_needed = self.flags_with_status(&FlagStatus::MissingNeeded);
        if !missing_needed.is_empty() {
            md.push_str("\n## Missing (Needed)\n\n");
            for flag in missing_needed {
                md.push_str(&format!("- `--{}` - {}\n", flag.name, flag.description));
            }
        }

        let missing_low = self.flags_with_status(&FlagStatus::MissingLowPriority);
        if !missing_low.is_empty() {
            md.push_str("\n## Missing (Low Priority)\n\n");
            for flag in missing_low {
                md.push_str(&format!("- `--{}` - {}\n", flag.name, flag.description));
            }
        }

        let not_supported = self.flags_with_status(&FlagStatus::NotSupported(String::new()));
        if !not_supported.is_empty() {
            md.push_str("\n## Not Supported\n\n");
            for flag in not_supported {
                if let FlagStatus::NotSupported(reason) = &flag.status {
                    md.push_str(&format!(
                        "- `--{}` - {} ({})\n",
                        flag.name, flag.description, reason
                    ));
                }
            }
        }

        md
    }
}

impl Default for CliAudit {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_has_core_flags() {
        let audit = CliAudit::new();
        assert!(audit.get("print").is_some());
        assert!(audit.get("model").is_some());
        assert!(audit.get("output-format").is_some());
    }

    #[test]
    fn test_flags_with_status() {
        let audit = CliAudit::new();
        let implemented = audit.flags_with_status(&FlagStatus::Implemented);
        assert!(!implemented.is_empty());

        // Verify all returned flags are actually implemented
        for flag in implemented {
            assert!(matches!(flag.status, FlagStatus::Implemented));
        }
    }

    #[test]
    fn test_count_by_status() {
        let audit = CliAudit::new();
        let counts = audit.count_by_status();

        assert!(counts["implemented"] > 0);
        // Total should equal number of flags
        let total: usize = counts.values().sum();
        assert_eq!(total, audit.flags.len());
    }

    #[test]
    fn test_to_markdown() {
        let audit = CliAudit::new();
        let md = audit.to_markdown();

        assert!(md.contains("# CLI Flag Audit"));
        assert!(md.contains("## Implemented"));
        assert!(md.contains("--print"));
        assert!(md.contains("--model"));
    }

    #[test]
    fn test_no_missing_needed_flags() {
        // This test ensures all needed flags are implemented
        let audit = CliAudit::new();
        let missing = audit.flags_with_status(&FlagStatus::MissingNeeded);

        assert!(
            missing.is_empty(),
            "Missing needed flags: {:?}",
            missing.iter().map(|f| f.name).collect::<Vec<_>>()
        );
    }
}
