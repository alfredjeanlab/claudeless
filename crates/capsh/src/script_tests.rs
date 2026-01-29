#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn parse_wait_pattern() {
    let cmds = parse(r#"wait "Ready>""#).unwrap();
    assert!(matches!(cmds[0], Command::WaitPattern(_, false, None)));
}

#[test]
fn parse_wait_pattern_negated() {
    let cmds = parse(r#"wait !"Loading""#).unwrap();
    if let Command::WaitPattern(regex, negated, timeout) = &cmds[0] {
        assert_eq!(regex.as_str(), "Loading");
        assert!(*negated);
        assert_eq!(*timeout, None);
    } else {
        panic!("expected WaitPattern");
    }
}

#[test]
fn parse_wait_pattern_with_timeout() {
    let cmds = parse(r#"wait "Ready>" 5000"#).unwrap();
    if let Command::WaitPattern(regex, negated, timeout) = &cmds[0] {
        assert_eq!(regex.as_str(), "Ready>");
        assert!(!*negated);
        assert_eq!(*timeout, Some(5000));
    } else {
        panic!("expected WaitPattern");
    }

    let cmds = parse(r#"wait "Ready>" 5s"#).unwrap();
    if let Command::WaitPattern(regex, negated, timeout) = &cmds[0] {
        assert_eq!(regex.as_str(), "Ready>");
        assert!(!*negated);
        assert_eq!(*timeout, Some(5000));
    } else {
        panic!("expected WaitPattern");
    }

    // Negated with timeout
    let cmds = parse(r#"wait !"Loading" 2s"#).unwrap();
    if let Command::WaitPattern(regex, negated, timeout) = &cmds[0] {
        assert_eq!(regex.as_str(), "Loading");
        assert!(*negated);
        assert_eq!(*timeout, Some(2000));
    } else {
        panic!("expected WaitPattern");
    }
}

#[test]
fn parse_wait_ms() {
    let cmds = parse("wait 2000").unwrap();
    assert!(matches!(cmds[0], Command::WaitMs(2000)));

    let cmds = parse("wait 2000ms").unwrap();
    assert!(matches!(cmds[0], Command::WaitMs(2000)));

    let cmds = parse("wait 2s").unwrap();
    assert!(matches!(cmds[0], Command::WaitMs(2000)));

    let cmds = parse("wait 1m").unwrap();
    assert!(matches!(cmds[0], Command::WaitMs(60000)));
}

#[test]
fn parse_send_text() {
    let cmds = parse(r#"send "hello\n""#).unwrap();
    if let Command::Send(parts) = &cmds[0] {
        assert_eq!(parts, &[SendPart::Bytes(b"hello\n".to_vec())]);
    } else {
        panic!("expected Send");
    }
}

#[test]
fn parse_send_special_keys() {
    let cmds = parse("send <Up> <C-d>").unwrap();
    if let Command::Send(parts) = &cmds[0] {
        assert_eq!(parts, &[SendPart::Bytes(b"\x1b[A\x04".to_vec())]);
    } else {
        panic!("expected Send");
    }
}

#[test]
fn parse_send_with_delay() {
    let cmds = parse(r#"send "hello" 150 <Enter>"#).unwrap();
    if let Command::Send(parts) = &cmds[0] {
        assert_eq!(
            parts,
            &[
                SendPart::Bytes(b"hello".to_vec()),
                SendPart::Delay(150),
                SendPart::Bytes(b"\r".to_vec()),
            ]
        );
    } else {
        panic!("expected Send");
    }

    // With duration suffix
    let cmds = parse(r#"send "hello" 1s <Enter>"#).unwrap();
    if let Command::Send(parts) = &cmds[0] {
        assert_eq!(
            parts,
            &[
                SendPart::Bytes(b"hello".to_vec()),
                SendPart::Delay(1000),
                SendPart::Bytes(b"\r".to_vec()),
            ]
        );
    } else {
        panic!("expected Send");
    }
}

#[test]
fn parse_snapshot() {
    let cmds = parse("snapshot").unwrap();
    assert!(matches!(cmds[0], Command::Snapshot(None)));
}

#[test]
fn parse_snapshot_with_name() {
    let cmds = parse(r#"snapshot "initial-state""#).unwrap();
    if let Command::Snapshot(Some(name)) = &cmds[0] {
        assert_eq!(name, "initial-state");
    } else {
        panic!("expected Snapshot with name");
    }
}

#[test]
fn parse_send_meta_key() {
    let cmds = parse("send <M-p>").unwrap();
    if let Command::Send(parts) = &cmds[0] {
        assert_eq!(parts, &[SendPart::Bytes(vec![0x1b, b'p'])]);
    } else {
        panic!("expected Send");
    }

    // Alt-P should also work
    let cmds = parse("send <A-p>").unwrap();
    if let Command::Send(parts) = &cmds[0] {
        assert_eq!(parts, &[SendPart::Bytes(vec![0x1b, b'p'])]);
    } else {
        panic!("expected Send");
    }

    // Uppercase should normalize to lowercase
    let cmds = parse("send <M-P>").unwrap();
    if let Command::Send(parts) = &cmds[0] {
        assert_eq!(parts, &[SendPart::Bytes(vec![0x1b, b'p'])]);
    } else {
        panic!("expected Send");
    }
}

#[test]
fn parse_kill_signal_name() {
    use nix::sys::signal::Signal;

    let cmds = parse("kill SIGTERM").unwrap();
    assert!(matches!(cmds[0], Command::Kill(Signal::SIGTERM)));

    let cmds = parse("kill TERM").unwrap();
    assert!(matches!(cmds[0], Command::Kill(Signal::SIGTERM)));

    let cmds = parse("kill SIGKILL").unwrap();
    assert!(matches!(cmds[0], Command::Kill(Signal::SIGKILL)));
}

#[test]
fn parse_kill_signal_number() {
    use nix::sys::signal::Signal;

    let cmds = parse("kill 9").unwrap();
    assert!(matches!(cmds[0], Command::Kill(Signal::SIGKILL)));

    let cmds = parse("kill 15").unwrap();
    assert!(matches!(cmds[0], Command::Kill(Signal::SIGTERM)));
}
