use super::*;
use tempfile::TempDir;

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

#[test]
fn screen_render_ansi_includes_escapes() {
    let mut screen = Screen::new(80, 24);
    // Bold text: ESC[1m
    screen.feed(b"\x1b[1mBold\x1b[0m Normal");
    let ansi = screen.render_ansi();

    // Should contain escape sequences
    assert!(ansi.contains("\x1b["));
    // Should contain the text
    assert!(ansi.contains("Bold"));
    assert!(ansi.contains("Normal"));
}

#[test]
fn screen_render_ansi_handles_colors() {
    let mut screen = Screen::new(80, 24);
    // Red foreground: ESC[31m
    screen.feed(b"\x1b[31mRed\x1b[0m");
    let ansi = screen.render_ansi();

    assert!(ansi.contains("Red"));
    // Should have color codes
    assert!(ansi.contains("\x1b["));
}

#[test]
fn screen_changed_detects_differences() {
    let mut screen = Screen::new(80, 24);

    // Initially changed (no last_frame)
    assert!(screen.changed());

    // After save, not changed
    let dir = TempDir::new().unwrap();
    screen.save_frame(dir.path()).unwrap();
    assert!(!screen.changed());

    // After new content, changed again
    screen.feed(b"new content");
    assert!(screen.changed());
}

#[test]
fn screen_save_frame_creates_both_files() {
    let mut screen = Screen::new(80, 24);
    screen.feed(b"Hello");

    let dir = TempDir::new().unwrap();
    let seq = screen.save_frame(dir.path()).unwrap();

    assert_eq!(seq, 1);
    assert!(dir.path().join("000001.txt").exists());
    assert!(dir.path().join("000001.ansi.txt").exists());
    assert!(dir.path().join("latest.txt").exists());

    // Plain text content
    let plain = std::fs::read_to_string(dir.path().join("000001.txt")).unwrap();
    assert!(plain.contains("Hello"));

    // ANSI content should also have the text
    let ansi = std::fs::read_to_string(dir.path().join("000001.ansi.txt")).unwrap();
    assert!(ansi.contains("Hello"));
}

#[test]
fn screen_save_frame_increments_seq() {
    let mut screen = Screen::new(80, 24);
    let dir = TempDir::new().unwrap();

    screen.feed(b"frame 1");
    let seq1 = screen.save_frame(dir.path()).unwrap();

    screen.feed(b"frame 2");
    let seq2 = screen.save_frame(dir.path()).unwrap();

    assert_eq!(seq1, 1);
    assert_eq!(seq2, 2);
    assert!(dir.path().join("000002.txt").exists());
}

#[test]
fn screen_save_frame_deduplicates_unchanged() {
    let mut screen = Screen::new(80, 24);
    let dir = TempDir::new().unwrap();

    screen.feed(b"same content");
    let seq1 = screen.save_frame(dir.path()).unwrap();

    // Save again without changes - should return same seq, no new file
    let seq2 = screen.save_frame(dir.path()).unwrap();

    assert_eq!(seq1, 1);
    assert_eq!(seq2, 1); // Same sequence number
    assert!(dir.path().join("000001.txt").exists());
    assert!(!dir.path().join("000002.txt").exists()); // No duplicate file

    // After actual change, should create new file
    screen.feed(b" more");
    let seq3 = screen.save_frame(dir.path()).unwrap();
    assert_eq!(seq3, 2);
    assert!(dir.path().join("000002.txt").exists());
}

#[test]
fn screen_handles_split_utf8() {
    let mut screen = Screen::new(80, 24);

    // The box-drawing character ─ is U+2500, encoded as E2 94 80 in UTF-8
    // Split it across two feed() calls
    screen.feed(&[0xE2, 0x94]); // First two bytes
    screen.feed(&[0x80]); // Last byte

    let text = screen.render();
    assert!(
        text.contains("─"),
        "should correctly reassemble split UTF-8: {:?}",
        text
    );
    assert!(
        !text.contains("�"),
        "should not contain replacement characters"
    );
}

#[test]
fn screen_handles_multiple_split_utf8() {
    let mut screen = Screen::new(80, 24);

    // Send "Hello ─ World" but split the ─ character
    screen.feed(b"Hello \xE2"); // 'Hello ' + first byte of ─
    screen.feed(b"\x94\x80 World"); // rest of ─ + ' World'

    let text = screen.render();
    assert!(
        text.contains("Hello ─ World"),
        "should handle split UTF-8 in context: {:?}",
        text
    );
}

#[test]
fn pen_to_ansi_empty_for_default() {
    let pen = avt::Pen::default();
    let ansi = pen_to_ansi(&pen);
    assert!(ansi.is_empty());
}

#[test]
fn pen_to_ansi_handles_attributes() {
    // Test via render_ansi with actual terminal sequences
    let mut screen = Screen::new(80, 24);
    // Feed bold text (SGR 1)
    screen.feed(b"\x1b[1mBold\x1b[0m");
    let ansi = screen.render_ansi();

    // Should contain SGR codes
    assert!(ansi.contains("\x1b["), "should have escape sequences");
    assert!(ansi.contains("Bold"), "should have text");
}
