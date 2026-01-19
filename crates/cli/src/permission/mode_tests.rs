#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_default_mode() {
    assert_eq!(PermissionMode::default(), PermissionMode::Default);
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
