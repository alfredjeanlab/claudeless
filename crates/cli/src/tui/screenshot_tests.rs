#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_screenshot_from_text() {
    let screenshot = Screenshot::from_text("Hello, world!\nSecond line", 40, 10);

    assert_eq!(screenshot.width, 40);
    assert_eq!(screenshot.height, 10);
    assert_eq!(screenshot.lines.len(), 2);
    assert!(screenshot.to_text().contains("Hello, world!"));
}

#[test]
fn test_screenshot_diff() {
    let s1 = Screenshot::from_text("First", 20, 5);
    let s2 = Screenshot::from_text("Second", 20, 5);

    let diffs = s1.diff(&s2);
    assert!(!diffs.is_empty());
}

#[test]
fn test_screenshot_matches() {
    let s1 = Screenshot::from_text("Same", 20, 5);
    let s2 = Screenshot::from_text("Same", 20, 5);

    assert!(s1.matches(&s2));
}

#[test]
fn test_capture_text() {
    let mut capture = ScreenshotCapture::new(40, 10);

    let screenshot = capture.capture_text("Hello, world!", Some("test"));

    assert_eq!(screenshot.width, 40);
    assert_eq!(screenshot.height, 10);
    assert!(screenshot.to_text().contains("Hello, world!"));
}

#[test]
fn test_multiple_captures() {
    let mut capture = ScreenshotCapture::new(20, 5);

    capture.capture_text("First", Some("first"));
    capture.capture_text("Second", Some("second"));

    assert_eq!(capture.captures().len(), 2);
    assert_eq!(
        capture.last().unwrap().metadata.label,
        Some("second".to_string())
    );
}
