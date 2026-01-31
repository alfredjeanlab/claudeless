// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Output format handling for text, JSON, and streaming JSON modes.

use crate::cli::OutputFormat;
use crate::config::{ResponseSpec, ToolCallSpec, UsageSpec};
use crate::state::{to_io_json, ContentBlock};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[path = "output_diagnostic.rs"]
mod output_diagnostic;
pub use output_diagnostic::{
    print_error, print_mcp, print_mcp_error, print_mcp_warning, print_warning,
};

#[path = "output_events.rs"]
mod output_events;
pub use output_events::{
    AssistantEvent, AssistantMessageContent, CondensedAssistantEvent, CondensedMessage,
    ContentBlockDeltaEvent, ContentBlockStartEvent, ContentBlockStopEvent, ExtendedUsage,
    McpServerInfo, SystemInitEvent,
};

/// Detailed usage statistics for result output
pub use crate::usage::UsageWithCost as ResultUsage;

/// Per-model usage breakdown
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModelUsage {
    #[serde(flatten)]
    pub models: std::collections::HashMap<String, ResultUsage>,
}

/// Result wrapper for JSON output matching real Claude's `--output-format json`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultOutput {
    #[serde(rename = "type")]
    pub output_type: String,
    pub subtype: String,
    pub cost_usd: f64,
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
    pub model_usage: ModelUsage,
    pub usage: ResultUsage,
    pub permission_denials: Vec<String>,
}

impl ResultOutput {
    /// Create a base result with common defaults.
    fn base(session_id: String) -> Self {
        Self {
            output_type: "result".to_string(),
            subtype: "success".to_string(),
            cost_usd: 0.0,
            is_error: false,
            duration_ms: 0,
            duration_api_ms: 0,
            num_turns: 0,
            result: None,
            error: None,
            session_id,
            uuid: uuid_stub(),
            retry_after: None,
            model_usage: ModelUsage::default(),
            usage: ResultUsage::from_tokens(0, 0),
            permission_denials: vec![],
        }
    }

    /// Create a success result with usage based on response
    pub fn success(result: String, session_id: String, duration_ms: u64) -> Self {
        Self::success_with_usage(
            result,
            session_id,
            duration_ms,
            100,
            0, // Will be estimated below
            "claude-opus-4-5-20251101",
        )
    }

    /// Create a success result with custom usage
    pub fn success_with_usage(
        result: String,
        session_id: String,
        duration_ms: u64,
        input_tokens: u32,
        output_tokens: u32,
        model: &str,
    ) -> Self {
        let output_tokens = if output_tokens == 0 {
            estimate_tokens(&result)
        } else {
            output_tokens
        };
        let usage = ResultUsage::from_tokens(input_tokens, output_tokens);
        let mut model_usage = ModelUsage::default();
        model_usage.models.insert(
            model.to_string(),
            ResultUsage::from_tokens(input_tokens, output_tokens),
        );

        Self {
            cost_usd: usage.cost_usd,
            duration_ms,
            duration_api_ms: duration_ms.saturating_sub(50),
            num_turns: 1,
            result: Some(result),
            model_usage,
            usage,
            ..Self::base(session_id)
        }
    }

    /// Create an error result
    pub fn error(error: String, session_id: String, duration_ms: u64) -> Self {
        Self {
            subtype: "error".to_string(),
            is_error: true,
            duration_ms,
            duration_api_ms: duration_ms.saturating_sub(10),
            error: Some(error),
            ..Self::base(session_id)
        }
    }

    /// Create a rate limit error result
    pub fn rate_limit(retry_after: u64, session_id: String) -> Self {
        Self {
            subtype: "error".to_string(),
            is_error: true,
            duration_ms: 50,
            duration_api_ms: 50,
            error: Some(format!(
                "Rate limited. Retry after {} seconds.",
                retry_after
            )),
            retry_after: Some(retry_after),
            ..Self::base(session_id)
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

/// Token usage statistics
pub use crate::usage::TokenCounts as Usage;

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
    crate::impl_tool_result_factories!([ToolResultContentBlock::Text], block_type: "tool_result".to_string());

    /// Create from a ToolExecutionResult.
    pub fn from_result(result: &crate::tools::result::ToolExecutionResult) -> Self {
        Self {
            block_type: "tool_result".to_string(),
            tool_use_id: result.tool_use_id.clone(),
            is_error: result.is_error,
            content: result.content.clone(),
        }
    }
}

/// Content block within a tool result (alias for ToolResultContent).
pub use crate::tools::ToolResultContent as ToolResultContentBlock;

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
        let (text, usage) = response.text_and_usage();

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

        let json = to_io_json(&json_response)?;
        writeln!(self.writer, "{}", json)
    }

    fn write_stream_json(
        &mut self,
        response: &ResponseSpec,
        tool_calls: &[ToolCallSpec],
    ) -> std::io::Result<()> {
        let (text, usage) = response.text_and_usage();

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
            let input_json = to_io_json(&tc.input)?;
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
        let json = to_io_json(event)?;
        writeln!(self.writer, "{}", json)
    }

    // =========================================================================
    // Real Claude Format Methods
    // =========================================================================

    /// Write a result in Claude's result wrapper format (--output-format json)
    pub fn write_result(&mut self, result: &ResultOutput) -> std::io::Result<()> {
        let json = to_io_json(result)?;
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
        mcp_servers: Vec<McpServerInfo>,
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
        let (text, usage_spec) = response.text_and_usage();

        let result = if let Some(usage) = usage_spec {
            ResultOutput::success_with_usage(
                text,
                session_id.to_string(),
                1000,
                usage.input_tokens,
                usage.output_tokens,
                &self.model,
            )
        } else {
            ResultOutput::success(text, session_id.to_string(), 1000)
        };

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
        mcp_servers: Vec<McpServerInfo>,
    ) -> std::io::Result<()> {
        let (text, usage) = response.text_and_usage();

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

        // 3. Final result with usage
        let result = ResultOutput::success_with_usage(
            text,
            session_id.to_string(),
            1000,
            usage_spec.input_tokens,
            usage_spec.output_tokens,
            &self.model,
        );
        self.write_json_line(&result)
    }

    /// Write a tool result block (for stream-json format).
    pub fn write_tool_result(
        &mut self,
        result: &crate::tools::result::ToolExecutionResult,
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
        writeln!(self.writer, "{}", to_io_json(value)?)
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
