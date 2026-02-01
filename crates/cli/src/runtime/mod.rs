// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Runtime module for orchestrating prompt execution.
//!
//! This module provides:
//! - [`RuntimeContext`] - Merged configuration from scenario + CLI
//! - [`Runtime`] - Core runtime for executing prompts
//! - [`RuntimeBuilder`] - Fluent API for constructing Runtime instances

mod builder;
mod context;
mod core;
mod print_mode;

pub use builder::{RuntimeBuildError, RuntimeBuilder};
pub use context::RuntimeContext;
pub use core::{PendingPermission, Runtime, TurnResult};
