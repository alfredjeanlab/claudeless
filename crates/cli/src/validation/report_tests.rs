// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_new_report() {
    let report = AccuracyReport::new();
    assert!(report.items.is_empty());
}

#[test]
fn test_add_item() {
    let mut report = AccuracyReport::new();
    report.add_item(ValidationItem {
        name: "--print".to_string(),
        category: FeatureCategory::CliFlags,
        status: ValidationStatus::Match,
        notes: None,
    });

    assert_eq!(report.items.len(), 1);
}

#[test]
fn test_add_cli_flags() {
    let mut report = AccuracyReport::new();
    let audit = CliAudit::new();
    report.add_cli_flags(&audit);

    assert!(!report.items.is_empty());

    // All items should be CLI flags
    for item in &report.items {
        assert_eq!(item.category, FeatureCategory::CliFlags);
    }
}

#[test]
fn test_items_by_category() {
    let mut report = AccuracyReport::new();
    report.add_item(ValidationItem {
        name: "test1".to_string(),
        category: FeatureCategory::CliFlags,
        status: ValidationStatus::Match,
        notes: None,
    });
    report.add_item(ValidationItem {
        name: "test2".to_string(),
        category: FeatureCategory::OutputFormats,
        status: ValidationStatus::Match,
        notes: None,
    });

    let by_cat = report.items_by_category();
    assert_eq!(by_cat.len(), 2);
    assert!(by_cat.contains_key(&FeatureCategory::CliFlags));
    assert!(by_cat.contains_key(&FeatureCategory::OutputFormats));
}

#[test]
fn test_count_by_status() {
    let mut report = AccuracyReport::new();
    report.add_item(ValidationItem {
        name: "match1".to_string(),
        category: FeatureCategory::CliFlags,
        status: ValidationStatus::Match,
        notes: None,
    });
    report.add_item(ValidationItem {
        name: "match2".to_string(),
        category: FeatureCategory::CliFlags,
        status: ValidationStatus::Match,
        notes: None,
    });
    report.add_item(ValidationItem {
        name: "partial".to_string(),
        category: FeatureCategory::CliFlags,
        status: ValidationStatus::Partial("note".to_string()),
        notes: None,
    });

    let counts = report.count_by_status();
    assert_eq!(counts["match"], 2);
    assert_eq!(counts["partial"], 1);
}

#[test]
fn test_to_markdown() {
    let mut report = AccuracyReport::new()
        .with_date("2025-01-18")
        .with_claude_version("1.0.0");

    report.add_item(ValidationItem {
        name: "--print".to_string(),
        category: FeatureCategory::CliFlags,
        status: ValidationStatus::Match,
        notes: Some("Print response and exit".to_string()),
    });

    let md = report.to_markdown();
    assert!(md.contains("# Claudelessulator Accuracy Report"));
    assert!(md.contains("2025-01-18"));
    assert!(md.contains("1.0.0"));
    assert!(md.contains("--print"));
    assert!(md.contains("✅"));
}
