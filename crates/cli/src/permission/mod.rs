// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Permission handling for tool execution.
//!
//! This module provides permission checking functionality matching real Claude's
//! behavior, including:
//!
//! - Permission modes (`--permission-mode` flag)
//! - Permission bypass (`--dangerously-skip-permissions` with `--allow-dangerously-skip-permissions`)
//! - Tool-specific permission checks

pub(crate) mod bypass;
pub(crate) mod check;
pub(crate) mod mode;
pub(crate) mod pattern;

pub use bypass::PermissionBypass;
pub use check::{PermissionChecker, PermissionResult};
pub use mode::PermissionMode;
pub use pattern::{PermissionPatterns, ToolPattern};
