// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Hook loader for building HookExecutor from ClaudeSettings.

use std::io::Write;

use super::executor::{HookConfig, HookExecutor};
use super::protocol::HookEvent;
use crate::state::ClaudeSettings;

/// Load hooks from settings into an executor.
///
/// Parses hook definitions from ClaudeSettings and registers them with a HookExecutor.
/// Currently supports Stop hooks with bash commands.
pub fn load_hooks(settings: &ClaudeSettings) -> std::io::Result<HookExecutor> {
    let mut executor = HookExecutor::new();

    for hook_def in &settings.hooks {
        let event = match hook_def.matcher.event.as_str() {
            "Stop" => Some(HookEvent::Stop),
            "PreToolUse" => Some(HookEvent::PreToolExecution),
            "PostToolUse" => Some(HookEvent::PostToolExecution),
            "SessionStart" => Some(HookEvent::SessionStart),
            "Notification" => Some(HookEvent::Notification),
            _ => None,
        };

        let Some(event) = event else {
            continue;
        };

        for cmd in &hook_def.hooks {
            if cmd.command_type != "bash" {
                continue;
            }

            // Create a temporary script file for the command
            let script_path = create_hook_script(&cmd.command)?;
            let config = HookConfig::new(script_path, cmd.timeout)
                .with_blocking(true)
                .with_matcher(hook_def.matcher.matcher.clone());
            executor.register(event.clone(), config);
        }
    }

    Ok(executor)
}

/// Create a temporary script file for a hook command.
fn create_hook_script(command: &str) -> std::io::Result<std::path::PathBuf> {
    let mut temp_file = tempfile::Builder::new()
        .prefix("claudeless-hook-")
        .suffix(".sh")
        .tempfile()?;

    writeln!(temp_file, "#!/bin/bash")?;
    writeln!(temp_file, "{}", command)?;
    temp_file.flush()?;

    // Keep the file around (don't auto-delete)
    let (_, path) = temp_file.keep()?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms)?;
    }

    Ok(path)
}

#[cfg(test)]
#[path = "loader_tests.rs"]
mod tests;
