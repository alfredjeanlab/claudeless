// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Slash command registry and filtering for the autocomplete menu.
//!
//! When the user types `/` in the input, a command menu appears showing all
//! available commands. As additional characters are typed, the menu filters
//! using fuzzy subsequence matching.

#[cfg(test)]
#[path = "slash_menu_tests.rs"]
mod tests;

/// A slash command definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashCommand {
    /// Command name without the leading `/` (e.g., "add-dir")
    pub name: &'static str,
    /// Human-readable description
    pub description: &'static str,
    /// Optional argument hint (e.g., "<path>")
    pub argument_hint: Option<&'static str>,
}

impl SlashCommand {
    /// Full command with leading slash.
    pub fn full_name(&self) -> String {
        format!("/{}", self.name)
    }
}

/// All available slash commands, in alphabetical order.
///
/// This list matches Claude Code v2.1.12's command set.
pub static COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "add-dir",
        description: "Add a new working directory",
        argument_hint: Some("<path>"),
    },
    SlashCommand {
        name: "agents",
        description: "Manage agent configurations",
        argument_hint: None,
    },
    SlashCommand {
        name: "bug",
        description: "Report a bug or issue",
        argument_hint: None,
    },
    SlashCommand {
        name: "clear",
        description: "Clear conversation history",
        argument_hint: None,
    },
    SlashCommand {
        name: "compact",
        description: "Compact conversation (keep a summary in context)",
        argument_hint: None,
    },
    SlashCommand {
        name: "config",
        description: "Open configuration settings",
        argument_hint: None,
    },
    SlashCommand {
        name: "context",
        description: "View current context usage",
        argument_hint: None,
    },
    SlashCommand {
        name: "cost",
        description: "Show session cost summary",
        argument_hint: None,
    },
    SlashCommand {
        name: "doctor",
        description: "Run diagnostics and check system health",
        argument_hint: None,
    },
    SlashCommand {
        name: "fork",
        description: "Create a fork of the current conversation at this point",
        argument_hint: None,
    },
    SlashCommand {
        name: "help",
        description: "Show help and available commands",
        argument_hint: None,
    },
    SlashCommand {
        name: "hooks",
        description: "Manage hook configurations for tool events",
        argument_hint: None,
    },
    SlashCommand {
        name: "init",
        description: "Initialize a new project or configuration",
        argument_hint: None,
    },
    SlashCommand {
        name: "login",
        description: "Log in to your account",
        argument_hint: None,
    },
    SlashCommand {
        name: "logout",
        description: "Log out of your account",
        argument_hint: None,
    },
    SlashCommand {
        name: "mcp",
        description: "Manage MCP server connections",
        argument_hint: None,
    },
    SlashCommand {
        name: "memory",
        description: "View or manage conversation memory",
        argument_hint: None,
    },
    SlashCommand {
        name: "model",
        description: "Switch the active model",
        argument_hint: Some("<model>"),
    },
    SlashCommand {
        name: "permissions",
        description: "View or manage permissions",
        argument_hint: None,
    },
    SlashCommand {
        name: "pr-comments",
        description: "View pull request comments",
        argument_hint: None,
    },
    SlashCommand {
        name: "review",
        description: "Review code changes",
        argument_hint: None,
    },
    SlashCommand {
        name: "status",
        description: "Show current session status",
        argument_hint: None,
    },
    SlashCommand {
        name: "tasks",
        description: "List and manage background tasks",
        argument_hint: None,
    },
    SlashCommand {
        name: "terminal-setup",
        description: "Configure terminal settings",
        argument_hint: None,
    },
    SlashCommand {
        name: "todos",
        description: "Show the current todo list",
        argument_hint: None,
    },
    SlashCommand {
        name: "vim",
        description: "Toggle vim keybindings mode",
        argument_hint: None,
    },
];

/// Check if `query` matches `text` using fuzzy subsequence matching.
///
/// Returns true if all characters in query appear in text in order,
/// but not necessarily consecutively.
///
/// # Examples
///
/// ```
/// use claudeless::tui::slash_menu::fuzzy_matches;
///
/// assert!(fuzzy_matches("co", "compact"));  // c_o_mpact
/// assert!(fuzzy_matches("hk", "hooks"));    // h_oo_k_s
/// assert!(!fuzzy_matches("xyz", "compact"));
/// ```
pub fn fuzzy_matches(query: &str, text: &str) -> bool {
    let query = query.to_lowercase();
    let text = text.to_lowercase();

    let mut query_chars = query.chars().peekable();

    for text_char in text.chars() {
        if let Some(&query_char) = query_chars.peek() {
            if text_char == query_char {
                query_chars.next();
            }
        }
    }

    query_chars.peek().is_none()
}

/// Filter commands by a query string (without leading `/`).
///
/// Returns commands where the name fuzzy-matches the query,
/// preserving alphabetical order.
pub fn filter_commands(query: &str) -> Vec<&'static SlashCommand> {
    COMMANDS
        .iter()
        .filter(|cmd| fuzzy_matches(query, cmd.name))
        .collect()
}

/// State of the slash command autocomplete menu.
#[derive(Clone, Debug)]
pub struct SlashMenuState {
    /// Characters typed after `/` (the filter query)
    pub filter: String,
    /// Index of the currently selected command in the filtered list
    pub selected_index: usize,
    /// Cached filtered commands (updated when filter changes)
    pub filtered_commands: Vec<&'static SlashCommand>,
}

impl Default for SlashMenuState {
    fn default() -> Self {
        Self::new()
    }
}

impl SlashMenuState {
    /// Create a new menu state showing all commands.
    pub fn new() -> Self {
        Self {
            filter: String::new(),
            selected_index: 0,
            filtered_commands: filter_commands(""),
        }
    }

    /// Update the filter and refresh the command list.
    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter.clone();
        self.filtered_commands = filter_commands(&filter);
        // Reset selection if it's out of bounds
        if self.selected_index >= self.filtered_commands.len() {
            self.selected_index = 0;
        }
    }

    /// Move selection down (wraps at end).
    pub fn select_next(&mut self) {
        if !self.filtered_commands.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_commands.len();
        }
    }

    /// Move selection up (wraps at beginning).
    pub fn select_prev(&mut self) {
        if !self.filtered_commands.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.filtered_commands.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Get the currently selected command.
    pub fn selected_command(&self) -> Option<&'static SlashCommand> {
        self.filtered_commands.get(self.selected_index).copied()
    }
}
