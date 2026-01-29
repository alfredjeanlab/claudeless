//! Terminal screen buffer using avt.
//!
//! Wraps the avt virtual terminal to track rendered state
//! and detect changes between frames.

use anyhow::Result;
use regex::Regex;
use std::fmt::Write;
use std::path::Path;

/// Terminal screen state with change detection.
pub struct Screen {
    vt: avt::Vt,
    last_frame: Option<String>,
    frame_seq: u64,
}

impl Screen {
    /// Create a new screen buffer.
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            vt: avt::Vt::new(cols as usize, rows as usize),
            last_frame: None,
            frame_seq: 0,
        }
    }

    /// Feed raw terminal output through the parser.
    pub fn feed(&mut self, data: &[u8]) {
        self.vt.feed_str(&String::from_utf8_lossy(data));
    }

    /// Render current screen state as text.
    pub fn render(&self) -> String {
        self.vt.text().join("\n")
    }

    /// Render current screen with ANSI escape codes.
    pub fn render_ansi(&self) -> String {
        let mut out = String::new();

        for (i, line) in self.vt.view().iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }

            let mut last_pen = avt::Pen::default();
            for cell in line.cells().iter() {
                let pen = cell.pen();
                if pen != &last_pen {
                    // Reset and apply new style
                    out.push_str("\x1b[0m");
                    let _ = write!(out, "{}", pen_to_ansi(pen));
                    last_pen = *pen;
                }
                out.push(cell.char());
            }
            // Reset at end of line
            out.push_str("\x1b[0m");
        }

        out
    }

    /// Check if screen changed since last save.
    pub fn changed(&self) -> bool {
        match &self.last_frame {
            Some(last) => &self.render() != last,
            None => true,
        }
    }

    /// Save frame (both plain and ANSI), returns frame number.
    pub fn save_frame(&mut self, dir: &Path) -> Result<u64> {
        self.frame_seq += 1;
        let seq = self.frame_seq;

        // Plain text
        let plain = self.render();
        std::fs::write(dir.join(format!("{:06}.txt", seq)), &plain)?;

        // ANSI
        let ansi = self.render_ansi();
        std::fs::write(dir.join(format!("{:06}.ansi.txt", seq)), &ansi)?;

        // Update latest symlink
        let latest = dir.join("latest.txt");
        let _ = std::fs::remove_file(&latest);
        #[cfg(unix)]
        std::os::unix::fs::symlink(format!("{:06}.txt", seq), &latest)?;

        self.last_frame = Some(plain);
        Ok(seq)
    }

    /// Check if pattern matches anywhere in current screen.
    pub fn matches(&self, pattern: &Regex) -> bool {
        pattern.is_match(&self.render())
    }
}

/// Convert a Pen to ANSI escape sequence.
fn pen_to_ansi(pen: &avt::Pen) -> String {
    if pen.is_default() {
        return String::new();
    }

    let mut codes: Vec<String> = Vec::new();

    // Text attributes
    if pen.is_bold() {
        codes.push("1".to_string());
    }
    if pen.is_faint() {
        codes.push("2".to_string());
    }
    if pen.is_italic() {
        codes.push("3".to_string());
    }
    if pen.is_underline() {
        codes.push("4".to_string());
    }
    if pen.is_blink() {
        codes.push("5".to_string());
    }
    if pen.is_inverse() {
        codes.push("7".to_string());
    }
    if pen.is_strikethrough() {
        codes.push("9".to_string());
    }

    // Foreground color
    if let Some(fg) = pen.foreground() {
        match fg {
            avt::Color::Indexed(n) => {
                if n < 8 {
                    codes.push(format!("{}", 30 + n));
                } else if n < 16 {
                    codes.push(format!("{}", 90 + n - 8));
                } else {
                    codes.push(format!("38;5;{}", n));
                }
            }
            avt::Color::RGB(rgb) => {
                codes.push(format!("38;2;{};{};{}", rgb.r, rgb.g, rgb.b));
            }
        }
    }

    // Background color
    if let Some(bg) = pen.background() {
        match bg {
            avt::Color::Indexed(n) => {
                if n < 8 {
                    codes.push(format!("{}", 40 + n));
                } else if n < 16 {
                    codes.push(format!("{}", 100 + n - 8));
                } else {
                    codes.push(format!("48;5;{}", n));
                }
            }
            avt::Color::RGB(rgb) => {
                codes.push(format!("48;2;{};{};{}", rgb.r, rgb.g, rgb.b));
            }
        }
    }

    if codes.is_empty() {
        String::new()
    } else {
        format!("\x1b[{}m", codes.join(";"))
    }
}

#[cfg(test)]
#[path = "screen_tests.rs"]
mod tests;
