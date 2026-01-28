// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]

use super::*;

#[test]
fn spinner_frames_returns_expected_count() {
    let frames = spinner_frames();
    assert_eq!(frames.len(), 6);
}

#[test]
fn spinner_cycle_has_breathing_effect() {
    let cycle = spinner_cycle();
    // Forward (6) + reverse minus endpoints (4) = 10
    assert_eq!(cycle.len(), 10);
    // First and last should be the same (completing the cycle)
    assert_eq!(cycle[0], "·");
    assert_eq!(cycle[cycle.len() - 1], "✢");
}

#[test]
fn random_verb_returns_valid_verb() {
    let verb = random_verb();
    assert!(SPINNER_VERBS.contains(&verb));
}

#[test]
fn spinner_verbs_not_empty() {
    assert!(!SPINNER_VERBS.is_empty());
}
