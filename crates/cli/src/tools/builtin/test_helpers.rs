// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared test helpers for builtin tool executor tests.

use super::*;
use crate::config::ToolCallSpec;
use crate::tools::result::ToolExecutionResult;
use std::path::PathBuf;
use tempfile::TempDir;

/// A temporary file with its parent directory for testing.
pub struct TestFile {
    // NOTE(lifetime): Held for Drop semantics (cleanup on test completion)
    #[allow(dead_code)]
    dir: TempDir,
    pub path: PathBuf,
}

impl TestFile {
    /// Create a new test file (file not yet created on disk).
    pub fn new(name: &str) -> Self {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(name);
        Self { dir, path }
    }

    /// Write content to the file.
    pub fn with_content(self, content: &str) -> Self {
        std::fs::write(&self.path, content).unwrap();
        self
    }

    /// Get the path as a string.
    pub fn path_str(&self) -> &str {
        self.path.to_str().unwrap()
    }
}

/// A temporary directory for testing with multiple files.
pub struct TestDir {
    dir: TempDir,
}

impl TestDir {
    /// Create a new empty test directory.
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
        }
    }

    /// Add a file with the given content.
    pub fn with_file(self, name: &str, content: &str) -> Self {
        std::fs::write(self.dir.path().join(name), content).unwrap();
        self
    }

    /// Get the directory path as a string.
    pub fn path_str(&self) -> &str {
        self.dir.path().to_str().unwrap()
    }
}

/// Create a tool call spec with the given tool name and input.
pub fn tool_call(tool: &str, input: serde_json::Value) -> ToolCallSpec {
    ToolCallSpec {
        tool: tool.to_string(),
        input,
        result: None,
    }
}

/// Execute a builtin tool executor with default context.
pub fn execute<E: BuiltinToolExecutor + Default>(input: serde_json::Value) -> ToolExecutionResult {
    let executor = E::default();
    let call = tool_call(executor.tool_name().as_str(), input);
    executor.execute(&call, "test_id", &BuiltinContext::default())
}

/// Assert that a tool result is an error containing the expected text.
pub fn assert_tool_error_contains(result: &ToolExecutionResult, expected: &str) {
    assert!(
        result.is_error,
        "Expected error but got success: {:?}",
        result
    );
    let text = result.text().expect("Error result should have text");
    assert!(
        text.contains(expected),
        "Expected '{}' in: {}",
        expected,
        text
    );
}

/// Assert that a tool result is a success containing the expected text.
pub fn assert_tool_success_contains(result: &ToolExecutionResult, expected: &str) {
    assert!(
        !result.is_error,
        "Expected success but got error: {:?}",
        result
    );
    let text = result.text().expect("Success result should have text");
    assert!(
        text.contains(expected),
        "Expected '{}' in: {}",
        expected,
        text
    );
}

/// Execute a builtin tool by name with the given input.
pub fn execute_tool(tool: &str, input: serde_json::Value) -> ToolExecutionResult {
    let executor = BuiltinExecutor::new();
    let call = tool_call(tool, input);
    executor.execute(
        &call,
        "test_id",
        &crate::tools::executor::ExecutionContext::default(),
    )
}

#[cfg(test)]
#[path = "test_helpers_tests.rs"]
mod tests;
