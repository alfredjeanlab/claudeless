#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn parse_wait_pattern() {
    let cmds = parse(r#"wait "Ready>""#).unwrap();
    assert!(matches!(cmds[0], Command::WaitPattern(_)));
}

#[test]
fn parse_wait_ms() {
    let cmds = parse("wait 2000").unwrap();
    assert!(matches!(cmds[0], Command::WaitMs(2000)));
}

#[test]
fn parse_send_text() {
    let cmds = parse(r#"send "hello\n""#).unwrap();
    if let Command::Send(bytes) = &cmds[0] {
        assert_eq!(bytes, b"hello\n");
    } else {
        panic!("expected Send");
    }
}

#[test]
fn parse_send_special_keys() {
    let cmds = parse("send <Up> <C-d>").unwrap();
    if let Command::Send(bytes) = &cmds[0] {
        assert_eq!(bytes, b"\x1b[A\x04");
    } else {
        panic!("expected Send");
    }
}
