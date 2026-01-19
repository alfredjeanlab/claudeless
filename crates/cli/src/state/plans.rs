// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Saved plans management.

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

    /// Load from file
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save to file
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
}

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
mod tests {
    use super::*;

    #[test]
    fn test_new_plan() {
        let plan = Plan::new("plan_1", "My Plan", "## Steps\n1. Do this\n2. Do that");

        assert_eq!(plan.id, "plan_1");
        assert_eq!(plan.title, "My Plan");
        assert!(plan.content.contains("Steps"));
        assert!(plan.project_path.is_none());
    }

    #[test]
    fn test_plan_with_project() {
        let plan = Plan::new("plan_1", "My Plan", "content").with_project("/some/project");

        assert_eq!(plan.project_path, Some("/some/project".to_string()));
    }

    #[test]
    fn test_update_content() {
        let mut plan = Plan::new_at("plan_1", "Title", "Original", 1000);

        plan.update_content("Updated content");

        assert_eq!(plan.content, "Updated content");
        assert!(plan.modified_at_ms > plan.created_at_ms);
    }

    #[test]
    fn test_plan_save_load() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("plan.json");

        let plan = Plan::new("test_plan", "Test", "Content here");
        plan.save(&path).unwrap();

        let loaded = Plan::load(&path).unwrap();
        assert_eq!(loaded.id, "test_plan");
        assert_eq!(loaded.title, "Test");
        assert_eq!(loaded.content, "Content here");
    }

    #[test]
    fn test_plans_manager_save_get() {
        let temp = tempfile::tempdir().unwrap();
        let manager = PlansManager::new(temp.path());

        let plan = Plan::new("plan_1", "First Plan", "Content");
        manager.save(&plan).unwrap();

        let loaded = manager.get("plan_1").unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().title, "First Plan");
    }

    #[test]
    fn test_plans_manager_list() {
        let temp = tempfile::tempdir().unwrap();
        let manager = PlansManager::new(temp.path());

        // Save plans with different timestamps
        let plan1 = Plan::new_at("plan_1", "Older", "Content", 1000);
        let plan2 = Plan::new_at("plan_2", "Newer", "Content", 2000);

        manager.save(&plan1).unwrap();
        manager.save(&plan2).unwrap();

        let plans = manager.list().unwrap();
        assert_eq!(plans.len(), 2);
        // Should be sorted by modified time descending
        assert_eq!(plans[0].id, "plan_2");
        assert_eq!(plans[1].id, "plan_1");
    }

    #[test]
    fn test_plans_manager_delete() {
        let temp = tempfile::tempdir().unwrap();
        let manager = PlansManager::new(temp.path());

        let plan = Plan::new("plan_1", "Title", "Content");
        manager.save(&plan).unwrap();

        assert!(manager.exists("plan_1"));
        assert!(manager.delete("plan_1").unwrap());
        assert!(!manager.exists("plan_1"));
        assert!(!manager.delete("plan_1").unwrap()); // Already deleted
    }

    #[test]
    fn test_plans_manager_count() {
        let temp = tempfile::tempdir().unwrap();
        let manager = PlansManager::new(temp.path());

        assert_eq!(manager.count().unwrap(), 0);

        manager.save(&Plan::new("p1", "T1", "C")).unwrap();
        manager.save(&Plan::new("p2", "T2", "C")).unwrap();

        assert_eq!(manager.count().unwrap(), 2);
    }

    #[test]
    fn test_plans_manager_empty_dir() {
        let temp = tempfile::tempdir().unwrap();
        let manager = PlansManager::new(temp.path().join("nonexistent"));

        let plans = manager.list().unwrap();
        assert!(plans.is_empty());

        assert!(manager.get("anything").unwrap().is_none());
    }
}
