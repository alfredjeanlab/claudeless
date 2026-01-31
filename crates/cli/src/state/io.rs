// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! IO helpers for JSON serialization with std::io::Error mapping.

use std::path::{Path, PathBuf};

/// Trait for loading JSON from file with IO error mapping.
pub trait JsonLoad: Sized + serde::de::DeserializeOwned {
    fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(to_io_error)
    }
}

/// Serialize to JSON with IO error mapping.
pub fn to_io_json<T: serde::Serialize>(value: &T) -> std::io::Result<String> {
    serde_json::to_string(value).map_err(to_io_error)
}

/// Map an error to std::io::Error with InvalidData kind.
pub fn to_io_error<E: std::error::Error + Send + Sync + 'static>(e: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
}

/// Parse content as JSON5, falling back to strict JSON on parse failure.
///
/// JSON5 supports comments and trailing commas, making config files more readable.
/// Falls back to strict JSON parsing if JSON5 parsing fails, for broad compatibility.
pub fn parse_json5_or_json<T: serde::de::DeserializeOwned>(
    content: &str,
) -> Result<T, serde_json::Error> {
    json5::from_str(content).or_else(|_| serde_json::from_str(content))
}

/// Ensure a file's parent directory exists, creating it and ancestors if needed.
///
/// This is useful before writing to a file path where the parent directory may not exist.
pub fn ensure_parent_exists(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

/// Iterate over file paths in a directory.
///
/// Silently skips entries that can't be read. Returns an empty iterator if the
/// directory doesn't exist or can't be read.
pub fn files_in(dir: &Path) -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
}

/// Iterate over JSON file paths in a directory.
///
/// Filters to only files with `.json` extension. Silently skips entries that
/// can't be read. Returns an empty iterator if the directory doesn't exist.
pub fn json_files_in(dir: &Path) -> impl Iterator<Item = PathBuf> {
    files_in(dir).filter(|p| p.extension().is_some_and(|e| e == "json"))
}
