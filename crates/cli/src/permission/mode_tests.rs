// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn test_default_mode() {
    assert_eq!(PermissionMode::default(), PermissionMode::Default);
}

// =========================================================================
// cycle_next Tests
// =========================================================================

#[test]
fn test_cycle_next_without_bypass() {
    // Without bypass: Default → AcceptEdits → Plan → Default
    assert_eq!(
        PermissionMode::Default.cycle_next(false),
        PermissionMode::AcceptEdits
    );
    assert_eq!(
        PermissionMode::AcceptEdits.cycle_next(false),
        PermissionMode::Plan
    );
    assert_eq!(
        PermissionMode::Plan.cycle_next(false),
        PermissionMode::Default
    );
}

#[test]
fn test_cycle_next_with_bypass() {
    // With bypass: Default → AcceptEdits → Plan → BypassPermissions → Default
    assert_eq!(
        PermissionMode::Default.cycle_next(true),
        PermissionMode::AcceptEdits
    );
    assert_eq!(
        PermissionMode::AcceptEdits.cycle_next(true),
        PermissionMode::Plan
    );
    assert_eq!(
        PermissionMode::Plan.cycle_next(true),
        PermissionMode::BypassPermissions
    );
    assert_eq!(
        PermissionMode::BypassPermissions.cycle_next(true),
        PermissionMode::Default
    );
}

#[test]
fn test_cycle_next_from_delegate_resets_to_default() {
    // Delegate is not in the normal cycle, goes back to Default
    assert_eq!(
        PermissionMode::Delegate.cycle_next(false),
        PermissionMode::Default
    );
    assert_eq!(
        PermissionMode::Delegate.cycle_next(true),
        PermissionMode::Default
    );
}

#[test]
fn test_cycle_next_from_dont_ask_resets_to_default() {
    // DontAsk is not in the normal cycle, goes back to Default
    assert_eq!(
        PermissionMode::DontAsk.cycle_next(false),
        PermissionMode::Default
    );
    assert_eq!(
        PermissionMode::DontAsk.cycle_next(true),
        PermissionMode::Default
    );
}

#[test]
fn test_cycle_next_bypass_permissions_always_goes_to_default() {
    // BypassPermissions always cycles to Default, regardless of allow_bypass
    assert_eq!(
        PermissionMode::BypassPermissions.cycle_next(false),
        PermissionMode::Default
    );
    assert_eq!(
        PermissionMode::BypassPermissions.cycle_next(true),
        PermissionMode::Default
    );
}

#[test]
fn test_allows_all() {
    assert!(PermissionMode::BypassPermissions.allows_all());
    assert!(!PermissionMode::Default.allows_all());
    assert!(!PermissionMode::AcceptEdits.allows_all());
}

#[test]
fn test_denies_all() {
    assert!(PermissionMode::DontAsk.denies_all());
    assert!(PermissionMode::Plan.denies_all());
    assert!(!PermissionMode::Default.denies_all());
}

#[test]
fn test_accepts_edits() {
    assert!(PermissionMode::AcceptEdits.accepts_edits());
    assert!(PermissionMode::BypassPermissions.accepts_edits());
    assert!(!PermissionMode::Default.accepts_edits());
    assert!(!PermissionMode::DontAsk.accepts_edits());
}

#[test]
fn test_value_enum_parsing() {
    // Test that all variants can be parsed from their kebab-case names
    assert_eq!(
        PermissionMode::from_str("accept-edits", true).unwrap(),
        PermissionMode::AcceptEdits
    );
    assert_eq!(
        PermissionMode::from_str("bypass-permissions", true).unwrap(),
        PermissionMode::BypassPermissions
    );
    assert_eq!(
        PermissionMode::from_str("default", true).unwrap(),
        PermissionMode::Default
    );
    assert_eq!(
        PermissionMode::from_str("delegate", true).unwrap(),
        PermissionMode::Delegate
    );
    assert_eq!(
        PermissionMode::from_str("dont-ask", true).unwrap(),
        PermissionMode::DontAsk
    );
    assert_eq!(
        PermissionMode::from_str("plan", true).unwrap(),
        PermissionMode::Plan
    );
}
