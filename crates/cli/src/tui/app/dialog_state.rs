// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Dialog state for the TUI application.
//!
//! Only one dialog can be active at a time. This enum represents all possible
//! dialog states in a type-safe way.

use crate::tui::widgets::export::ExportDialog;
use crate::tui::widgets::help::HelpDialog;
use crate::tui::widgets::tasks::TasksDialog;
use crate::tui::widgets::thinking::ThinkingDialog;
use crate::tui::widgets::{HooksDialog, MemoryDialog, ModelPickerDialog};

use crate::tui::app::types::{PermissionRequest, TrustPromptState};

/// Active dialog state (only one dialog can be active at a time)
#[derive(Clone, Debug, Default)]
pub enum DialogState {
    #[default]
    None,
    Trust(TrustPromptState),
    Thinking(ThinkingDialog),
    Tasks(TasksDialog),
    Export(ExportDialog),
    Help(HelpDialog),
    Hooks(HooksDialog),
    Memory(MemoryDialog),
    ModelPicker(ModelPickerDialog),
    Permission(PermissionRequest),
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

    /// Get trust prompt state if active
    pub fn as_trust(&self) -> Option<&TrustPromptState> {
        match self {
            Self::Trust(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable trust prompt state if active
    pub fn as_trust_mut(&mut self) -> Option<&mut TrustPromptState> {
        match self {
            Self::Trust(state) => Some(state),
            _ => None,
        }
    }

    /// Get thinking dialog state if active
    pub fn as_thinking(&self) -> Option<&ThinkingDialog> {
        match self {
            Self::Thinking(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable thinking dialog state if active
    pub fn as_thinking_mut(&mut self) -> Option<&mut ThinkingDialog> {
        match self {
            Self::Thinking(state) => Some(state),
            _ => None,
        }
    }

    /// Get tasks dialog state if active
    pub fn as_tasks(&self) -> Option<&TasksDialog> {
        match self {
            Self::Tasks(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable tasks dialog state if active
    pub fn as_tasks_mut(&mut self) -> Option<&mut TasksDialog> {
        match self {
            Self::Tasks(state) => Some(state),
            _ => None,
        }
    }

    /// Get export dialog state if active
    pub fn as_export(&self) -> Option<&ExportDialog> {
        match self {
            Self::Export(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable export dialog state if active
    pub fn as_export_mut(&mut self) -> Option<&mut ExportDialog> {
        match self {
            Self::Export(state) => Some(state),
            _ => None,
        }
    }

    /// Get help dialog state if active
    pub fn as_help(&self) -> Option<&HelpDialog> {
        match self {
            Self::Help(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable help dialog state if active
    pub fn as_help_mut(&mut self) -> Option<&mut HelpDialog> {
        match self {
            Self::Help(state) => Some(state),
            _ => None,
        }
    }

    /// Get hooks dialog state if active
    pub fn as_hooks(&self) -> Option<&HooksDialog> {
        match self {
            Self::Hooks(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable hooks dialog state if active
    pub fn as_hooks_mut(&mut self) -> Option<&mut HooksDialog> {
        match self {
            Self::Hooks(state) => Some(state),
            _ => None,
        }
    }

    /// Get memory dialog state if active
    pub fn as_memory(&self) -> Option<&MemoryDialog> {
        match self {
            Self::Memory(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable memory dialog state if active
    pub fn as_memory_mut(&mut self) -> Option<&mut MemoryDialog> {
        match self {
            Self::Memory(state) => Some(state),
            _ => None,
        }
    }

    /// Get model picker dialog state if active
    pub fn as_model_picker(&self) -> Option<&ModelPickerDialog> {
        match self {
            Self::ModelPicker(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable model picker dialog state if active
    pub fn as_model_picker_mut(&mut self) -> Option<&mut ModelPickerDialog> {
        match self {
            Self::ModelPicker(state) => Some(state),
            _ => None,
        }
    }

    /// Get permission dialog state if active
    pub fn as_permission(&self) -> Option<&PermissionRequest> {
        match self {
            Self::Permission(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable permission dialog state if active
    pub fn as_permission_mut(&mut self) -> Option<&mut PermissionRequest> {
        match self {
            Self::Permission(state) => Some(state),
            _ => None,
        }
    }
}
