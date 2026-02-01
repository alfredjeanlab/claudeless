// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Permission handling for tool execution.
//!
//! Contains:
//! - Permission confirmation and dialog display
//! - Session-level permission grants
//! - Tool use content building for JSONL recording

use std::sync::Arc;

use parking_lot::RwLock;

use crate::state::{ContentBlock, StateWriter};
use crate::tui::widgets::permission::{
    DiffLine, PermissionSelection, PermissionType, RichPermissionDialog,
};

use super::super::state::{DialogState, TuiAppState, TuiAppStateInner};
use super::super::types::{AppMode, PermissionRequest};

/// Get state writer from runtime if available.
fn get_state_writer(inner: &TuiAppStateInner) -> Option<Arc<RwLock<StateWriter>>> {
    inner.runtime.as_ref().and_then(|r| r.state_writer())
}

impl TuiAppState {
    /// Confirm the current permission selection
    pub(in crate::tui::app) fn confirm_permission(&self) {
        let mut inner = self.inner.lock();

        // Extract the permission from dialog state
        let perm = if let DialogState::Permission(p) = std::mem::take(&mut inner.dialog) {
            Some(p)
        } else {
            None
        };
        inner.mode = AppMode::Input;

        if let Some(perm) = perm {
            let tool_name = match &perm.dialog.permission_type {
                PermissionType::Bash { command, .. } => format!("Bash: {}", command),
                PermissionType::Edit { file_path, .. } => format!("Edit: {}", file_path),
                PermissionType::Write { file_path, .. } => format!("Write: {}", file_path),
            };

            let granted = matches!(
                perm.dialog.selected,
                PermissionSelection::Yes | PermissionSelection::YesSession
            );

            // Record tool result to JSONL
            let state_writer = get_state_writer(&inner);
            if let (Some(ref writer), Some(ref assistant_uuid), Some(ref tool_use_id)) = (
                &state_writer,
                &inner.display.pending_assistant_uuid,
                &perm.tool_use_id,
            ) {
                let (result_content, result_json) = if granted {
                    let content = format!("[Permission granted for {}]", tool_name);
                    (content, serde_json::json!({"success": true}))
                } else {
                    let content = format!("[Permission denied for {}]", tool_name);
                    (
                        content,
                        serde_json::json!({"success": false, "denied": true}),
                    )
                };

                let _ = writer.write().record_tool_result(
                    tool_use_id,
                    &result_content,
                    assistant_uuid,
                    result_json,
                );
            }
            inner.display.pending_assistant_uuid = None;

            match perm.dialog.selected {
                PermissionSelection::Yes => {
                    // Continue with tool execution (single request)
                    inner
                        .display
                        .response_content
                        .push_str(&format!("\n[Permission granted for {}]\n", tool_name));
                }
                PermissionSelection::YesSession => {
                    // Store session-level grant
                    let key = perm.dialog.session_key();
                    inner.session_grants.insert(key);

                    // Continue with tool execution (session-level grant)
                    inner.display.response_content.push_str(&format!(
                        "\n[Permission granted for session: {}]\n",
                        tool_name
                    ));
                }
                PermissionSelection::No => {
                    inner
                        .display
                        .response_content
                        .push_str(&format!("\n[Permission denied for {}]\n", tool_name));
                }
            }
        }
    }

