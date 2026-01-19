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
mod tests {
    use super::*;

    #[test]
    fn test_word_list_sizes() {
        assert_eq!(ADJECTIVES.len(), 20);
        assert_eq!(VERBS.len(), 20);
        assert_eq!(NOUNS.len(), 20);
    }

    #[test]
    fn test_all_adjectives_lowercase() {
        for adj in ADJECTIVES {
            assert!(
                adj.chars().all(|c| c.is_ascii_lowercase()),
                "Adjective not lowercase: {}",
                adj
            );
        }
    }

    #[test]
    fn test_all_verbs_lowercase() {
        for verb in VERBS {
            assert!(
                verb.chars().all(|c| c.is_ascii_lowercase()),
                "Verb not lowercase: {}",
                verb
            );
        }
    }

    #[test]
    fn test_all_nouns_lowercase() {
        for noun in NOUNS {
            assert!(
                noun.chars().all(|c| c.is_ascii_lowercase()),
                "Noun not lowercase: {}",
                noun
            );
        }
    }

    #[test]
    fn test_generate_plan_name_format() {
        let name = generate_plan_name();
        let parts: Vec<&str> = name.split('-').collect();
        assert_eq!(parts.len(), 3, "Name should have 3 parts: {}", name);

        // Verify each part is lowercase
        for part in &parts {
            assert!(
                part.chars().all(|c| c.is_ascii_lowercase()),
                "Part not lowercase: {}",
                part
            );
        }
    }

    #[test]
    fn test_generate_plan_name_uses_valid_words() {
        let name = generate_plan_name();
        let parts: Vec<&str> = name.split('-').collect();

        assert!(
            ADJECTIVES.contains(&parts[0]),
            "First part should be adjective: {}",
            parts[0]
        );
        assert!(
            VERBS.contains(&parts[1]),
            "Second part should be verb: {}",
            parts[1]
        );
        assert!(
            NOUNS.contains(&parts[2]),
            "Third part should be noun: {}",
            parts[2]
        );
    }
}
