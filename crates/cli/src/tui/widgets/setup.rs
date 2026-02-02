// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Setup wizard dialog widget.
//!
//! Shown on first run when `logged_in = false` to guide the user through
//! theme selection and login method choice.

/// Which step of the setup wizard we're on
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetupStep {
    ThemeSelection,
    LoginMethod,
}

/// Theme options (6 choices matching real Claude)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemeChoice {
    Dark,
    Light,
    DarkColorblind,
    LightColorblind,
    DarkAnsi,
    LightAnsi,
}

impl ThemeChoice {
    /// Display name for the theme option.
    pub fn display_name(&self) -> &'static str {
        match self {
            ThemeChoice::Dark => "Dark mode",
            ThemeChoice::Light => "Light mode",
            ThemeChoice::DarkColorblind => "Dark mode (colorblind-friendly)",
            ThemeChoice::LightColorblind => "Light mode (colorblind-friendly)",
            ThemeChoice::DarkAnsi => "Dark mode (ANSI colors only)",
            ThemeChoice::LightAnsi => "Light mode (ANSI colors only)",
        }
    }

    /// Syntax theme name for the preview area.
    pub fn syntax_theme_name(&self) -> &'static str {
        match self {
            ThemeChoice::Dark => "Monokai Extended",
            ThemeChoice::Light => "Monokai Extended",
            ThemeChoice::DarkColorblind => "Monokai Extended",
            ThemeChoice::LightColorblind => "Monokai Extended",
            ThemeChoice::DarkAnsi => "ansi",
            ThemeChoice::LightAnsi => "ansi",
        }
    }

    /// Get theme choice from index (0-5).
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => ThemeChoice::Dark,
            1 => ThemeChoice::Light,
            2 => ThemeChoice::DarkColorblind,
            3 => ThemeChoice::LightColorblind,
            4 => ThemeChoice::DarkAnsi,
            5 => ThemeChoice::LightAnsi,
            _ => ThemeChoice::Dark,
        }
    }

    /// Total number of theme options.
    pub const COUNT: usize = 6;
}

/// Login method options (3 choices)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoginChoice {
    ClaudeSubscription,
    AnthropicConsole,
    ThirdParty,
}

impl LoginChoice {
    /// Total number of login options.
    pub const COUNT: usize = 3;
}

/// Full setup wizard state
#[derive(Clone, Debug)]
pub struct SetupState {
    pub step: SetupStep,
    pub selected_theme: usize,
    pub selected_login: usize,
    pub syntax_highlighting: bool,
    pub claude_version: String,
}

impl SetupState {
    /// Create a new setup state with defaults.
    pub fn new(claude_version: String) -> Self {
        Self {
            step: SetupStep::ThemeSelection,
            selected_theme: 0,
            selected_login: 0,
            syntax_highlighting: true,
            claude_version,
        }
    }

    /// Get the currently selected theme choice.
    pub fn theme_choice(&self) -> ThemeChoice {
        ThemeChoice::from_index(self.selected_theme)
    }

    /// Move theme selection up (wrapping).
    pub fn theme_up(&mut self) {
        if self.selected_theme == 0 {
            self.selected_theme = ThemeChoice::COUNT - 1;
        } else {
            self.selected_theme -= 1;
        }
    }

    /// Move theme selection down (wrapping).
    pub fn theme_down(&mut self) {
        self.selected_theme = (self.selected_theme + 1) % ThemeChoice::COUNT;
    }

    /// Move login selection up (wrapping).
    pub fn login_up(&mut self) {
        if self.selected_login == 0 {
            self.selected_login = LoginChoice::COUNT - 1;
        } else {
            self.selected_login -= 1;
        }
    }

    /// Move login selection down (wrapping).
    pub fn login_down(&mut self) {
        self.selected_login = (self.selected_login + 1) % LoginChoice::COUNT;
    }

    /// Advance from theme selection to login method.
    pub fn advance_to_login(&mut self) {
        self.step = SetupStep::LoginMethod;
    }
}

/// ASCII art banner for the setup wizard (12 lines, fixed content).
pub const SETUP_ART: &[&str] = &[
    "     *                                       █████▓▓░",
    "                                 *         ███▓░     ░░",
    "            ░░░░░░                        ███▓░",
    "    ░░░   ░░░░░░░░░░                      ███▓░",
    "   ░░░░░░░░░░░░░░░░░░░    *                ██▓░░      ▓",
    "                                             ░▓▓███▓▓░",
    " *                                 ░░░░",
    "                                 ░░░░░░░░",
    "                               ░░░░░░░░░░░░░░░░",
    "                                                      *",
    "      ▗ ▗     ▖ ▖                       *",
    "                      *",
];

/// Full ellipsis separator (fixed 58 chars, used at top of setup screen).
pub const SETUP_SEPARATOR: &str = "…………………………………………………………………………………………………………………………………………………………";

/// Split ellipsis separator (fixed 58 chars, used between art and content).
pub const SETUP_SPLIT_SEPARATOR: &str =
    "…………………         ………………………………………………………………………………………………………………";

/// Theme option labels (index-aligned with ThemeChoice).
pub const THEME_LABELS: &[&str] = &[
    "Dark mode",
    "Light mode",
    "Dark mode (colorblind-friendly)",
    "Light mode (colorblind-friendly)",
    "Dark mode (ANSI colors only)",
    "Light mode (ANSI colors only)",
];

/// Syntax preview diff block (fixed content).
pub const SYNTAX_PREVIEW: &[&str] = &[
    " 1  function greet() {",
    " 2 -  console.log(\"Hello, World!\");",
    " 2 +  console.log(\"Hello, Claude!\");",
    " 3  }",
];

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
