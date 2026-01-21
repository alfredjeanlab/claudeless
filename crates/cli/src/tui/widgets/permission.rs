// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Rich permission dialog widget.
//!
//! Renders permission dialogs for Bash commands, file edits, and file writes
//! matching the real Claude Code TUI format.

/// Categories of bash commands for permission text generation
#[derive(Debug, Clone, PartialEq, Eq)]
enum BashCommandCategory {
    /// Command reads from /etc/ directory
    ReadingEtc,
    /// Named command (npm, cargo, git, rm, etc.)
    NamedCommand(String),
    /// Fallback for complex or unrecognized commands
    Generic,
}

/// Categorize a bash command for permission text generation.
///
/// Priority:
/// 1. If command contains `/etc/` path, categorize as ReadingEtc
/// 2. Extract first word as the command name
/// 3. Fallback to Generic for empty or unrecognizable commands
fn categorize_bash_command(command: &str) -> BashCommandCategory {
    // Check for /etc/ access first (highest priority)
    if command.contains("/etc/") {
        return BashCommandCategory::ReadingEtc;
    }

    // Extract the first word (command name)
    let first_word = command.split_whitespace().next().unwrap_or("");

    // Handle commands with paths (e.g., /usr/bin/npm -> npm)
    let command_name = first_word.rsplit('/').next().unwrap_or(first_word);

    if command_name.is_empty() {
        BashCommandCategory::Generic
    } else {
        BashCommandCategory::NamedCommand(command_name.to_string())
    }
}

/// Type of permission being requested
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionType {
    /// Bash command execution
    Bash {
        command: String,
        description: Option<String>,
    },
    /// File edit with diff
    Edit {
        file_path: String,
        diff_lines: Vec<DiffLine>,
    },
    /// New file creation
    Write {
        file_path: String,
        content_lines: Vec<String>,
    },
}

/// A line in a diff preview
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub line_num: Option<u32>,
    pub kind: DiffKind,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffKind {
    Context,
    Added,
    Removed,
    NoNewline,
}

/// User's selection in the permission dialog
#[derive(Clone, Debug, PartialEq, Eq, Copy, Default)]
pub enum PermissionSelection {
    #[default]
    Yes, // Option 1: Yes (single request)
    YesSession, // Option 2: Yes, allow for session
    No,         // Option 3: No
}

impl PermissionSelection {
    /// Move to next selection (down)
    pub fn next(self) -> Self {
        match self {
            Self::Yes => Self::YesSession,
            Self::YesSession => Self::No,
            Self::No => Self::Yes,
        }
    }

    /// Move to previous selection (up)
    pub fn prev(self) -> Self {
        match self {
            Self::Yes => Self::No,
            Self::YesSession => Self::Yes,
            Self::No => Self::YesSession,
        }
    }
}

/// State for rich permission dialog
#[derive(Clone, Debug)]
pub struct RichPermissionDialog {
    pub permission_type: PermissionType,
    pub selected: PermissionSelection,
}

impl RichPermissionDialog {
    pub fn new(permission_type: PermissionType) -> Self {
        Self {
            permission_type,
            selected: PermissionSelection::Yes,
        }
    }

    /// Get the option 2 text based on permission type
    fn option2_text(&self) -> String {
        match &self.permission_type {
            PermissionType::Bash { command, .. } => match categorize_bash_command(command) {
                BashCommandCategory::ReadingEtc => {
                    "Yes, allow reading from etc/ from this project".to_string()
                }
                BashCommandCategory::NamedCommand(name) => {
                    format!("Yes, allow {} commands from this project", name)
                }
                BashCommandCategory::Generic => {
                    "Yes, allow this command from this project".to_string()
                }
            },
            PermissionType::Edit { .. } | PermissionType::Write { .. } => {
                "Yes, allow all edits during this session (shift+tab)".to_string()
            }
        }
    }

    /// Get the question text based on permission type
    fn question_text(&self) -> String {
        match &self.permission_type {
            PermissionType::Bash { .. } => "Do you want to proceed?".to_string(),
            PermissionType::Edit { file_path, .. } => {
                format!("Do you want to make this edit to {}?", file_path)
            }
            PermissionType::Write { file_path, .. } => {
                format!("Do you want to create {}?", file_path)
            }
        }
    }

    /// Render the dialog to a string
    pub fn render(&self, width: usize) -> String {
        let mut lines = Vec::new();

        // Full-width separator
        lines.push(make_separator('─', width));

        // Title based on permission type
        match &self.permission_type {
            PermissionType::Bash { .. } => {
                lines.push(" Bash command".to_string());
            }
            PermissionType::Edit { file_path, .. } => {
                lines.push(format!(" Edit file {}", file_path));
            }
            PermissionType::Write { file_path, .. } => {
                lines.push(format!(" Create file {}", file_path));
            }
        }

        // Content section based on type
        match &self.permission_type {
            PermissionType::Bash {
                command,
                description,
            } => {
                lines.push(String::new());
                lines.push(format!("   {}", command));
                if let Some(desc) = description {
                    lines.push(format!("   {}", desc));
                }
                lines.push(String::new()); // Blank line before question
            }
            PermissionType::Edit { diff_lines, .. } => {
                lines.push(make_separator('╌', width));
                for line in diff_lines {
                    lines.push(render_diff_line(line));
                }
                lines.push(make_separator('╌', width));
            }
            PermissionType::Write { content_lines, .. } => {
                lines.push(make_separator('╌', width));
                for (i, content) in content_lines.iter().enumerate() {
                    lines.push(format!(" {:2} {}", i + 1, content));
                }
                lines.push(make_separator('╌', width));
            }
        }

        // Question
        lines.push(format!(" {}", self.question_text()));

        // Options
        let yes_indicator = if self.selected == PermissionSelection::Yes {
            " ❯ "
        } else {
            "   "
        };
        let session_indicator = if self.selected == PermissionSelection::YesSession {
            " ❯ "
        } else {
            "   "
        };
        let no_indicator = if self.selected == PermissionSelection::No {
            " ❯ "
        } else {
            "   "
        };

        lines.push(format!("{}1. Yes", yes_indicator));
        lines.push(format!("{}2. {}", session_indicator, self.option2_text()));
        lines.push(format!("{}3. No", no_indicator));

        // Footer
        lines.push(String::new());
        lines.push(" Esc to cancel · Tab to add additional instructions".to_string());

        lines.join("\n")
    }
}

/// Create a separator line of specified character and width
fn make_separator(ch: char, width: usize) -> String {
    ch.to_string().repeat(width)
}

/// Render a diff line with proper formatting
fn render_diff_line(line: &DiffLine) -> String {
    let prefix = match line.kind {
        DiffKind::Removed => "-",
        DiffKind::Added => "+",
        DiffKind::Context | DiffKind::NoNewline => " ",
    };

    match line.line_num {
        // Format: " {line_num} {prefix}{content}" - space, number, space, prefix, content
        Some(n) => format!(" {} {}{}", n, prefix, line.content),
        None => format!("    {}{}", prefix, line.content),
    }
}

#[cfg(test)]
#[path = "permission_tests.rs"]
mod tests;
