use super::*;
use serde_json::json;

#[test]
fn extract_file_path_prefers_file_path() {
    let input = json!({
        "file_path": "/foo/bar.txt",
        "path": "/other/path.txt"
    });
    assert_eq!(extract_file_path(&input), Some("/foo/bar.txt"));
}

#[test]
fn extract_file_path_falls_back_to_path() {
    let input = json!({
        "path": "/fallback/path.txt"
    });
    assert_eq!(extract_file_path(&input), Some("/fallback/path.txt"));
}

#[test]
fn extract_file_path_returns_none_when_missing() {
    let input = json!({
        "other": "value"
    });
    assert_eq!(extract_file_path(&input), None);
}

#[test]
fn extract_directory_prefers_path() {
    let input = json!({
        "path": "/foo",
        "directory": "/bar"
    });
    assert_eq!(extract_directory(&input), Some("/foo"));
}

#[test]
fn extract_directory_falls_back_to_directory() {
    let input = json!({
        "directory": "/fallback"
    });
    assert_eq!(extract_directory(&input), Some("/fallback"));
}

#[test]
fn extract_directory_returns_none_when_missing() {
    let input = json!({});
    assert_eq!(extract_directory(&input), None);
}

#[test]
fn extract_str_works() {
    let input = json!({
        "foo": "bar",
        "baz": 123
    });
    assert_eq!(extract_str(&input, "foo"), Some("bar"));
    assert_eq!(extract_str(&input, "baz"), None); // not a string
    assert_eq!(extract_str(&input, "missing"), None);
}

#[test]
fn extract_bool_returns_value_when_present() {
    let input = json!({
        "enabled": true,
        "disabled": false
    });
    assert!(extract_bool(&input, "enabled", false));
    assert!(!extract_bool(&input, "disabled", true));
}

#[test]
fn extract_bool_returns_default_when_missing() {
    let input = json!({});
    assert!(extract_bool(&input, "missing", true));
    assert!(!extract_bool(&input, "missing", false));
}

#[test]
fn extract_bool_returns_default_when_not_bool() {
    let input = json!({
        "not_bool": "true"
    });
    assert!(extract_bool(&input, "not_bool", true));
    assert!(!extract_bool(&input, "not_bool", false));
}
