#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_bypass_requires_allow() {
    let bypass = PermissionBypass::new(false, true);
    assert_eq!(bypass.validate(), BypassValidation::NotAllowed);
    assert!(!bypass.is_active());
    assert!(bypass.is_not_allowed());
}

#[test]
fn test_bypass_enabled_when_both_set() {
    let bypass = PermissionBypass::new(true, true);
    assert_eq!(bypass.validate(), BypassValidation::Enabled);
    assert!(bypass.is_active());
    assert!(!bypass.is_not_allowed());
}

#[test]
fn test_bypass_disabled_when_not_requested() {
    let bypass = PermissionBypass::new(true, false);
    assert_eq!(bypass.validate(), BypassValidation::Disabled);
    assert!(!bypass.is_active());
    assert!(!bypass.is_not_allowed());
}

#[test]
fn test_bypass_disabled_by_default() {
    let bypass = PermissionBypass::default();
    assert_eq!(bypass.validate(), BypassValidation::Disabled);
    assert!(!bypass.is_active());
}

#[test]
fn test_neither_flag_set() {
    let bypass = PermissionBypass::new(false, false);
    assert_eq!(bypass.validate(), BypassValidation::Disabled);
    assert!(!bypass.is_active());
    assert!(!bypass.is_not_allowed());
}

#[test]
fn test_error_message_content() {
    let msg = PermissionBypass::error_message();
    assert!(msg.contains("--dangerously-skip-permissions"));
    assert!(msg.contains("--allow-dangerously-skip-permissions"));
    assert!(msg.contains("sandboxed"));
}
