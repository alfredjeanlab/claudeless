// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Output format handling for text, JSON, and streaming JSON modes.

use crate::cli::OutputFormat;
use crate::config::{ResponseSpec, ToolCallSpec, UsageSpec};
use serde::{Deserialize, Serialize};
use std::io::Write;

/// Result wrapper for JSON output matching real Claude's `--output-format json`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultOutput {
    #[serde(rename = "type")]
    pub output_type: String,
    pub subtype: String,
    pub total_cost_usd: f64,
    pub is_error: bool,
    pub duration_ms: u64,
    pub duration_api_ms: u64,
    pub num_turns: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub session_id: String,
    pub uuid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
    #[serde(rename = "modelUsage")]
    pub model_usage: serde_json::Value,
    pub usage: serde_json::Value,
    pub permission_denials: Vec<String>,
}

impl ResultOutput {
    /// Create a success result
    pub fn success(result: String, session_id: String, duration_ms: u64) -> Self {
        Self {
            output_type: "result".to_string(),
            subtype: "success".to_string(),
            total_cost_usd: 0.0, // Simulator doesn't track real costs
            is_error: false,
            duration_ms,
            duration_api_ms: duration_ms.saturating_sub(50),
            num_turns: 1,
            result: Some(result),
            error: None,
            session_id,
            uuid: uuid_stub(),
            retry_after: None,
            model_usage: serde_json::json!({}),
            usage: serde_json::json!({}),
            permission_denials: vec![],
        }
    }

    /// Create an error result
    pub fn error(error: String, session_id: String, duration_ms: u64) -> Self {
        Self {
            output_type: "result".to_string(),
            subtype: "error".to_string(),
            total_cost_usd: 0.0,
            is_error: true,
            duration_ms,
            duration_api_ms: duration_ms.saturating_sub(10),
            num_turns: 0,
            result: None,
            error: Some(error),
            session_id,
            uuid: uuid_stub(),
            retry_after: None,
            model_usage: serde_json::json!({}),
            usage: serde_json::json!({}),
            permission_denials: vec![],
        }
    }

    /// Create a rate limit error result
    pub fn rate_limit(retry_after: u64, session_id: String) -> Self {
        Self {
            output_type: "result".to_string(),
            subtype: "error".to_string(),
            total_cost_usd: 0.0,
            is_error: true,
            duration_ms: 50,
            duration_api_ms: 50,
            num_turns: 0,
            result: None,
            error: Some(format!(
                "Rate limited. Retry after {} seconds.",
                retry_after
            )),
            session_id,
            uuid: uuid_stub(),
            retry_after: Some(retry_after),
            model_usage: serde_json::json!({}),
            usage: serde_json::json!({}),
            permission_denials: vec![],
        }
    }
}

/// JSON response structure matching Claude's output
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonResponse {
    pub id: String,
    pub model: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: String,
    pub usage: Usage,
}

/// Content block in a response
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// Token usage statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Streaming JSON event types
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart {
        message: StreamMessage,
    },
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: u32,
        delta: Delta,
    },
    ContentBlockStop {
        index: u32,
    },
    MessageDelta {
        delta: MessageDelta,
        usage: Usage,
    },
    MessageStop,
}

/// Message header for streaming
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamMessage {
    pub id: String,
    pub model: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
}

/// Delta types for streaming content
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Delta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

/// Delta for message-level updates
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: String,
}

/// Tool result block for stream-json output
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResultBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub tool_use_id: String,
    pub is_error: bool,
    pub content: Vec<ToolResultContentBlock>,
}

impl ToolResultBlock {
    /// Create a successful tool result.
    pub fn success(tool_use_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            block_type: "tool_result".to_string(),
            tool_use_id: tool_use_id.into(),
            is_error: false,
            content: vec![ToolResultContentBlock::Text { text: text.into() }],
        }
    }

    /// Create an error tool result.
    pub fn error(tool_use_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            block_type: "tool_result".to_string(),
            tool_use_id: tool_use_id.into(),
            is_error: true,
            content: vec![ToolResultContentBlock::Text {
                text: message.into(),
            }],
        }
    }

    /// Create from a ToolExecutionResult.
    pub fn from_result(result: &crate::tools::ToolExecutionResult) -> Self {
        Self {
            block_type: "tool_result".to_string(),
            tool_use_id: result.tool_use_id.clone(),
            is_error: result.is_error,
            content: result
                .content
                .iter()
                .map(|c| match c {
                    crate::tools::ToolResultContent::Text { text } => {
                        ToolResultContentBlock::Text { text: text.clone() }
                    }
                    crate::tools::ToolResultContent::Image { data, media_type } => {
                        ToolResultContentBlock::Image {
                            data: data.clone(),
                            media_type: media_type.clone(),
                        }
                    }
                })
                .collect(),
        }
    }
}

