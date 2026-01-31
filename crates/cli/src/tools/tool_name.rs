// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

// NOTE(compat): Keep full API surface for future use
#![allow(dead_code)]

//! Tool name enum for type-safe tool identification.

use std::fmt;

/// Enum representing all known tool names.
///
/// This provides type-safe tool identification, replacing raw string literals
/// throughout the codebase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolName {
    // Builtin executors
    Bash,
    Read,
    Write,
    Edit,
    Glob,
    Grep,

    // Stateful tools
    TodoWrite,
    ExitPlanMode,

    // Network tools
    WebFetch,
    WebSearch,

    // Other
    NotebookEdit,
    Task,
}

impl ToolName {
    /// Get the string representation of the tool name.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Bash => "Bash",
            Self::Read => "Read",
            Self::Write => "Write",
            Self::Edit => "Edit",
            Self::Glob => "Glob",
            Self::Grep => "Grep",
            Self::TodoWrite => "TodoWrite",
            Self::ExitPlanMode => "ExitPlanMode",
            Self::WebFetch => "WebFetch",
            Self::WebSearch => "WebSearch",
            Self::NotebookEdit => "NotebookEdit",
            Self::Task => "Task",
        }
    }

    /// Try to parse a tool name from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Bash" => Some(Self::Bash),
            "Read" => Some(Self::Read),
            "Write" => Some(Self::Write),
            "Edit" => Some(Self::Edit),
            "Glob" => Some(Self::Glob),
            "Grep" => Some(Self::Grep),
            "TodoWrite" => Some(Self::TodoWrite),
            "ExitPlanMode" => Some(Self::ExitPlanMode),
            "WebFetch" => Some(Self::WebFetch),
            "WebSearch" => Some(Self::WebSearch),
            "NotebookEdit" => Some(Self::NotebookEdit),
            "Task" => Some(Self::Task),
            _ => None,
        }
    }

    /// Get the permission action for this tool.
    pub const fn action(&self) -> &'static str {
        match self {
            Self::Bash => "execute",
            Self::Read | Self::Glob | Self::Grep => "read",
            Self::Write | Self::Edit | Self::NotebookEdit => "write",
            Self::WebFetch | Self::WebSearch => "network",
            Self::Task => "delegate",
            Self::TodoWrite | Self::ExitPlanMode => "state",
        }
    }
}

impl fmt::Display for ToolName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
#[path = "tool_name_tests.rs"]
mod tests;
