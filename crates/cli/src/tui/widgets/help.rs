// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Help dialog widget.
//!
//! Shown when user executes `/help` to display help and available commands.

#[cfg(test)]
#[path = "help_tests.rs"]
mod tests;

/// Available tabs in the help dialog
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HelpTab {
    #[default]
    General,
    Commands,
    CustomCommands,
}

impl HelpTab {
    /// Get all tabs in order
    pub fn all() -> &'static [HelpTab] {
        &[HelpTab::General, HelpTab::Commands, HelpTab::CustomCommands]
    }

    /// Get the next tab (wraps around)
    pub fn next(self) -> HelpTab {
        match self {
            HelpTab::General => HelpTab::Commands,
            HelpTab::Commands => HelpTab::CustomCommands,
            HelpTab::CustomCommands => HelpTab::General,
        }
    }

    /// Get the previous tab (wraps around)
    pub fn prev(self) -> HelpTab {
        match self {
            HelpTab::General => HelpTab::CustomCommands,
            HelpTab::Commands => HelpTab::General,
            HelpTab::CustomCommands => HelpTab::Commands,
        }
    }

    /// Get display name for the tab
    pub fn name(self) -> &'static str {
        match self {
            HelpTab::General => "general",
            HelpTab::Commands => "commands",
            HelpTab::CustomCommands => "custom-commands",
        }
    }
}

/// State for the /help dialog
#[derive(Clone, Debug)]
pub struct HelpDialog {
    /// Currently active tab
    pub active_tab: HelpTab,
    /// Selected command index in Commands tab (0-based)
    pub commands_selected: usize,
    /// Selected command index in CustomCommands tab (0-based)
    pub custom_selected: usize,
    /// Claude version string for display
    pub version: String,
}

impl Default for HelpDialog {
    fn default() -> Self {
        Self::new("2.1.12".to_string())
    }
}

impl HelpDialog {
    pub fn new(version: String) -> Self {
        Self {
            active_tab: HelpTab::General,
            commands_selected: 0,
            custom_selected: 0,
            version,
        }
    }

    /// Switch to next tab
    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
    }

    /// Switch to previous tab
    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
    }

    /// Move selection up in current command list (wraps at boundaries)
    pub fn select_prev(&mut self, total_commands: usize) {
        match self.active_tab {
            HelpTab::Commands => {
                if total_commands > 0 {
                    if self.commands_selected == 0 {
                        self.commands_selected = total_commands - 1;
                    } else {
                        self.commands_selected -= 1;
                    }
                }
            }
            HelpTab::CustomCommands => {
                // Similar logic for custom commands when implemented
            }
            HelpTab::General => {}
        }
    }

    /// Move selection down in current command list (wraps at boundaries)
    pub fn select_next(&mut self, total_commands: usize) {
        match self.active_tab {
            HelpTab::Commands => {
                if total_commands > 0 {
                    self.commands_selected = (self.commands_selected + 1) % total_commands;
                }
            }
            HelpTab::CustomCommands => {
                // Similar logic for custom commands when implemented
            }
            HelpTab::General => {}
        }
    }
}