/// Content block within a tool result
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContentBlock {
    Text { text: String },
    Image { data: String, media_type: String },
}

// =============================================================================
// Real Claude Format Types
// =============================================================================
// The types below match the actual format produced by Claude Code CLI

/// System init event for stream-json
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemInitEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub subtype: String,
    pub session_id: String,
    pub tools: Vec<String>,
    pub mcp_servers: Vec<String>,
}

impl SystemInitEvent {
    pub fn new(session_id: impl Into<String>, tools: Vec<String>) -> Self {
        Self {
            event_type: "system".to_string(),
            subtype: "init".to_string(),
            session_id: session_id.into(),
            tools,
            mcp_servers: vec![],
        }
    }

    /// Create with MCP servers included.
    pub fn with_mcp_servers(
        session_id: impl Into<String>,
        tools: Vec<String>,
        mcp_servers: Vec<String>,
    ) -> Self {
        Self {
            event_type: "system".to_string(),
            subtype: "init".to_string(),
            session_id: session_id.into(),
            tools,
            mcp_servers,
        }
    }
}

/// Assistant message event for stream-json
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssistantEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub subtype: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<AssistantMessageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ExtendedUsage>,
}

impl AssistantEvent {
    pub fn message_start(message: AssistantMessageContent) -> Self {
        Self {
            event_type: "assistant".to_string(),
            subtype: "message_start".to_string(),
            message: Some(message),
            usage: None,
        }
    }

    pub fn message_delta(usage: ExtendedUsage) -> Self {
        Self {
            event_type: "assistant".to_string(),
            subtype: "message_delta".to_string(),
            message: None,
            usage: Some(usage),
        }
    }

    pub fn message_stop() -> Self {
        Self {
            event_type: "assistant".to_string(),
            subtype: "message_stop".to_string(),
            message: None,
            usage: None,
        }
    }
}

/// Content of assistant message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssistantMessageContent {
    pub id: String,
    pub model: String,
    pub role: String,
    pub content: Vec<serde_json::Value>,
    pub stop_reason: Option<String>,
}

/// Condensed assistant event for stream-json (matches real Claude output)
/// This is the format used in real Claude CLI output - no subtype, includes full message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CondensedAssistantEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub message: CondensedMessage,
    pub session_id: String,
    pub uuid: String,
}

impl CondensedAssistantEvent {
    pub fn new(message: CondensedMessage, session_id: impl Into<String>) -> Self {
        Self {
            event_type: "assistant".to_string(),
            message,
            session_id: session_id.into(),
            uuid: uuid_stub(),
        }
    }
}

/// Message content for condensed assistant event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CondensedMessage {
    pub id: String,
    pub model: String,
    pub role: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub content: serde_json::Value,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: serde_json::Value,
}

/// Extended usage info matching real Claude
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtendedUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
}

impl ExtendedUsage {
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        }
    }
}

/// Content block start event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentBlockStartEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub subtype: String,
    pub index: u32,
}

impl ContentBlockStartEvent {
    pub fn text(index: u32) -> Self {
        Self {
            event_type: "content_block_start".to_string(),
            subtype: "text".to_string(),
            index,
        }
    }

    pub fn tool_use(index: u32) -> Self {
        Self {
            event_type: "content_block_start".to_string(),
            subtype: "tool_use".to_string(),
            index,
        }
    }
}

/// Content block delta event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentBlockDeltaEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub subtype: String,
    pub index: u32,
    pub delta: String,
}

impl ContentBlockDeltaEvent {
    pub fn text(index: u32, delta: impl Into<String>) -> Self {
        Self {
            event_type: "content_block_delta".to_string(),
            subtype: "text_delta".to_string(),
            index,
            delta: delta.into(),
        }
    }
}

/// Content block stop event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentBlockStopEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub index: u32,
}

impl ContentBlockStopEvent {
    pub fn new(index: u32) -> Self {
        Self {
            event_type: "content_block_stop".to_string(),
            index,
        }
    }
}

/// Output writer that handles different formats
pub struct OutputWriter<W: Write> {
    writer: W,
    format: OutputFormat,
    model: String,
}

impl<W: Write> OutputWriter<W> {
    /// Create a new output writer
    pub fn new(writer: W, format: OutputFormat, model: String) -> Self {
        Self {
            writer,
            format,
            model,
        }
    }

    /// Write a response in the configured format
    pub fn write_response(
        &mut self,
        response: &ResponseSpec,
        tool_calls: &[ToolCallSpec],
    ) -> std::io::Result<()> {
        match self.format {
            OutputFormat::Text => self.write_text(response),
            OutputFormat::Json => self.write_json(response, tool_calls),
            OutputFormat::StreamJson => self.write_stream_json(response, tool_calls),
        }
    }

