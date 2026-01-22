// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI widget components.
//!
//! Note: Rendering has been moved to app.rs as part of the iocraft migration.
//! This module provides the type definitions used by app.rs.

pub mod permission;
pub mod tasks;
pub mod thinking;
pub mod tool_block;
pub mod trust;

pub use permission::{
    DiffKind, DiffLine, PermissionSelection, PermissionType, RichPermissionDialog,
};
pub use tasks::TasksDialog;
pub use thinking::{ThinkingDialog, ThinkingMode};
pub use tool_block::{ToolBlockState, ToolStatus};
pub use trust::{TrustChoice, TrustPrompt};
