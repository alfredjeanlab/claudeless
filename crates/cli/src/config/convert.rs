// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Conversion from v1 TOML types to canonical types.

use super::types::{
    ClaudeConfig, Pattern, Response, ResponseRule, ScenarioConfig, ToolCall, ToolConfig,
    ToolsConfig, Turn,
};
use super::v1;

impl From<v1::ScenarioConfig> for ScenarioConfig {
    fn from(v1: v1::ScenarioConfig) -> Self {
        Self {
            claude: ClaudeConfig {
                session_id: v1.identity.session_id,
                project_path: v1.environment.project_path,
                launch_timestamp: v1.timing.launch_timestamp,
                username: v1.identity.user_name,
                version: v1.identity.claude_version,
                model: v1.identity.default_model,
                provider: v1.identity.provider,
                placeholder: v1.identity.placeholder,
                working_directory: v1.environment.working_directory,
                show_welcome_back: v1.identity.show_welcome_back,
                welcome_back_right_panel: v1.identity.welcome_back_right_panel,
                timeouts: v1.timing.timeouts,
            },
            trusted: v1.environment.trusted,
            logged_in: v1.environment.logged_in,
            permission_mode: v1.environment.permission_mode,
            default: v1.default_response.map(convert_response_spec),
            responses: v1.responses.into_iter().map(convert_response_rule).collect(),
            tools: v1.tool_execution.map(convert_tool_execution),
        }
    }
}

fn convert_pattern(spec: v1::PatternSpec) -> Pattern {
    match spec {
        v1::PatternSpec::Any => Pattern::Glob("*".to_string()),
        v1::PatternSpec::Exact { text } => Pattern::Glob(text),
        v1::PatternSpec::Glob { pattern } => Pattern::Glob(pattern),
        v1::PatternSpec::Contains { text } => Pattern::Contains(text),
        v1::PatternSpec::Regex { pattern } => Pattern::Regexp(pattern),
    }
}

fn convert_response_spec(spec: v1::ResponseSpec) -> Response {
    match spec {
        v1::ResponseSpec::Simple(s) => Response { say: Some(s), ..Default::default() },
        v1::ResponseSpec::Detailed { text, tool_calls, usage, delay_ms } => Response {
            say: Some(text),
            tools: tool_calls.into_iter().map(convert_tool_call).collect(),
            usage,
            delay_ms,
        },
    }
}

fn convert_response_rule(rule: v1::ResponseRule) -> ResponseRule {
    let response = rule.response.map(convert_response_spec);
    ResponseRule {
        on: convert_pattern(rule.pattern),
        say: response.as_ref().and_then(|r| r.say.clone()),
        tools: response.as_ref().map(|r| r.tools.clone()).unwrap_or_default(),
        usage: response.as_ref().and_then(|r| r.usage.clone()),
        delay_ms: response.as_ref().and_then(|r| r.delay_ms),
        failure: rule.failure,
        max: rule.max_matches,
        then: rule.turns.into_iter().map(convert_turn).collect(),
    }
}

fn convert_turn(turn: v1::ConversationTurn) -> Turn {
    let response = convert_response_spec(turn.response);
    Turn {
        on: convert_pattern(turn.expect),
        say: response.say,
        tools: response.tools,
        usage: response.usage,
        delay_ms: response.delay_ms,
        failure: turn.failure,
    }
}

fn convert_tool_call(spec: v1::ToolCallSpec) -> ToolCall {
    ToolCall { call: spec.tool, input: spec.input, result: spec.result }
}

fn convert_tool_execution(te: v1::ToolExecutionConfig) -> ToolsConfig {
    ToolsConfig {
        mode: te.mode,
        per_tool: te.tools.into_iter().map(|(k, v)| (k, convert_tool_config(v))).collect(),
    }
}

fn convert_tool_config(tc: v1::ToolConfig) -> ToolConfig {
    ToolConfig { approve: tc.auto_approve, result: tc.result, error: tc.error, answers: tc.answers }
}

#[cfg(test)]
#[path = "convert_tests.rs"]
mod tests;
