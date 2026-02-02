// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn model_choice_from_opus_model_id() {
    assert_eq!(
        ModelChoice::from_model_id("claude-opus-4-5-20251101"),
        ModelChoice::Opus
    );
}

#[test]
fn model_choice_from_sonnet_model_id() {
    assert_eq!(
        ModelChoice::from_model_id("claude-sonnet-4-20250514"),
        ModelChoice::Default
    );
}

#[test]
fn model_choice_from_haiku_model_id() {
    assert_eq!(
        ModelChoice::from_model_id("claude-haiku-4-5-20251101"),
        ModelChoice::Haiku
    );
}

#[test]
fn model_picker_navigation() {
    let mut dialog = ModelPickerDialog::new("claude-sonnet-4-20250514");
    assert_eq!(dialog.selected, ModelChoice::Default);

    dialog.move_down();
    assert_eq!(dialog.selected, ModelChoice::Opus);

    dialog.move_down();
    assert_eq!(dialog.selected, ModelChoice::Haiku);

    dialog.move_down(); // Wraps
    assert_eq!(dialog.selected, ModelChoice::Default);

    dialog.move_up(); // Wraps back
    assert_eq!(dialog.selected, ModelChoice::Haiku);
}
