// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Settings source definitions for selective loading.

use clap::ValueEnum;
use std::str::FromStr;

/// Available settings sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum SettingSource {
    /// Global/user settings (~/.claude/settings.json)
    User,
    /// Project settings (.claude/settings.json)
    Project,
    /// Local overrides (.claude/settings.local.json)
    Local,
}

impl SettingSource {
    /// Return all sources in precedence order (lowest to highest).
    pub fn all() -> &'static [SettingSource] {
        &[Self::User, Self::Project, Self::Local]
    }
}

impl FromStr for SettingSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" | "global" => Ok(Self::User),
            "project" => Ok(Self::Project),
            "local" => Ok(Self::Local),
            _ => Err(format!("unknown setting source: {s}")),
        }
    }
}

#[cfg(test)]
#[path = "settings_source_tests.rs"]
mod tests;
