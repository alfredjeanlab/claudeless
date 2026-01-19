// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Validation infrastructure for comparing simulator against real Claude.

pub mod cli_audit;
pub mod output_samples;
pub mod report;

pub use cli_audit::{CliAudit, FlagDef, FlagStatus};
pub use report::{AccuracyReport, FeatureCategory, ValidationItem, ValidationStatus};
