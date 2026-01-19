// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Hook simulation module for bi-directional communication with oj.
//!
//! This module provides hook simulation matching Claude Code's hook protocol,
//! including pre/post tool execution, notifications, and permission requests.

pub mod executor;
pub mod protocol;
pub mod registry;

pub use executor::{HookConfig, HookError, HookExecutor};
pub use protocol::{HookEvent, HookMessage, HookPayload, HookResponse, NotificationLevel};
pub use registry::HookRegistry;