    fn write_text(&mut self, response: &ResponseSpec) -> std::io::Result<()> {
        let text = match response {
            ResponseSpec::Simple(s) => s.as_str(),
            ResponseSpec::Detailed { text, .. } => text.as_str(),
        };
        writeln!(self.writer, "{}", text)
    }

    fn write_json(
        &mut self,
        response: &ResponseSpec,
        tool_calls: &[ToolCallSpec],
    ) -> std::io::Result<()> {
        let (text, usage) = match response {
            ResponseSpec::Simple(s) => (s.clone(), None),
            ResponseSpec::Detailed { text, usage, .. } => (text.clone(), usage.clone()),
        };

        let mut content = vec![ContentBlock::Text { text: text.clone() }];
        for tc in tool_calls {
            content.push(ContentBlock::ToolUse {
                id: format!("toolu_{}", uuid_stub()),
                name: tc.tool.clone(),
                input: tc.input.clone(),
            });
        }

        let usage = usage.unwrap_or_else(|| UsageSpec {
            input_tokens: 100,
            output_tokens: estimate_tokens(&text),
        });

        let json_response = JsonResponse {
            id: format!("msg_{}", uuid_stub()),
            model: self.model.clone(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content,
            stop_reason: "end_turn".to_string(),
            usage: Usage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
            },
        };

        let json = serde_json::to_string(&json_response)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(self.writer, "{}", json)
    }

    fn write_stream_json(
        &mut self,
        response: &ResponseSpec,
        tool_calls: &[ToolCallSpec],
    ) -> std::io::Result<()> {
        let (text, usage) = match response {
            ResponseSpec::Simple(s) => (s.clone(), None),
            ResponseSpec::Detailed { text, usage, .. } => (text.clone(), usage.clone()),
        };

        let msg_id = format!("msg_{}", uuid_stub());

        // message_start
        let start = StreamEvent::MessageStart {
            message: StreamMessage {
                id: msg_id.clone(),
                model: self.model.clone(),
                message_type: "message".to_string(),
                role: "assistant".to_string(),
            },
        };
        self.write_event(&start)?;

        // content_block_start for text
        let block_start = StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::Text {
                text: String::new(),
            },
        };
        self.write_event(&block_start)?;

        // Stream text in chunks
        let chunk_size = 20;
        for chunk in text.as_bytes().chunks(chunk_size) {
            let chunk_text = String::from_utf8_lossy(chunk);
            let delta = StreamEvent::ContentBlockDelta {
                index: 0,
                delta: Delta::TextDelta {
                    text: chunk_text.to_string(),
                },
            };
            self.write_event(&delta)?;
        }

        // content_block_stop for text
        let block_stop = StreamEvent::ContentBlockStop { index: 0 };
        self.write_event(&block_stop)?;

        // Stream tool calls if any
        for (i, tc) in tool_calls.iter().enumerate() {
            let idx = (i + 1) as u32;

            // content_block_start for tool use
            let tool_start = StreamEvent::ContentBlockStart {
                index: idx,
                content_block: ContentBlock::ToolUse {
                    id: format!("toolu_{}", uuid_stub()),
                    name: tc.tool.clone(),
                    input: serde_json::Value::Object(Default::default()),
                },
            };
            self.write_event(&tool_start)?;

            // Stream the input JSON
            let input_json = serde_json::to_string(&tc.input)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let input_delta = StreamEvent::ContentBlockDelta {
                index: idx,
                delta: Delta::InputJsonDelta {
                    partial_json: input_json,
                },
            };
            self.write_event(&input_delta)?;

            let tool_stop = StreamEvent::ContentBlockStop { index: idx };
            self.write_event(&tool_stop)?;
        }

