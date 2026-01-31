#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn from_str_user() {
    assert_eq!(
        "user".parse::<SettingSource>().unwrap(),
        SettingSource::User
    );
    assert_eq!(
        "User".parse::<SettingSource>().unwrap(),
        SettingSource::User
    );
    assert_eq!(
        "USER".parse::<SettingSource>().unwrap(),
        SettingSource::User
    );
}

#[test]
fn from_str_global_alias() {
    assert_eq!(
        "global".parse::<SettingSource>().unwrap(),
        SettingSource::User
    );
    assert_eq!(
        "Global".parse::<SettingSource>().unwrap(),
        SettingSource::User
    );
}

#[test]
fn from_str_project() {
    assert_eq!(
        "project".parse::<SettingSource>().unwrap(),
        SettingSource::Project
    );
    assert_eq!(
        "Project".parse::<SettingSource>().unwrap(),
        SettingSource::Project
    );
}

#[test]
fn from_str_local() {
    assert_eq!(
        "local".parse::<SettingSource>().unwrap(),
        SettingSource::Local
    );
    assert_eq!(
        "Local".parse::<SettingSource>().unwrap(),
        SettingSource::Local
    );
}

#[test]
fn from_str_unknown() {
    let err = "unknown".parse::<SettingSource>().unwrap_err();
    assert!(err.contains("unknown setting source"));
}

#[test]
fn all_returns_precedence_order() {
    let all = SettingSource::all();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0], SettingSource::User);
    assert_eq!(all[1], SettingSource::Project);
    assert_eq!(all[2], SettingSource::Local);
}
