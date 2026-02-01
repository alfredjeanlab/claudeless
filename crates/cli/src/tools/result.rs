// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool execution result types.

use serde::{Deserialize, Serialize};

/// Generates `success()` and `error()` factory methods for tool result types.
///
/// Both `ToolExecutionResult` and `ToolResultBlock` share this pattern.
/// Use: `impl_tool_result_factories!([ContentType::Text], extra_field: value, ...);`
#[macro_export]
macro_rules! impl_tool_result_factories {
    ([$($content_text_path:tt)+], $($extra_field:ident: $extra_value:expr),*) => {
        /// Create a successful text result.
        pub fn success(tool_use_id: impl Into<String>, text: impl Into<String>) -> Self {
            Self {
                $($extra_field: $extra_value,)*
                tool_use_id: tool_use_id.into(),
                is_error: false,
                content: vec![$($content_text_path)+ { text: text.into() }],
            }
        }

        /// Create an error result.
        pub fn error(tool_use_id: impl Into<String>, message: impl Into<String>) -> Self {
            Self {
                $($extra_field: $extra_value,)*
                tool_use_id: tool_use_id.into(),
                is_error: true,
                content: vec![$($content_text_path)+ { text: message.into() }],
            }
        }
    };
}

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

    /// Whether this tool needs an interactive permission prompt before executing.
    #[serde(skip)]
    pub needs_prompt: bool,
}

impl ToolExecutionResult {
    impl_tool_result_factories!([ToolResultContent::Text], tool_use_result: None, needs_prompt: false);

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
            needs_prompt: false,
        }
    }

    /// Create a result indicating an interactive permission prompt is needed.
    pub fn needs_prompt(tool_use_id: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            is_error: false,
            content: vec![],
            tool_use_result: None,
            needs_prompt: true,
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
#[path = "result_tests.rs"]
mod tests;
