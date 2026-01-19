// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Claude CLI Simulator
//!
//! A test crate that simulates the `claude` CLI for integration testing.
//! Provides a controllable test double that responds to the same CLI interface
//! as real Claude, enabling deterministic integration testing without API costs.

pub mod api;
pub mod capture;
pub mod cli;
pub mod config;
pub mod failure;
pub mod hooks;
pub mod inspect;
pub mod mcp;
pub mod output;
pub mod permission;
pub mod scenario;
pub mod session;
pub mod state;
pub mod time;
pub mod tools;
pub mod tui;
pub mod validation;

pub use api::{BinarySimulatorHandle, SimulatorBuilder, SimulatorHandle};
pub use capture::{CaptureLog, CapturedArgs, CapturedInteraction, CapturedOutcome};
pub use cli::{Cli, FailureMode, OutputFormat};
pub use config::{
    ConversationTurn, FailureSpec, PatternSpec, ResponseRule, ResponseSpec, ScenarioConfig,
    ToolCallSpec, ToolExecutionConfig, ToolExecutionMode, UsageSpec,
};
pub use failure::FailureExecutor;
pub use mcp::{
    load_mcp_config, McpConfig, McpConfigError, McpManager, McpServer, McpServerDef,
    McpServerStatus, McpToolCall, McpToolDef, McpToolResult, McpToolTemplates,
};
pub use output::{
    ContentBlock, JsonResponse, OutputWriter, StreamEvent, ToolResultBlock, ToolResultContentBlock,
    Usage,
};
pub use permission::{PermissionBypass, PermissionChecker, PermissionMode, PermissionResult};
pub use scenario::{MatchResult, Scenario, ScenarioError};
pub use session::SessionContext;
pub use state::StateWriter;
pub use tools::{
    create_executor, ExecutionContext, MockExecutor, ToolExecutionResult, ToolExecutor,
    ToolResultContent,
};
pub use tui::{TuiApp, TuiTestHarness};