        // message_delta
        let usage = usage.unwrap_or_else(|| UsageSpec {
            input_tokens: 100,
            output_tokens: estimate_tokens(&text),
        });
        let msg_delta = StreamEvent::MessageDelta {
            delta: MessageDelta {
                stop_reason: "end_turn".to_string(),
            },
            usage: Usage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
            },
        };
        self.write_event(&msg_delta)?;

        // message_stop
        let stop = StreamEvent::MessageStop;
        self.write_event(&stop)
    }

    fn write_event(&mut self, event: &StreamEvent) -> std::io::Result<()> {
        let json = serde_json::to_string(event)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(self.writer, "{}", json)
    }

    // =========================================================================
    // Real Claude Format Methods
    // =========================================================================

    /// Write a result in Claude's result wrapper format (--output-format json)
    pub fn write_result(&mut self, result: &ResultOutput) -> std::io::Result<()> {
        let json = serde_json::to_string(result)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(self.writer, "{}", json)
    }

    /// Write a complete response in real Claude format
    pub fn write_real_response(
        &mut self,
        response: &ResponseSpec,
        session_id: &str,
        tools: Vec<String>,
    ) -> std::io::Result<()> {
        self.write_real_response_with_mcp(response, session_id, tools, vec![])
    }

    /// Write a complete response in real Claude format with MCP servers
    pub fn write_real_response_with_mcp(
        &mut self,
        response: &ResponseSpec,
        session_id: &str,
        tools: Vec<String>,
        mcp_servers: Vec<String>,
    ) -> std::io::Result<()> {
        match self.format {
            OutputFormat::Text => self.write_text(response),
            OutputFormat::Json => self.write_real_json(response, session_id),
            OutputFormat::StreamJson => {
                self.write_real_stream_json(response, session_id, tools, mcp_servers)
            }
        }
    }

    /// Write JSON in real Claude's result wrapper format
    fn write_real_json(
        &mut self,
        response: &ResponseSpec,
        session_id: &str,
    ) -> std::io::Result<()> {
        let text = match response {
            ResponseSpec::Simple(s) => s.clone(),
            ResponseSpec::Detailed { text, .. } => text.clone(),
        };

        let result = ResultOutput::success(text, session_id.to_string(), 1000);
        self.write_result(&result)
    }

    /// Write stream-JSON in real Claude format (condensed 3-event format)
    ///
    /// Real Claude CLI outputs 3 events:
    /// 1. System init event
    /// 2. Assistant event with full message (no subtype)
    /// 3. Result event
    fn write_real_stream_json(
        &mut self,
        response: &ResponseSpec,
        session_id: &str,
        tools: Vec<String>,
        mcp_servers: Vec<String>,
    ) -> std::io::Result<()> {
        let (text, usage) = match response {
            ResponseSpec::Simple(s) => (s.clone(), None),
            ResponseSpec::Detailed { text, usage, .. } => (text.clone(), usage.clone()),
        };

        let msg_id = format!("msg_{}", uuid_stub());

        // 1. System init with tools and MCP servers
        let init = SystemInitEvent::with_mcp_servers(session_id, tools, mcp_servers);
        self.write_json_line(&init)?;

        // 2. Condensed assistant event with full message content
        let usage_spec = usage.unwrap_or_else(|| UsageSpec {
            input_tokens: 100,
            output_tokens: estimate_tokens(&text),
        });
        let message = CondensedMessage {
            id: msg_id,
            model: self.model.clone(),
            role: "assistant".to_string(),
            message_type: "message".to_string(),
            content: serde_json::json!([{"type": "text", "text": text}]),
            stop_reason: None,
            stop_sequence: None,
            usage: serde_json::json!({
                "input_tokens": usage_spec.input_tokens,
                "output_tokens": usage_spec.output_tokens
            }),
        };
        let assistant = CondensedAssistantEvent::new(message, session_id);
        self.write_json_line(&assistant)?;

        // 3. Final result
        let result = ResultOutput::success(text, session_id.to_string(), 1000);
        self.write_json_line(&result)
    }

    /// Write a tool result block (for stream-json format).
    pub fn write_tool_result(
        &mut self,
        result: &crate::tools::ToolExecutionResult,
    ) -> std::io::Result<()> {
        match self.format {
            OutputFormat::Text => {
                // For text mode, just print the result
                if let Some(text) = result.text() {
                    if result.is_error {
                        writeln!(self.writer, "Error: {}", text)
                    } else {
                        writeln!(self.writer, "{}", text)
                    }
                } else {
                    Ok(())
                }
            }
            OutputFormat::Json | OutputFormat::StreamJson => {
                let block = ToolResultBlock::from_result(result);
                self.write_json_line(&block)
            }
        }
    }

    /// Write a tool result block directly.
    pub fn write_tool_result_block(&mut self, block: &ToolResultBlock) -> std::io::Result<()> {
        match self.format {
            OutputFormat::Text => {
                if let Some(ToolResultContentBlock::Text { text }) = block.content.first() {
                    if block.is_error {
                        writeln!(self.writer, "Error: {}", text)
                    } else {
                        writeln!(self.writer, "{}", text)
                    }
                } else {
                    Ok(())
                }
            }
            OutputFormat::Json | OutputFormat::StreamJson => self.write_json_line(block),
        }
    }

    /// Write a JSON-serializable object as a line
    fn write_json_line<T: serde::Serialize>(&mut self, value: &T) -> std::io::Result<()> {
        let json = serde_json::to_string(value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(self.writer, "{}", json)
    }
}

/// Generate a deterministic UUID-like stub for testing
fn uuid_stub() -> String {
    "01234567890abcdef".to_string()
}

/// Estimate token count from text (rough approximation: 4 chars per token)
fn estimate_tokens(text: &str) -> u32 {
    (text.len() / 4).max(1) as u32
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
