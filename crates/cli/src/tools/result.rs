// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool execution result types.

use serde::{Deserialize, Serialize};

/// Result of a tool execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    /// The tool use ID this result corresponds to.
    pub tool_use_id: String,

    /// Whether this result represents an error.
    pub is_error: bool,

    /// Content of the result.
    pub content: Vec<ToolResultContent>,

    /// Tool-specific result data for JSONL recording (e.g., oldTodos/newTodos for TodoWrite).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_result: Option<serde_json::Value>,
}

impl ToolExecutionResult {
    /// Create a successful text result.
    pub fn success(tool_use_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            is_error: false,
            content: vec![ToolResultContent::Text { text: text.into() }],
            tool_use_result: None,
        }
    }

    /// Create a successful result with tool-specific result data.
    pub fn success_with_result(
        tool_use_id: impl Into<String>,
        text: impl Into<String>,
        tool_use_result: serde_json::Value,
    ) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            is_error: false,
            content: vec![ToolResultContent::Text { text: text.into() }],
            tool_use_result: Some(tool_use_result),
        }
    }

    /// Create an error result.
    pub fn error(tool_use_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            is_error: true,
            content: vec![ToolResultContent::Text {
                text: message.into(),
            }],
            tool_use_result: None,
        }
    }

    /// Create a result indicating no mock result was configured.
    pub fn no_mock_result(tool_use_id: impl Into<String>, tool_name: &str) -> Self {
        Self::error(
            tool_use_id,
            format!("No mock result configured for tool '{}'", tool_name),
        )
    }

    /// Create a result indicating tool execution is disabled.
    pub fn disabled(tool_use_id: impl Into<String>) -> Self {
        Self::error(tool_use_id, "Tool execution is disabled")
    }

    /// Create a result indicating permission was denied.
    pub fn permission_denied(tool_use_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::error(tool_use_id, format!("Permission denied: {}", reason.into()))
    }

    /// Get the text content if this is a simple text result.
    pub fn text(&self) -> Option<&str> {
        if self.content.len() == 1 {
            if let ToolResultContent::Text { text } = &self.content[0] {
                return Some(text);
            }
        }
        None
    }

    /// Get the tool-specific result data.
    pub fn tool_use_result(&self) -> Option<serde_json::Value> {
        self.tool_use_result.clone()
    }
}

/// Content types within a tool result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContent {
    /// Text content
    Text { text: String },
    /// Image content (base64 encoded)
    Image { data: String, media_type: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_result() {
        let result = ToolExecutionResult::success("toolu_123", "file contents");
        assert!(!result.is_error);
        assert_eq!(result.tool_use_id, "toolu_123");
        assert_eq!(result.text(), Some("file contents"));
    }

    #[test]
    fn test_error_result() {
        let result = ToolExecutionResult::error("toolu_456", "file not found");
        assert!(result.is_error);
        assert_eq!(result.text(), Some("file not found"));
    }

    #[test]
    fn test_no_mock_result() {
        let result = ToolExecutionResult::no_mock_result("toolu_789", "Bash");
        assert!(result.is_error);
        assert!(result.text().unwrap().contains("No mock result"));
        assert!(result.text().unwrap().contains("Bash"));
    }

    #[test]
    fn test_disabled_result() {
        let result = ToolExecutionResult::disabled("toolu_abc");
        assert!(result.is_error);
        assert!(result.text().unwrap().contains("disabled"));
    }

    #[test]
    fn test_permission_denied() {
        let result = ToolExecutionResult::permission_denied("toolu_def", "DontAsk mode");
        assert!(result.is_error);
        assert!(result.text().unwrap().contains("Permission denied"));
        assert!(result.text().unwrap().contains("DontAsk mode"));
    }

    #[test]
    fn test_serialization() {
        let result = ToolExecutionResult::success("toolu_123", "output");
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolExecutionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tool_use_id, "toolu_123");
        assert!(!parsed.is_error);
    }
}
