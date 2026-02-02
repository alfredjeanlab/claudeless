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
    /// Read-only command accessing a specific path directory (ls, cat, head, etc.)
    ReadingPath(String),
    /// Command accesses a specific path directory
    PathAccess(String),
    /// Named command (npm, cargo, git, rm, etc.)
    NamedCommand(String),
    /// Fallback for complex or unrecognized commands
    Generic,
}

/// Commands that are read-only (used for "allow reading from" vs "allow access to")
const READ_ONLY_COMMANDS: &[&str] = &[
    "ls", "cat", "head", "tail", "less", "more", "wc", "file", "stat", "find", "grep", "egrep",
    "fgrep", "rg", "ag", "fd", "tree", "du", "df", "readlink", "realpath", "diff",
];

/// Categorize a bash command for permission text generation.
///
/// Priority:
/// 1. If command contains `/etc/` path, categorize as ReadingEtc
/// 2. If command has path arguments, categorize as PathAccess or ReadingPath
/// 3. Extract first word as the command name
/// 4. Fallback to Generic for empty or unrecognizable commands
fn categorize_bash_command(command: &str) -> BashCommandCategory {
    // Check for /etc/ access first (highest priority)
    if command.contains("/etc/") {
        return BashCommandCategory::ReadingEtc;
    }

    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return BashCommandCategory::Generic;
    }

    // Extract command name for read-only check
    let first_word = parts[0];
    let command_name = first_word.rsplit('/').next().unwrap_or(first_word);
    let is_read_only = READ_ONLY_COMMANDS.contains(&command_name);

    // Check if any argument is a path — use directory-based categorization
    for part in parts.iter().skip(1) {
        if part.starts_with('/') || part.starts_with("~/") {
            // Extract the first directory component from the path
            let path = part.trim_start_matches('/').trim_start_matches("~/");
            if let Some(dir) = path.split('/').next() {
                if !dir.is_empty() {
                    let dir_with_slash = format!("{}/", dir);
                    return if is_read_only {
                        BashCommandCategory::ReadingPath(dir_with_slash)
                    } else {
                        BashCommandCategory::PathAccess(dir_with_slash)
                    };
                }
            }
        }
    }

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

/// Key for identifying session-level permission grants
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SessionPermissionKey {
    /// Bash command matching by prefix (e.g., "cat /etc/" grants all "cat /etc/*")
    BashPrefix(String),
    /// Edit permission for all files (session-level edit grants apply to all edits)
    EditAll,
    /// Write permission for all files (session-level write grants apply to all writes)
    WriteAll,
}

/// Extract a prefix from a bash command for permission matching
///
/// For commands with path arguments, extracts the directory portion.
/// For other commands, returns just the command name.
///
/// Examples:
/// - "cat /etc/passwd" -> "cat /etc/"
/// - "npm test" -> "npm"
/// - "rm -rf /tmp/foo" -> "rm"
pub fn extract_bash_prefix(command: &str) -> String {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return command.to_string();
    }

    // If second part looks like a path, include the directory
    if parts.len() > 1 && parts[1].starts_with('/') {
        if let Some(dir_end) = parts[1].rfind('/') {
            return format!("{} {}", parts[0], &parts[1][..=dir_end]);
        }
    }

    parts[0].to_string()
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

    /// Extract the session permission key for this dialog
    pub fn session_key(&self) -> SessionPermissionKey {
        match &self.permission_type {
            PermissionType::Bash { command, .. } => {
                SessionPermissionKey::BashPrefix(extract_bash_prefix(command))
            }
            PermissionType::Edit { .. } => SessionPermissionKey::EditAll,
            PermissionType::Write { .. } => SessionPermissionKey::WriteAll,
        }
    }

    /// Get the option 2 text based on permission type
    fn option2_text(&self) -> String {
        match &self.permission_type {
            PermissionType::Bash { command, .. } => match categorize_bash_command(command) {
                BashCommandCategory::ReadingEtc => {
                    "Yes, allow reading from etc/ from this project".to_string()
                }
                BashCommandCategory::ReadingPath(dir) => {
                    format!("Yes, allow reading from {} from this project", dir)
                }
                BashCommandCategory::PathAccess(dir) => {
                    format!("Yes, and always allow access to {} from this project", dir)
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
                lines.push(" Edit file".to_string());
                lines.push(format!(" {}", file_path));
            }
            PermissionType::Write { file_path, .. } => {
                lines.push(" Create file".to_string());
                lines.push(format!(" {}", file_path));
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
        lines.push(" Esc to cancel \u{00b7} Tab to amend".to_string());

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
        DiffKind::Context => " ",
        DiffKind::NoNewline => "  ",
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
