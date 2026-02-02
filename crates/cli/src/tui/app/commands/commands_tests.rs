// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn resolves_short_aliases() {
    assert_eq!(resolve_model_id("haiku"), "claude-haiku-4-5-20251001");
    assert_eq!(resolve_model_id("sonnet"), "claude-sonnet-4-20250514");
    assert_eq!(resolve_model_id("opus"), "claude-opus-4-5-20251101");
}

#[test]
fn resolves_claude_prefixed_aliases() {
    assert_eq!(
        resolve_model_id("claude-haiku"),
        "claude-haiku-4-5-20251001"
    );
    assert_eq!(
        resolve_model_id("claude-sonnet"),
        "claude-sonnet-4-20250514"
    );
    assert_eq!(resolve_model_id("claude-opus"), "claude-opus-4-5-20251101");
}

#[test]
fn passes_through_full_ids() {
    assert_eq!(
        resolve_model_id("claude-haiku-4-5-20251001"),
        "claude-haiku-4-5-20251001"
    );
    assert_eq!(
        resolve_model_id("claude-opus-4-5-20251101"),
        "claude-opus-4-5-20251101"
    );
}

#[test]
fn case_insensitive() {
    assert_eq!(resolve_model_id("Haiku"), "claude-haiku-4-5-20251001");
    assert_eq!(resolve_model_id("OPUS"), "claude-opus-4-5-20251101");
}
