#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn error_plain_text_when_not_terminal() {
    let mut buf = Vec::new();
    write_error(&mut buf, "something went wrong", false);
    let output = String::from_utf8(buf).unwrap();
    assert_eq!(output, "Error: something went wrong\n");
}

#[test]
fn error_with_ansi_when_terminal() {
    let mut buf = Vec::new();
    write_error(&mut buf, "something went wrong", true);
    let output = String::from_utf8(buf).unwrap();
    assert_eq!(output, "\x1b[31mError: something went wrong\x1b[0m\n");
}

#[test]
fn warning_plain_text_when_not_terminal() {
    let mut buf = Vec::new();
    write_warning(&mut buf, "something might be wrong", false);
    let output = String::from_utf8(buf).unwrap();
    assert_eq!(output, "Warning: something might be wrong\n");
}

#[test]
fn warning_with_ansi_when_terminal() {
    let mut buf = Vec::new();
    write_warning(&mut buf, "something might be wrong", true);
    let output = String::from_utf8(buf).unwrap();
    assert_eq!(output, "\x1b[33mWarning: something might be wrong\x1b[0m\n");
}

#[test]
fn error_with_format_args() {
    let mut buf = Vec::new();
    write_error(&mut buf, format_args!("failed after {} attempts", 3), false);
    let output = String::from_utf8(buf).unwrap();
    assert_eq!(output, "Error: failed after 3 attempts\n");
}

#[test]
fn warning_with_format_args() {
    let mut buf = Vec::new();
    write_warning(&mut buf, format_args!("retrying {} times", 5), false);
    let output = String::from_utf8(buf).unwrap();
    assert_eq!(output, "Warning: retrying 5 times\n");
}
