// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI widget components.
//!
//! Note: Rendering has been moved to app.rs as part of the iocraft migration.
//! This module provides the type definitions used by app.rs.

pub mod context;
pub mod export;
pub mod help;
pub mod hooks;
pub mod memory;
pub mod model_picker;
pub mod permission;
pub mod scrollable;
pub mod tasks;
pub mod thinking;
pub mod trust;

pub use hooks::{HookType, HooksDialog, HooksView};
pub use memory::MemoryDialog;
pub use model_picker::{ModelChoice, ModelPickerDialog};