    /// Show a permission request with rich dialog
    pub fn show_permission_request(&self, permission_type: PermissionType) {
        let tool_name = match &permission_type {
            PermissionType::Bash { command, .. } => format!("Bash: {}", command),
            PermissionType::Edit { file_path, .. } => format!("Edit: {}", file_path),
            PermissionType::Write { file_path, .. } => format!("Write: {}", file_path),
        };

        // Check if bypass mode is enabled - auto-approve all permissions
        {
            let inner = self.inner.lock();
            if inner.permission_mode.allows_all() {
                // Record tool_use and immediate result for bypass mode
                let state_writer = get_state_writer(&inner);
                if let Some(ref writer) = state_writer {
                    if let Some(ref user_uuid) = inner.display.pending_user_uuid {
                        let (tool_use_id, content) = build_tool_use_content(&permission_type);
                        if let Ok(assistant_uuid) =
                            writer.write().record_assistant_tool_use(user_uuid, content)
                        {
                            let result_content =
                                format!("[Permission auto-granted (bypass): {}]", tool_name);
                            let _ = writer.write().record_tool_result(
                                &tool_use_id,
                                &result_content,
                                &assistant_uuid,
                                serde_json::json!({"success": true, "auto_granted": true}),
                            );
                        }
                    }
                }

                drop(inner);
                simulate_permission_accept(self, &permission_type, &tool_name);
                return;
            }
        }

        // Check if this permission type is already granted for the session
        if self.is_session_granted(&permission_type) {
            // Auto-approve without showing dialog
            let mut inner = self.inner.lock();

            // Record tool_use and immediate result for session grant
            let state_writer = get_state_writer(&inner);
            if let Some(ref writer) = state_writer {
                if let Some(ref user_uuid) = inner.display.pending_user_uuid {
                    let (tool_use_id, content) = build_tool_use_content(&permission_type);
                    if let Ok(assistant_uuid) =
                        writer.write().record_assistant_tool_use(user_uuid, content)
                    {
                        let result_content =
                            format!("[Permission auto-granted (session): {}]", tool_name);
                        let _ = writer.write().record_tool_result(
                            &tool_use_id,
                            &result_content,
                            &assistant_uuid,
                            serde_json::json!({"success": true, "auto_granted": true}),
                        );
                    }
                }
            }

            inner.display.response_content.push_str(&format!(
                "\n[Permission auto-granted (session): {}]\n",
                tool_name
            ));
            return;
        }

        // Show dialog as normal - record tool_use message first
        let mut inner = self.inner.lock();

        // Get state_writer from runtime and user_uuid
        let writer_opt = get_state_writer(&inner);
        let user_uuid_opt = inner.display.pending_user_uuid.clone();

        let (tool_use_id, assistant_uuid) =
            if let (Some(ref writer), Some(ref user_uuid)) = (&writer_opt, &user_uuid_opt) {
                let (tool_use_id, content) = build_tool_use_content(&permission_type);
                match writer.write().record_assistant_tool_use(user_uuid, content) {
                    Ok(uuid) => (Some(tool_use_id), Some(uuid)),
                    Err(_) => (None, None),
                }
            } else {
                (None, None)
            };

        if let Some(uuid) = assistant_uuid {
            inner.display.pending_assistant_uuid = Some(uuid);
        }

        inner.dialog = DialogState::Permission(PermissionRequest {
            dialog: RichPermissionDialog::new(permission_type),
            tool_use_id,
        });
        inner.mode = AppMode::Permission;
    }

    /// Show a bash command permission request
    pub fn show_bash_permission(&self, command: String, description: Option<String>) {
        self.show_permission_request(PermissionType::Bash {
            command,
            description,
        });
    }

    /// Show an edit file permission request
    pub fn show_edit_permission(&self, file_path: String, diff_lines: Vec<DiffLine>) {
        self.show_permission_request(PermissionType::Edit {
            file_path,
            diff_lines,
        });
    }

    /// Show a write file permission request
    pub fn show_write_permission(&self, file_path: String, content_lines: Vec<String>) {
        self.show_permission_request(PermissionType::Write {
            file_path,
            content_lines,
        });
    }
}

/// Simulate accepting a permission (for bypass mode)
fn simulate_permission_accept(
    state: &TuiAppState,
    permission_type: &PermissionType,
    tool_name: &str,
) {
    let mut inner = state.inner.lock();
    inner
        .display
        .response_content
        .push_str(&format!("\n⏺ {}({})\n", tool_name, {
            match permission_type {
                PermissionType::Bash { command, .. } => command.clone(),
                PermissionType::Edit { file_path, .. } => file_path.clone(),
                PermissionType::Write { file_path, .. } => file_path.clone(),
            }
        }));
    inner.mode = AppMode::Input;
}

/// Build a tool_use content block from a permission type.
///
/// Returns (tool_use_id, content_blocks) for recording to JSONL.
/// Tool use IDs follow the Claude API format: `toolu_{uuid}`.
pub(crate) fn build_tool_use_content(
    permission_type: &PermissionType,
) -> (String, Vec<ContentBlock>) {
    let tool_use_id = format!("toolu_{}", uuid::Uuid::new_v4().simple());

    let content = match permission_type {
        PermissionType::Bash {
            command,
            description,
        } => {
            let mut input = serde_json::json!({ "command": command });
            if let Some(desc) = description {
                input["description"] = serde_json::json!(desc);
            }
            ContentBlock::ToolUse {
                id: tool_use_id.clone(),
                name: "Bash".to_string(),
                input,
            }
        }
        PermissionType::Edit {
            file_path,
            diff_lines,
        } => ContentBlock::ToolUse {
            id: tool_use_id.clone(),
            name: "Edit".to_string(),
            input: serde_json::json!({
                "file_path": file_path,
                "changes": diff_lines.len()
            }),
        },
        PermissionType::Write {
            file_path,
            content_lines,
        } => ContentBlock::ToolUse {
            id: tool_use_id.clone(),
            name: "Write".to_string(),
            input: serde_json::json!({
                "file_path": file_path,
                "content": content_lines.join("\n")
            }),
        },
    };

    (tool_use_id, vec![content])
}

#[cfg(test)]
#[path = "permission_tests.rs"]
mod tests;
