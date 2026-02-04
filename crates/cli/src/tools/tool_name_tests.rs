// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn as_str_returns_correct_string() {
    assert_eq!(ToolName::Bash.as_str(), "Bash");
    assert_eq!(ToolName::Read.as_str(), "Read");
    assert_eq!(ToolName::Write.as_str(), "Write");
    assert_eq!(ToolName::Edit.as_str(), "Edit");
    assert_eq!(ToolName::Glob.as_str(), "Glob");
    assert_eq!(ToolName::Grep.as_str(), "Grep");
    assert_eq!(ToolName::TodoWrite.as_str(), "TodoWrite");
    assert_eq!(ToolName::ExitPlanMode.as_str(), "ExitPlanMode");
    assert_eq!(ToolName::WebFetch.as_str(), "WebFetch");
    assert_eq!(ToolName::WebSearch.as_str(), "WebSearch");
    assert_eq!(ToolName::NotebookEdit.as_str(), "NotebookEdit");
    assert_eq!(ToolName::Task.as_str(), "Task");
    assert_eq!(ToolName::AskUserQuestion.as_str(), "AskUserQuestion");
}

#[test]
fn parse_parses_valid_names() {
    assert_eq!(ToolName::parse("Bash"), Some(ToolName::Bash));
    assert_eq!(ToolName::parse("Read"), Some(ToolName::Read));
    assert_eq!(ToolName::parse("Write"), Some(ToolName::Write));
    assert_eq!(ToolName::parse("Edit"), Some(ToolName::Edit));
    assert_eq!(ToolName::parse("Glob"), Some(ToolName::Glob));
    assert_eq!(ToolName::parse("Grep"), Some(ToolName::Grep));
    assert_eq!(ToolName::parse("TodoWrite"), Some(ToolName::TodoWrite));
    assert_eq!(
        ToolName::parse("ExitPlanMode"),
        Some(ToolName::ExitPlanMode)
    );
    assert_eq!(ToolName::parse("WebFetch"), Some(ToolName::WebFetch));
    assert_eq!(ToolName::parse("WebSearch"), Some(ToolName::WebSearch));
    assert_eq!(
        ToolName::parse("NotebookEdit"),
        Some(ToolName::NotebookEdit)
    );
    assert_eq!(ToolName::parse("Task"), Some(ToolName::Task));
    assert_eq!(
        ToolName::parse("AskUserQuestion"),
        Some(ToolName::AskUserQuestion)
    );
}

#[test]
fn parse_returns_none_for_unknown() {
    assert_eq!(ToolName::parse("Unknown"), None);
    assert_eq!(ToolName::parse("bash"), None);
    assert_eq!(ToolName::parse(""), None);
}

#[test]
fn action_returns_correct_permission_type() {
    assert_eq!(ToolName::Bash.action(), "execute");
    assert_eq!(ToolName::Read.action(), "read");
    assert_eq!(ToolName::Glob.action(), "read");
    assert_eq!(ToolName::Grep.action(), "read");
    assert_eq!(ToolName::Write.action(), "write");
    assert_eq!(ToolName::Edit.action(), "write");
    assert_eq!(ToolName::NotebookEdit.action(), "write");
    assert_eq!(ToolName::WebFetch.action(), "network");
    assert_eq!(ToolName::WebSearch.action(), "network");
    assert_eq!(ToolName::Task.action(), "delegate");
    assert_eq!(ToolName::TodoWrite.action(), "state");
    assert_eq!(ToolName::ExitPlanMode.action(), "state");
    assert_eq!(ToolName::AskUserQuestion.action(), "state");
}

#[test]
fn display_impl() {
    assert_eq!(format!("{}", ToolName::Bash), "Bash");
    assert_eq!(format!("{}", ToolName::TodoWrite), "TodoWrite");
}

#[test]
fn roundtrip_all_variants() {
    let variants = [
        ToolName::Bash,
        ToolName::Read,
        ToolName::Write,
        ToolName::Edit,
        ToolName::Glob,
        ToolName::Grep,
        ToolName::TodoWrite,
        ToolName::ExitPlanMode,
        ToolName::WebFetch,
        ToolName::WebSearch,
        ToolName::NotebookEdit,
        ToolName::Task,
        ToolName::AskUserQuestion,
    ];
    for variant in variants {
        let s = variant.as_str();
        assert_eq!(ToolName::parse(s), Some(variant));
    }
}
