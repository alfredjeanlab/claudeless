#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
