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
fn parse_if_wait_then_else_end() {
    let script = r#"
if wait "Ready" 2s
    snapshot "ready"
else
    snapshot "not_ready"
end
"#;
    let cmds = parse(script).unwrap();
    assert_eq!(cmds.len(), 1);
    if let Command::IfWait {
        pattern,
        negated,
        timeout,
        then_cmds,
        else_cmds,
    } = &cmds[0]
    {
        assert_eq!(pattern.as_str(), "Ready");
        assert!(!negated);
        assert_eq!(*timeout, Some(2000));
        assert_eq!(then_cmds.len(), 1);
        assert_eq!(else_cmds.len(), 1);
    } else {
        panic!("expected IfWait");
    }
}

#[test]
fn parse_if_wait_without_else() {
    let script = r#"
if wait "Ready"
    snapshot "ready"
end
"#;
    let cmds = parse(script).unwrap();
    if let Command::IfWait { else_cmds, .. } = &cmds[0] {
        assert!(else_cmds.is_empty());
    } else {
        panic!("expected IfWait");
    }
}

#[test]
fn parse_if_wait_negated() {
    let script = r#"
if wait !"Loading" 1s
    snapshot "loaded"
end
"#;
    let cmds = parse(script).unwrap();
    if let Command::IfWait { negated, .. } = &cmds[0] {
        assert!(*negated);
    } else {
        panic!("expected IfWait");
    }
}

#[test]
fn parse_else_if() {
    let script = r#"
if wait "StateA" 1s
    snapshot "state_a"
else if wait "StateB" 1s
    snapshot "state_b"
else
    snapshot "unknown"
end
"#;
    let cmds = parse(script).unwrap();
    assert_eq!(cmds.len(), 1);

    // Outer if
    if let Command::IfWait {
        pattern,
        then_cmds,
        else_cmds,
        ..
    } = &cmds[0]
    {
        assert_eq!(pattern.as_str(), "StateA");
        assert_eq!(then_cmds.len(), 1);
        assert!(matches!(then_cmds[0], Command::Snapshot(Some(ref n)) if n == "state_a"));

        // Else contains the else-if as a nested IfWait
        assert_eq!(else_cmds.len(), 1);
        if let Command::IfWait {
            pattern: inner_pattern,
            then_cmds: inner_then,
            else_cmds: inner_else,
            ..
        } = &else_cmds[0]
        {
            assert_eq!(inner_pattern.as_str(), "StateB");
            assert_eq!(inner_then.len(), 1);
            assert!(matches!(inner_then[0], Command::Snapshot(Some(ref n)) if n == "state_b"));
            assert_eq!(inner_else.len(), 1);
            assert!(matches!(inner_else[0], Command::Snapshot(Some(ref n)) if n == "unknown"));
        } else {
            panic!("expected nested IfWait in else block");
        }
    } else {
        panic!("expected IfWait");
    }
}

#[test]
fn parse_multiple_else_if() {
    let script = r#"
if wait "A" 1s
    snapshot "a"
else if wait "B" 1s
    snapshot "b"
else if wait "C" 1s
    snapshot "c"
else if wait "D" 1s
    snapshot "d"
else
    snapshot "fallback"
end
"#;
    let cmds = parse(script).unwrap();
    assert_eq!(cmds.len(), 1);

    // Walk the chain: A -> B -> C -> D -> fallback
    let mut current = &cmds[0];
    for (expected_pattern, expected_snapshot) in [("A", "a"), ("B", "b"), ("C", "c"), ("D", "d")] {
        if let Command::IfWait {
            pattern,
            then_cmds,
            else_cmds,
            ..
        } = current
        {
            assert_eq!(pattern.as_str(), expected_pattern);
            assert_eq!(then_cmds.len(), 1);
            assert!(
                matches!(then_cmds[0], Command::Snapshot(Some(ref n)) if n == expected_snapshot)
            );
            assert_eq!(else_cmds.len(), 1);
            current = &else_cmds[0];
        } else {
            panic!("expected IfWait for pattern {}", expected_pattern);
        }
    }

    // Final else should be snapshot "fallback"
    assert!(matches!(current, Command::Snapshot(Some(ref n)) if n == "fallback"));
}

