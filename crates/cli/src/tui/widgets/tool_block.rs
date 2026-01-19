// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool use display widget.
//!
//! Note: Rendering has been moved to app.rs as part of the iocraft migration.
//! This module keeps the types used by the app component.

/// Tool execution state
#[derive(Clone, Debug)]
pub struct ToolBlockState {
    pub tool_name: String,
    pub status: ToolStatus,
    pub input_preview: String,
    pub output_preview: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

// Note: The render function has been removed as iocraft uses declarative layout
// within the App component in app.rs.
