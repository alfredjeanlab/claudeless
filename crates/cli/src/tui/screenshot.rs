// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Terminal screenshot capture for testing.
//!
//! Note: Screenshot capture has been simplified for iocraft migration.
//! The buffer-based capture is no longer available; use string-based
//! comparison instead.

use serde::{Deserialize, Serialize};

/// A captured screenshot of the terminal state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Screenshot {
    /// Width of the capture
    pub width: u16,

    /// Height of the capture
    pub height: u16,

    /// Captured content as lines
    pub lines: Vec<String>,

    /// Metadata about the capture
    pub metadata: ScreenshotMetadata,
}

/// Metadata about the screenshot
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScreenshotMetadata {
    /// Timestamp (ms since epoch)
    pub timestamp: u64,

    /// App mode at capture time
    pub mode: String,

    /// Description/label for the screenshot
    pub label: Option<String>,
}

impl Screenshot {
    /// Create from text content
    pub fn from_text(text: &str, width: u16, height: u16) -> Self {
        let lines: Vec<String> = text.lines().map(|l| l.trim_end().to_string()).collect();

        Self {
            width,
            height,
            lines,
            metadata: ScreenshotMetadata::default(),
        }
    }

    /// Convert to a single string for display/comparison
    pub fn to_text(&self) -> String {
        self.lines.join("\n")
    }

    /// Compare with another screenshot
    pub fn diff(&self, other: &Screenshot) -> Vec<LineDiff> {
        let mut diffs = Vec::new();

        let max_lines = self.lines.len().max(other.lines.len());
        for i in 0..max_lines {
            let a = self.lines.get(i).map(String::as_str).unwrap_or("");
            let b = other.lines.get(i).map(String::as_str).unwrap_or("");

            if a != b {
                diffs.push(LineDiff {
                    line_number: i,
                    expected: a.to_string(),
                    actual: b.to_string(),
                });
            }
        }

        diffs
    }

    /// Check if two screenshots are identical
    pub fn matches(&self, other: &Screenshot) -> bool {
        self.width == other.width && self.height == other.height && self.lines == other.lines
    }
}

/// A difference between two lines
#[derive(Clone, Debug)]
pub struct LineDiff {
    pub line_number: usize,
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Display for LineDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Line {}:", self.line_number)?;
        writeln!(f, "  expected: {:?}", self.expected)?;
        writeln!(f, "  actual:   {:?}", self.actual)
    }
}

/// Screenshot capture helper for testing
/// Note: This is a simplified version that doesn't require ratatui
pub struct ScreenshotCapture {
    /// Terminal width
    width: u16,

    /// Terminal height
    height: u16,

    /// Captured screenshots
    captures: Vec<Screenshot>,
}

impl ScreenshotCapture {
    /// Create a new capture helper
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            captures: Vec::new(),
        }
    }

    /// Capture text content directly
    pub fn capture_text(&mut self, content: &str, label: Option<&str>) -> Screenshot {
        let metadata = ScreenshotMetadata {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            mode: String::new(),
            label: label.map(String::from),
        };

        let lines: Vec<String> = content.lines().map(|l| l.trim_end().to_string()).collect();

        let screenshot = Screenshot {
            width: self.width,
            height: self.height,
            lines,
            metadata,
        };

        self.captures.push(screenshot.clone());
        screenshot
    }

    /// Get all captured screenshots
    pub fn captures(&self) -> &[Screenshot] {
        &self.captures
    }

    /// Get the most recent capture
    pub fn last(&self) -> Option<&Screenshot> {
        self.captures.last()
    }

    /// Clear all captures
    pub fn clear(&mut self) {
        self.captures.clear();
    }
}

/// Assert that a screenshot matches expected content
#[macro_export]
macro_rules! assert_screenshot {
    ($screenshot:expr, $expected:expr) => {
        let expected_lines: Vec<&str> = $expected.lines().collect();
        let actual_lines: Vec<&str> = $screenshot.lines.iter().map(|s| s.as_str()).collect();

        for (i, (exp, act)) in expected_lines.iter().zip(actual_lines.iter()).enumerate() {
            assert_eq!(
                exp.trim_end(),
                act.trim_end(),
                "Line {} mismatch:\n  expected: {:?}\n  actual:   {:?}",
                i,
                exp,
                act
            );
        }
    };
}

#[cfg(test)]
mod tests {
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
}
