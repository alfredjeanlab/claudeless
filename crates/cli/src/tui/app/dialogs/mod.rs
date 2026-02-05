// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Dialog state and component modules.
//!
//! Each submodule colocates a dialog's rendering and key handling, so all
//! logic for a single dialog lives in one place.

mod boxed;
mod help;
mod panels;
mod simple;

pub(crate) use boxed::{render_export_dialog, render_tasks_dialog};
pub(crate) use help::render_help_dialog;
pub(crate) use panels::{render_hooks_dialog, render_model_picker_dialog, render_setup_wizard};
pub(crate) use simple::{
    render_bypass_confirm_dialog, render_elicitation_dialog, render_memory_dialog,
    render_permission_dialog, render_plan_approval_dialog, render_thinking_dialog,
    render_trust_prompt,
};

use crate::tui::widgets::elicitation::ElicitationState;
use crate::tui::widgets::export::ExportDialog;
use crate::tui::widgets::help::HelpDialog;
use crate::tui::widgets::plan_approval::PlanApprovalState;
use crate::tui::widgets::setup::SetupState;
use crate::tui::widgets::tasks::TasksDialog;
use crate::tui::widgets::thinking::ThinkingDialog;
use crate::tui::widgets::{HooksDialog, MemoryDialog, ModelPickerDialog};

use super::types::{BypassConfirmState, PermissionRequest, TrustPromptState};

// ── Shared helpers for list-style dialogs ───────────────────────────────

pub(super) fn cursor(selected: bool) -> &'static str {
    if selected {
        " ❯ "
    } else {
        "   "
    }
}

pub(super) fn check(active: bool) -> &'static str {
    if active {
        " ✔"
    } else {
        ""
    }
}

pub(super) struct SelectionList<'a> {
    labels: &'a [&'a str],
    descriptions: &'a [&'a str],
    selected: usize,
    current: Option<usize>,
}

impl<'a> SelectionList<'a> {
    pub fn new(labels: &'a [&'a str]) -> Self {
        Self {
            labels,
            descriptions: &[],
            selected: 0,
            current: None,
        }
    }

    pub fn descriptions(mut self, descs: &'a [&'a str]) -> Self {
        self.descriptions = descs;
        self
    }

    pub fn selected(mut self, idx: usize) -> Self {
        self.selected = idx;
        self
    }

    pub fn current(mut self, idx: usize) -> Self {
        self.current = Some(idx);
        self
    }

    /// Format all option lines as strings.
    pub fn lines(&self) -> Vec<String> {
        self.labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let desc = self.descriptions.get(i).copied().unwrap_or("");
                let is_current = self.current == Some(i);
                let suffix = if desc.is_empty() {
                    check(is_current).to_string()
                } else if is_current {
                    format!("{}  {}", check(true), desc)
                } else {
                    format!("   {}", desc)
                };
                format!(
                    "{}{}. {}{}",
                    cursor(i == self.selected),
                    i + 1,
                    label,
                    suffix,
                )
            })
            .collect()
    }
}

/// Active dialog state (only one dialog can be active at a time)
#[derive(Clone, Debug, Default)]
pub enum DialogState {
    #[default]
    None,
    Setup(SetupState),
    Trust(TrustPromptState),
    BypassConfirm(BypassConfirmState),
    Thinking(ThinkingDialog),
    Tasks(TasksDialog),
    Export(ExportDialog),
    Help(HelpDialog),
    Hooks(HooksDialog),
    Memory(MemoryDialog),
    ModelPicker(ModelPickerDialog),
    Permission(PermissionRequest),
    Elicitation(ElicitationState),
    PlanApproval(PlanApprovalState),
}

macro_rules! dialog_accessor {
    ($name:ident, $name_mut:ident, $variant:ident, $ty:ty) => {
        pub fn $name(&self) -> Option<&$ty> {
            match self {
                Self::$variant(s) => Some(s),
                _ => None,
            }
        }
        pub fn $name_mut(&mut self) -> Option<&mut $ty> {
            match self {
                Self::$variant(s) => Some(s),
                _ => None,
            }
        }
    };
}

impl DialogState {
    /// Check if any dialog is active
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Dismiss the current dialog
    pub fn dismiss(&mut self) {
        *self = Self::None;
    }

    dialog_accessor!(as_setup, as_setup_mut, Setup, SetupState);
    dialog_accessor!(as_trust, as_trust_mut, Trust, TrustPromptState);
    dialog_accessor!(
        as_bypass_confirm,
        as_bypass_confirm_mut,
        BypassConfirm,
        BypassConfirmState
    );
    dialog_accessor!(as_thinking, as_thinking_mut, Thinking, ThinkingDialog);
    dialog_accessor!(as_tasks, as_tasks_mut, Tasks, TasksDialog);
    dialog_accessor!(as_export, as_export_mut, Export, ExportDialog);
    dialog_accessor!(as_help, as_help_mut, Help, HelpDialog);
    dialog_accessor!(as_hooks, as_hooks_mut, Hooks, HooksDialog);
    dialog_accessor!(as_memory, as_memory_mut, Memory, MemoryDialog);
    dialog_accessor!(
        as_model_picker,
        as_model_picker_mut,
        ModelPicker,
        ModelPickerDialog
    );
    dialog_accessor!(
        as_permission,
        as_permission_mut,
        Permission,
        PermissionRequest
    );
    dialog_accessor!(
        as_elicitation,
        as_elicitation_mut,
        Elicitation,
        ElicitationState
    );
    dialog_accessor!(
        as_plan_approval,
        as_plan_approval_mut,
        PlanApproval,
        PlanApprovalState
    );
}
