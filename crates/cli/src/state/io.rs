// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! IO helpers for JSON serialization with std::io::Error mapping.

use std::path::Path;

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
