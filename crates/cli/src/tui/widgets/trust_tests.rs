// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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