#[test]
fn parse_else_if_without_final_else() {
    let script = r#"
if wait "A" 1s
    snapshot "a"
else if wait "B" 1s
    snapshot "b"
end
"#;
    let cmds = parse(script).unwrap();
    assert_eq!(cmds.len(), 1);

    if let Command::IfWait { else_cmds, .. } = &cmds[0] {
        assert_eq!(else_cmds.len(), 1);
        if let Command::IfWait {
            else_cmds: inner_else,
            ..
        } = &else_cmds[0]
        {
            assert!(inner_else.is_empty());
        } else {
            panic!("expected nested IfWait");
        }
    } else {
        panic!("expected IfWait");
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

#[test]
fn parse_match_basic() {
    let script = r#"
match 3s
    "Sonnet" -> snapshot "model_sonnet"
    "Opus" -> snapshot "model_opus"
    "Haiku" -> snapshot "model_haiku"
end
"#;
    let cmds = parse(script).unwrap();
    assert_eq!(cmds.len(), 1);

    if let Command::Match {
        timeout,
        arms,
        else_cmds,
    } = &cmds[0]
    {
        assert_eq!(*timeout, Some(3000));
        assert_eq!(arms.len(), 3);
        assert_eq!(arms[0].pattern.as_str(), "Sonnet");
        assert_eq!(arms[1].pattern.as_str(), "Opus");
        assert_eq!(arms[2].pattern.as_str(), "Haiku");
        assert!(
            matches!(arms[0].commands[0], Command::Snapshot(Some(ref n)) if n == "model_sonnet")
        );
        assert!(matches!(arms[1].commands[0], Command::Snapshot(Some(ref n)) if n == "model_opus"));
        assert!(
            matches!(arms[2].commands[0], Command::Snapshot(Some(ref n)) if n == "model_haiku")
        );
        assert!(else_cmds.is_empty());
    } else {
        panic!("expected Match");
    }
}

#[test]
fn parse_match_with_else() {
    let script = r#"
match 5s
    "Ready" -> snapshot "ready"
    "Error" -> snapshot "error"
else
    snapshot "timeout"
end
"#;
    let cmds = parse(script).unwrap();

    if let Command::Match {
        timeout,
        arms,
        else_cmds,
    } = &cmds[0]
    {
        assert_eq!(*timeout, Some(5000));
        assert_eq!(arms.len(), 2);
        assert_eq!(else_cmds.len(), 1);
        assert!(matches!(else_cmds[0], Command::Snapshot(Some(ref n)) if n == "timeout"));
    } else {
        panic!("expected Match");
    }
}

#[test]
fn parse_match_no_timeout() {
    let script = r#"
match
    "pattern" -> snapshot "found"
end
"#;
    let cmds = parse(script).unwrap();

    if let Command::Match { timeout, arms, .. } = &cmds[0] {
        assert_eq!(*timeout, None);
        assert_eq!(arms.len(), 1);
    } else {
        panic!("expected Match");
    }
}

#[test]
fn parse_match_with_blocks() {
    let script = r#"
match 3s
    "Sonnet" ->
        snapshot "model_sonnet"
        send "selected sonnet\n"
    "Opus" ->
        snapshot "model_opus"
        send "selected opus\n"
    "Haiku" -> snapshot "model_haiku"
else
    snapshot "unknown"
end
"#;
    let cmds = parse(script).unwrap();

    if let Command::Match {
        timeout,
        arms,
        else_cmds,
    } = &cmds[0]
    {
        assert_eq!(*timeout, Some(3000));
        assert_eq!(arms.len(), 3);

        // Sonnet arm has 2 commands (block)
        assert_eq!(arms[0].pattern.as_str(), "Sonnet");
        assert_eq!(arms[0].commands.len(), 2);
        assert!(
            matches!(arms[0].commands[0], Command::Snapshot(Some(ref n)) if n == "model_sonnet")
        );
        assert!(matches!(arms[0].commands[1], Command::Send(_)));

        // Opus arm has 2 commands (block)
        assert_eq!(arms[1].pattern.as_str(), "Opus");
        assert_eq!(arms[1].commands.len(), 2);

        // Haiku arm has 1 command (inline)
        assert_eq!(arms[2].pattern.as_str(), "Haiku");
        assert_eq!(arms[2].commands.len(), 1);

        // Else block
        assert_eq!(else_cmds.len(), 1);
    } else {
        panic!("expected Match");
    }
}
