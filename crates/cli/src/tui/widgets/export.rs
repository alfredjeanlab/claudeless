// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Export dialog widget.
//!
//! Shown when user executes `/export` to export the conversation.

#[cfg(test)]
#[path = "export_tests.rs"]
mod tests;

/// Export method options
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ExportMethod {
    #[default]
    Clipboard,
    File,
}

/// Current step in the export workflow
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ExportStep {
    /// Selecting export method (clipboard or file)
    #[default]
    MethodSelection,
    /// Entering filename for file export
    FilenameInput,
}

/// State for the /export dialog
#[derive(Clone, Debug)]
pub struct ExportDialog {
    /// Current step in the workflow
    pub step: ExportStep,
    /// Selected export method
    pub selected_method: ExportMethod,
    /// Filename input buffer (for file export)
    pub filename: String,
    /// Default filename (generated on dialog open)
    pub default_filename: String,
}

impl Default for ExportDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportDialog {
    pub fn new() -> Self {
        // Generate default filename with timestamp
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let default_filename = format!("conversation_{}.txt", timestamp);

        Self {
            step: ExportStep::MethodSelection,
            selected_method: ExportMethod::Clipboard,
            filename: default_filename.clone(),
            default_filename,
        }
    }

    /// Toggle between clipboard and file methods
    pub fn toggle_method(&mut self) {
        self.selected_method = match self.selected_method {
            ExportMethod::Clipboard => ExportMethod::File,
            ExportMethod::File => ExportMethod::Clipboard,
        };
    }

    /// Move selection up (wraps to bottom)
    pub fn move_selection_up(&mut self) {
        self.toggle_method();
    }

    /// Move selection down (wraps to top)
    pub fn move_selection_down(&mut self) {
        self.toggle_method();
    }

    /// Confirm current selection and advance workflow
    pub fn confirm_selection(&mut self) -> bool {
        match self.step {
            ExportStep::MethodSelection => {
                if self.selected_method == ExportMethod::File {
                    self.step = ExportStep::FilenameInput;
                    false // Not done yet
                } else {
                    true // Clipboard selected, ready to export
                }
            }
            ExportStep::FilenameInput => true, // Ready to save file
        }
    }

    /// Go back from filename input to method selection
    pub fn go_back(&mut self) -> bool {
        match self.step {
            ExportStep::FilenameInput => {
                self.step = ExportStep::MethodSelection;
                false // Stay in dialog
            }
            ExportStep::MethodSelection => true, // Cancel dialog
        }
    }

    /// Handle character input for filename
    pub fn push_char(&mut self, c: char) {
        if self.step == ExportStep::FilenameInput {
            self.filename.push(c);
        }
    }

    /// Handle backspace for filename
    pub fn pop_char(&mut self) {
        if self.step == ExportStep::FilenameInput {
            self.filename.pop();
        }
    }
}
