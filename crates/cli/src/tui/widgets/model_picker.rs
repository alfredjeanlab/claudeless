// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Model picker dialog widget.
//!
//! Shown when user presses Meta+P to switch between Claude models.

/// Available model choices
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelChoice {
    Default, // Sonnet 4.5 (recommended)
    Opus,    // Opus 4.5
    Haiku,   // Haiku 4.5
}

impl ModelChoice {
    /// Returns the full model ID string
    pub fn model_id(&self) -> &'static str {
        match self {
            ModelChoice::Default => "claude-sonnet-4-20250514",
            ModelChoice::Opus => "claude-opus-4-5-20251101",
            ModelChoice::Haiku => "claude-haiku-4-5-20251101",
        }
    }

    /// Returns the display name for the model
    pub fn display_name(&self) -> &'static str {
        match self {
            ModelChoice::Default => "Sonnet 4.5",
            ModelChoice::Opus => "Opus 4.5",
            ModelChoice::Haiku => "Haiku 4.5",
        }
    }

    /// Returns the description for the picker
    pub fn description(&self) -> &'static str {
        match self {
            ModelChoice::Default => "Best for everyday tasks",
            ModelChoice::Opus => "Most capable for complex work",
            ModelChoice::Haiku => "Fastest for quick answers",
        }
    }

    /// Returns all choices in display order
    pub fn all() -> [ModelChoice; 3] {
        [ModelChoice::Default, ModelChoice::Opus, ModelChoice::Haiku]
    }

    /// Convert from model ID string
    pub fn from_model_id(id: &str) -> Self {
        let lower = id.to_lowercase();
        if lower.contains("haiku") {
            ModelChoice::Haiku
        } else if lower.contains("opus") {
            ModelChoice::Opus
        } else {
            ModelChoice::Default
        }
    }
}

/// Model picker dialog state
#[derive(Clone, Debug)]
pub struct ModelPickerDialog {
    /// Currently highlighted option (cursor position)
    pub selected: ModelChoice,
    /// Currently active model (shows checkmark)
    pub current: ModelChoice,
}

impl ModelPickerDialog {
    pub fn new(current_model: &str) -> Self {
        let current = ModelChoice::from_model_id(current_model);
        Self {
            selected: current.clone(),
            current,
        }
    }

    /// Move selection up (wraps around)
    pub fn move_up(&mut self) {
        self.selected = match self.selected {
            ModelChoice::Default => ModelChoice::Haiku,
            ModelChoice::Opus => ModelChoice::Default,
            ModelChoice::Haiku => ModelChoice::Opus,
        };
    }

    /// Move selection down (wraps around)
    pub fn move_down(&mut self) {
        self.selected = match self.selected {
            ModelChoice::Default => ModelChoice::Opus,
            ModelChoice::Opus => ModelChoice::Haiku,
            ModelChoice::Haiku => ModelChoice::Default,
        };
    }
}

#[cfg(test)]
#[path = "model_picker_tests.rs"]
mod tests;
