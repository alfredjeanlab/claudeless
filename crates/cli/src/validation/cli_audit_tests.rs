// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn test_audit_has_core_flags() {
    let audit = CliAudit::new();
    assert!(audit.get("print").is_some());
    assert!(audit.get("model").is_some());
    assert!(audit.get("output-format").is_some());
}

#[test]
fn test_flags_with_status() {
    let audit = CliAudit::new();
    let implemented = audit.flags_with_status(&FlagStatus::Implemented);
    assert!(!implemented.is_empty());

    // Verify all returned flags are actually implemented
    for flag in implemented {
        assert!(matches!(flag.status, FlagStatus::Implemented));
    }
}

#[test]
fn test_count_by_status() {
    let audit = CliAudit::new();
    let counts = audit.count_by_status();

    assert!(counts["implemented"] > 0);
    // Total should equal number of flags
    let total: usize = counts.values().sum();
    assert_eq!(total, audit.flags.len());
}

#[test]
fn test_to_markdown() {
    let audit = CliAudit::new();
    let md = audit.to_markdown();

    assert!(md.contains("# CLI Flag Audit"));
    assert!(md.contains("## Implemented"));
    assert!(md.contains("--print"));
    assert!(md.contains("--model"));
}

#[test]
fn test_no_missing_needed_flags() {
    // This test ensures all needed flags are implemented
    let audit = CliAudit::new();
    let missing = audit.flags_with_status(&FlagStatus::MissingNeeded);

    assert!(
        missing.is_empty(),
        "Missing needed flags: {:?}",
        missing.iter().map(|f| f.name).collect::<Vec<_>>()
    );
}
