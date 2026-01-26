// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
