// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Trust prompt dialog widget.
//!
//! Shown when starting in an untrusted directory to ask user if they
//! trust the files in the folder.
//!
//! Note: Rendering has been moved to app.rs as part of the iocraft migration.
//! This module keeps the types used by the app component.

/// User's choice in trust prompt
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrustChoice {
    Yes,
    No,
}

// Note: TrustPrompt state has been moved to TrustPromptState in app.rs.
// This type alias maintains backward compatibility.
pub type TrustPrompt = crate::tui::app::TrustPromptState;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::TrustPromptState;

    #[test]
    fn test_trust_prompt_creation() {
        let prompt = TrustPromptState::new("/path/to/dir".to_string());
        assert_eq!(prompt.working_directory, "/path/to/dir");
        assert_eq!(prompt.selected, TrustChoice::Yes);
    }

    #[test]
    fn test_trust_choice_toggle() {
        let mut prompt = TrustPromptState::new("/test".to_string());
        assert_eq!(prompt.selected, TrustChoice::Yes);
        prompt.selected = TrustChoice::No;
        assert_eq!(prompt.selected, TrustChoice::No);
    }
}
