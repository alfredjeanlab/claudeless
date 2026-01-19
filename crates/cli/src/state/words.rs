// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Word lists for plan naming (adjective-verb-noun format).

/// Adjectives for plan name generation.
pub const ADJECTIVES: &[&str] = &[
    "velvety", "swirling", "gleaming", "dancing", "quiet", "bright", "ancient", "swift", "gentle",
    "bold", "frozen", "golden", "hollow", "eager", "secret", "distant", "misty", "tender", "wild",
    "calm",
];

/// Verbs (gerund form) for plan name generation.
pub const VERBS: &[&str] = &[
    "crunching",
    "gliding",
    "spinning",
    "weaving",
    "drifting",
    "singing",
    "flowing",
    "growing",
    "building",
    "seeking",
    "watching",
    "waiting",
    "running",
    "falling",
    "rising",
    "turning",
    "crossing",
    "finding",
    "making",
    "taking",
];

/// Nouns for plan name generation.
pub const NOUNS: &[&str] = &[
    "ocean", "forest", "mountain", "river", "meadow", "valley", "island", "canyon", "desert",
    "glacier", "thunder", "shadow", "crystal", "ember", "garden", "harbor", "beacon", "bridge",
    "tunnel", "tower",
];

/// Generate a random plan name in the format `{adjective}-{verb}-{noun}`.
///
/// # Examples
///
/// ```
/// use claudeless::state::words::generate_plan_name;
///
/// let name = generate_plan_name();
/// let parts: Vec<&str> = name.split('-').collect();
/// assert_eq!(parts.len(), 3);
/// ```
pub fn generate_plan_name() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    // Use time-based randomness (simple approach without adding rand dependency)
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let mut hasher = DefaultHasher::new();
    nanos.hash(&mut hasher);
    let hash = hasher.finish();

    // Use different bits of the hash for each word
    let adj_idx = (hash as usize) % ADJECTIVES.len();
    let verb_idx = ((hash >> 16) as usize) % VERBS.len();
    let noun_idx = ((hash >> 32) as usize) % NOUNS.len();

    format!(
        "{}-{}-{}",
        ADJECTIVES[adj_idx], VERBS[verb_idx], NOUNS[noun_idx]
    )
}

#[cfg(test)]
#[path = "words_tests.rs"]
mod tests;
