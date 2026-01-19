// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Accuracy reporting for simulator validation.

use super::cli_audit::CliAudit;
use std::collections::BTreeMap;

/// Validation status for a feature
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationStatus {
    /// Matches real Claude behavior
    Match,
    /// Partial match with known differences
    Partial(String),
    /// Does not match (needs fixing)
    Mismatch(String),
    /// Not validated yet
    NotValidated,
    /// Intentionally different (by design)
    IntentionalDifference(String),
}

/// Feature category for organization
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeatureCategory {
    CliFlags,
    OutputFormats,
    HookProtocol,
    StateDirectory,
    ErrorBehavior,
}

impl std::fmt::Display for FeatureCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureCategory::CliFlags => write!(f, "CLI Flags"),
            FeatureCategory::OutputFormats => write!(f, "Output Formats"),
            FeatureCategory::HookProtocol => write!(f, "Hook Protocol"),
            FeatureCategory::StateDirectory => write!(f, "State Directory"),
            FeatureCategory::ErrorBehavior => write!(f, "Error Behavior"),
        }
    }
}

/// A validation item with status
#[derive(Clone, Debug)]
pub struct ValidationItem {
    pub name: String,
    pub category: FeatureCategory,
    pub status: ValidationStatus,
    pub notes: Option<String>,
}

/// Accuracy report for the simulator
pub struct AccuracyReport {
    items: Vec<ValidationItem>,
    validated_date: Option<String>,
    claude_version: Option<String>,
}

impl AccuracyReport {
    /// Create a new accuracy report
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            validated_date: None,
            claude_version: None,
        }
    }

    /// Set validation date
    pub fn with_date(mut self, date: impl Into<String>) -> Self {
        self.validated_date = Some(date.into());
        self
    }

    /// Set Claude version validated against
    pub fn with_claude_version(mut self, version: impl Into<String>) -> Self {
        self.claude_version = Some(version.into());
        self
    }

    /// Add a validation item
    pub fn add_item(&mut self, item: ValidationItem) {
        self.items.push(item);
    }

    /// Add CLI flag validation from audit
    pub fn add_cli_flags(&mut self, audit: &CliAudit) {
        use super::cli_audit::FlagStatus;

        for flag in audit.all_flags() {
            let status = match &flag.status {
                FlagStatus::Implemented => ValidationStatus::Match,
                FlagStatus::Partial(note) => ValidationStatus::Partial(note.clone()),
                FlagStatus::MissingNeeded => {
                    ValidationStatus::Mismatch("Not implemented".to_string())
                }
                FlagStatus::MissingLowPriority => ValidationStatus::NotValidated,
                FlagStatus::NotSupported(reason) => {
                    ValidationStatus::IntentionalDifference(reason.clone())
                }
            };

            self.add_item(ValidationItem {
                name: format!("--{}", flag.name),
                category: FeatureCategory::CliFlags,
                status,
                notes: Some(flag.description.to_string()),
            });
        }
    }

    /// Get all items by category
    pub fn items_by_category(&self) -> BTreeMap<FeatureCategory, Vec<&ValidationItem>> {
        let mut map: BTreeMap<FeatureCategory, Vec<&ValidationItem>> = BTreeMap::new();
        for item in &self.items {
            map.entry(item.category.clone()).or_default().push(item);
        }
        map
    }

    /// Count items by status
    pub fn count_by_status(&self) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        counts.insert("match", 0);
        counts.insert("partial", 0);
        counts.insert("mismatch", 0);
        counts.insert("not_validated", 0);
        counts.insert("intentional", 0);

        for item in &self.items {
            let key = match &item.status {
                ValidationStatus::Match => "match",
                ValidationStatus::Partial(_) => "partial",
                ValidationStatus::Mismatch(_) => "mismatch",
                ValidationStatus::NotValidated => "not_validated",
                ValidationStatus::IntentionalDifference(_) => "intentional",
            };
            if let Some(count) = counts.get_mut(key) {
                *count += 1;
            }
        }

        counts
    }

    /// Generate markdown report
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Claudelessulator Accuracy Report\n\n");

        if let Some(date) = &self.validated_date {
            md.push_str(&format!("Last validated: {}\n", date));
        }
        if let Some(version) = &self.claude_version {
            md.push_str(&format!("Claude Code version: {}\n", version));
        }
        md.push('\n');

        // Summary
        md.push_str("## Summary\n\n");
        let counts = self.count_by_status();
        md.push_str(&format!("- Match: {}\n", counts["match"]));
        md.push_str(&format!("- Partial: {}\n", counts["partial"]));
        md.push_str(&format!("- Mismatch: {}\n", counts["mismatch"]));
        md.push_str(&format!("- Not Validated: {}\n", counts["not_validated"]));
        md.push_str(&format!(
            "- Intentional Difference: {}\n",
            counts["intentional"]
        ));
        md.push('\n');

        // By category
        for (category, items) in self.items_by_category() {
            md.push_str(&format!("## {}\n\n", category));
            md.push_str("| Feature | Status | Notes |\n");
            md.push_str("|---------|--------|-------|\n");

            for item in items {
                let (status_icon, status_note) = match &item.status {
                    ValidationStatus::Match => ("✅", String::new()),
                    ValidationStatus::Partial(note) => ("⚠️", format!(" ({})", note)),
                    ValidationStatus::Mismatch(note) => ("❌", format!(" ({})", note)),
                    ValidationStatus::NotValidated => ("❓", String::new()),
                    ValidationStatus::IntentionalDifference(note) => ("🔵", format!(" ({})", note)),
                };

                let notes = item.notes.as_deref().unwrap_or("");
                md.push_str(&format!(
                    "| `{}` | {}{} | {} |\n",
                    item.name, status_icon, status_note, notes
                ));
            }
            md.push('\n');
        }

        md
    }
}

impl Default for AccuracyReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
