// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Spinner animation for the TUI.
//!
//! Provides animated spinner frames and whimsical verb options
//! matching Claude Code's "Thinking..." animation behavior.

/// Spinner animation frames (platform-aware)
pub fn spinner_frames() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["·", "✢", "✳", "✶", "✻", "✽"]
    } else {
        &["·", "✢", "*", "✶", "✻", "✽"]
    }
}

/// Full animation cycle (forward + reverse for breathing effect)
pub fn spinner_cycle() -> Vec<&'static str> {
    let frames = spinner_frames();
    let mut cycle: Vec<&str> = frames.to_vec();
    // Add reverse (skip first and last to avoid duplicates at endpoints)
    // Forward: [·, ✢, ✳, ✶, ✻, ✽] (6 frames)
    // Reverse: [✻, ✶, ✳, ✢] (4 frames, skip both endpoints)
    cycle.extend(frames.iter().rev().skip(1).take(frames.len() - 2));
    cycle
}

/// Whimsical verbs for status messages
pub const SPINNER_VERBS: &[&str] = &[
    "Thinking",
    "Computing",
    "Pondering",
    "Processing",
    "Contemplating",
    "Cogitating",
    "Deliberating",
    "Musing",
];

/// Get a random spinner verb
pub fn random_verb() -> &'static str {
    let idx = fastrand::usize(..SPINNER_VERBS.len());
    SPINNER_VERBS[idx]
}

#[cfg(test)]
#[path = "spinner_tests.rs"]
mod tests;
