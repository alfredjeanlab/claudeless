#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn screen_renders_text() {
    let mut screen = Screen::new(80, 24);
    screen.feed(b"Hello World\r\n");
    screen.feed(b"Line Two\r\n");
    let text = screen.render();
    assert!(text.contains("Hello World"));
    assert!(text.contains("Line Two"));
}

#[test]
fn screen_handles_cursor_positioning() {
    let mut screen = Screen::new(80, 24);
    // Position cursor at row 2, col 5 and print "ABC"
    screen.feed(b"\x1b[2;5HABC");
    let text = screen.render();
    assert!(text.contains("ABC"));
}

#[test]
fn screen_matches_pattern() {
    let mut screen = Screen::new(80, 24);
    screen.feed(b"Ready> ");
    let pattern = Regex::new("Ready>").unwrap();
    assert!(screen.matches(&pattern));
}
