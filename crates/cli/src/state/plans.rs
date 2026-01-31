// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Saved plans management.

use super::io::JsonLoad;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A saved plan
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    /// Unique plan ID
    pub id: String,

    /// Plan title/name
    pub title: String,

    /// Plan content (markdown)
    pub content: String,

    /// Creation timestamp (millis since epoch)
    pub created_at_ms: u64,

    /// Last modified timestamp (millis since epoch)
    pub modified_at_ms: u64,

    /// Associated project path
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
}

impl Plan {
    /// Create a new plan
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            id: id.into(),
            title: title.into(),
            content: content.into(),
            created_at_ms: now_ms,
            modified_at_ms: now_ms,
            project_path: None,
        }
    }

    /// Create a plan with a specific timestamp (for testing)
    pub fn new_at(
        id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            content: content.into(),
            created_at_ms: timestamp_ms,
            modified_at_ms: timestamp_ms,
            project_path: None,
        }
    }

    /// Set project path
    pub fn with_project(mut self, path: impl Into<String>) -> Self {
        self.project_path = Some(path.into());
        self
    }

    /// Update content and modified timestamp
    pub fn update_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.modified_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }

    /// Get creation time as SystemTime
    pub fn created_at(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.created_at_ms)
    }

    /// Get modified time as SystemTime
    pub fn modified_at(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.modified_at_ms)
    }

    /// Save to file
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
}

impl JsonLoad for Plan {}

/// Plans manager
pub struct PlansManager {
    plans_dir: PathBuf,
}

impl PlansManager {
    /// Create a new plans manager
    pub fn new(plans_dir: impl Into<PathBuf>) -> Self {
        Self {
            plans_dir: plans_dir.into(),
        }
    }

    /// Get the plans directory
    pub fn plans_dir(&self) -> &Path {
        &self.plans_dir
    }

    /// List all plans
    pub fn list(&self) -> std::io::Result<Vec<Plan>> {
        let mut plans = Vec::new();
        if self.plans_dir.exists() {
            for entry in std::fs::read_dir(&self.plans_dir)? {
                let path = entry?.path();
                if path.extension().is_some_and(|e| e == "json") {
                    if let Ok(plan) = Plan::load(&path) {
                        plans.push(plan);
                    }
                }
            }
        }
        // Sort by modified time descending (most recent first)
        plans.sort_by(|a, b| b.modified_at_ms.cmp(&a.modified_at_ms));
        Ok(plans)
    }

    /// Get a plan by ID
    pub fn get(&self, id: &str) -> std::io::Result<Option<Plan>> {
        let path = self.plans_dir.join(format!("{}.json", id));
        if path.exists() {
            Ok(Some(Plan::load(&path)?))
        } else {
            Ok(None)
        }
    }

    /// Save a plan
    pub fn save(&self, plan: &Plan) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.plans_dir)?;
        let path = self.plans_dir.join(format!("{}.json", plan.id));
        plan.save(&path)
    }

    /// Delete a plan
    pub fn delete(&self, id: &str) -> std::io::Result<bool> {
        let path = self.plans_dir.join(format!("{}.json", id));
        if path.exists() {
            std::fs::remove_file(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check if a plan exists
    pub fn exists(&self, id: &str) -> bool {
        self.plans_dir.join(format!("{}.json", id)).exists()
    }

    /// Get plan count
    pub fn count(&self) -> std::io::Result<usize> {
        if !self.plans_dir.exists() {
            return Ok(0);
        }
        let count = std::fs::read_dir(&self.plans_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .count();
        Ok(count)
    }

    /// Create a new plan file with markdown content and generated name.
    ///
    /// Real Claude CLI uses word-based naming: `{adjective}-{verb}-{noun}.md`
    /// (e.g., `velvety-crunching-ocean.md`).
    ///
    /// Returns the generated plan name (without extension).
    pub fn create_markdown(&self, content: &str) -> std::io::Result<String> {
        use super::words::generate_plan_name;

        std::fs::create_dir_all(&self.plans_dir)?;

        // Generate unique name (retry if exists)
        let mut name = generate_plan_name();
        let mut attempts = 0;
        while self.plans_dir.join(format!("{}.md", name)).exists() && attempts < 10 {
            name = generate_plan_name();
            attempts += 1;
            // Add small delay to ensure different hash
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        let path = self.plans_dir.join(format!("{}.md", name));
        std::fs::write(&path, content)?;

        Ok(name)
    }

    /// Check if a markdown plan exists
    pub fn markdown_exists(&self, name: &str) -> bool {
        self.plans_dir.join(format!("{}.md", name)).exists()
    }
}

#[cfg(test)]
#[path = "plans_tests.rs"]
mod tests;
