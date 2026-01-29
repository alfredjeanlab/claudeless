//! Terminal screen buffer using avt.
//!
//! Wraps the avt virtual terminal to track rendered state
//! and detect changes between frames.

use anyhow::Result;
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

    /// Check if screen changed since last snapshot.
    pub fn changed(&self) -> bool {
        let current = self.render();
        match &self.last_frame {
            Some(last) => &current != last,
            None => true,
        }
    }

    /// Save frame if changed, returns frame number if saved.
    pub fn save_if_changed(&mut self, dir: &Path) -> Result<Option<u64>> {
        if !self.changed() {
            return Ok(None);
        }

        let frame = self.render();
        self.frame_seq += 1;
        let seq = self.frame_seq;

        // Write numbered frame
        let path = dir.join(format!("{:06}.txt", seq));
        std::fs::write(&path, &frame)?;

        // Update latest symlink
        let latest = dir.join("latest.txt");
        let _ = std::fs::remove_file(&latest);
        #[cfg(unix)]
        std::os::unix::fs::symlink(&path, &latest)?;

        self.last_frame = Some(frame);
        Ok(Some(seq))
    }
}
