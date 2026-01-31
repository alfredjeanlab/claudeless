// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI application state and main iocraft component.

mod commands;
mod dialogs;
mod input;
pub(crate) mod render;
pub(crate) mod state;
pub(crate) mod types;

pub use state::TuiAppState;
pub use types::{AppMode, ExitReason, PermissionRequest, StatusInfo, TrustPromptState, TuiConfig};

use iocraft::prelude::*;

use crate::scenario::Scenario;
use crate::state::session::SessionManager;
use crate::time::ClockHandle;

use render::render_main_content;

/// Props for the main App component
#[derive(Default, Props)]
pub struct AppProps {
    pub state: Option<TuiAppState>,
}

/// Main TUI App component using iocraft
#[component]
pub fn App(mut hooks: Hooks, props: &AppProps) -> impl Into<AnyElement<'static>> {
    // Get state from props with fallback error display
    let Some(state) = props.state.clone() else {
        return element! {
            View(flex_direction: FlexDirection::Column) {
                Text(content: "Error: TuiAppState must be provided via props")
            }
        };
    };

    let mut should_exit = hooks.use_state(|| false);
    // Render counter to force re-renders when state changes
    let mut render_counter = hooks.use_state(|| 0u64);
    // Timer counter for periodic updates (compacting, streaming, etc.)
    let mut timer_counter = hooks.use_state(|| 0u64);
    let state_clone = state.clone();

    // Handle terminal events (keyboard input and resize)
    hooks.use_terminal_events({
        let state = state.clone();
        move |event| match event {
            TerminalEvent::Key(key) if key.kind != KeyEventKind::Release => {
                state.handle_key_event(key);
                // Increment counter to trigger re-render
                let current = *render_counter.read();
                render_counter.set(current.wrapping_add(1));
                if state.should_exit() {
                    should_exit.set(true);
                }
            }
            TerminalEvent::Resize(width, _height) => {
                state.set_terminal_width(width);
                // Increment counter to trigger re-render
                let current = *render_counter.read();
                render_counter.set(current.wrapping_add(1));
            }
            _ => {}
        }
    });

    // Periodic timer for updates (compacting, streaming, spinner animation, etc.)
    // 120ms matches Claude Code's spinner timing
    hooks.use_future({
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                let current = *timer_counter.read();
                timer_counter.set(current.wrapping_add(1));
            }
        }
    });

    // Check for timeouts (both compacting and exit hint)
    state_clone.check_compacting();
    state_clone.check_exit_hint_timeout();

    // Advance spinner animation when in Responding or Thinking mode
    {
        let mode = state_clone.mode();
        if matches!(mode, AppMode::Responding | AppMode::Thinking) {
            state_clone.advance_spinner();
        }
    }

    // Get current render state
    let render_state = state_clone.render_state();

    // Exit if needed
    let should_exit_val = should_exit.read();
    if *should_exit_val || state_clone.should_exit() {
        hooks.use_context_mut::<SystemContext>().exit();
    }

    // Render based on mode
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
        ) {
            #(render_main_content(&render_state))
        }
    }
}

/// Legacy TuiApp struct for compatibility
/// This wraps the iocraft-based app and provides the same interface
pub struct TuiApp {
    state: TuiAppState,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new(
        scenario: Scenario,
        sessions: SessionManager,
        clock: ClockHandle,
        config: TuiConfig,
    ) -> std::io::Result<Self> {
        let state = TuiAppState::new(scenario, sessions, clock, config);
        Ok(Self { state })
    }

    /// Run the main event loop using iocraft fullscreen
    pub fn run(&mut self) -> std::io::Result<ExitReason> {
        loop {
            let state = self.state.clone();

            // Check if we're already in a tokio runtime
            if tokio::runtime::Handle::try_current().is_ok() {
                // Already in a runtime - use block_in_place to run async code
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        // ignore_ctrl_c() prevents iocraft from exiting on Ctrl+C - we handle it ourselves
                        // Use render_loop() instead of fullscreen() to:
                        // 1. Render inline (like the real Claude CLI) instead of alternate screen
                        // 2. Not enable mouse capture (allows normal text selection/copy)
                        element!(App(state: Some(state.clone())))
                            .render_loop()
                            .ignore_ctrl_c()
                            .await
                    })
                })?;
            } else {
                // No runtime - create a new one
                let rt = tokio::runtime::Runtime::new()?;
                // ignore_ctrl_c() prevents iocraft from exiting on Ctrl+C - we handle it ourselves
                // Use render_loop() instead of fullscreen() to:
                // 1. Render inline (like the real Claude CLI) instead of alternate screen
                // 2. Not enable mouse capture (allows normal text selection/copy)
                rt.block_on(async {
                    element!(App(state: Some(state.clone())))
                        .render_loop()
                        .ignore_ctrl_c()
                        .await
                })?;
            }

            // Check if we exited due to suspend request
            if matches!(self.state.exit_reason(), Some(ExitReason::Suspended)) {
                // Print suspend messages
                println!("Claude Code has been suspended. Run `fg` to bring Claude Code back.");
                println!("Note: ctrl + z now suspends Claude Code, ctrl + _ undoes input.");

                // Raise SIGTSTP to actually suspend the process
                // After this, execution pauses until SIGCONT is received
                #[cfg(unix)]
                {
                    let _ = signal_hook::low_level::raise(signal_hook::consts::SIGTSTP);
                }

                // On resume (SIGCONT), clear exit state and re-enter fullscreen
                self.state.clear_exit_state();
                continue;
            }

            // Exit for any other reason
            return Ok(self.state.exit_reason().unwrap_or(ExitReason::Completed));
        }
    }

    /// Get state reference for testing
    pub fn state(&self) -> &TuiAppState {
        &self.state
    }

    // Compatibility methods that delegate to state
    pub fn exit(&mut self, reason: ExitReason) {
        self.state.exit(reason);
    }

    pub fn mode(&self) -> AppMode {
        self.state.mode()
    }

    pub fn exit_message(&self) -> Option<String> {
        self.state.exit_message()
    }

    pub fn input_buffer(&self) -> String {
        self.state.input_buffer()
    }

    pub fn cursor_pos(&self) -> usize {
        self.state.cursor_pos()
    }

    pub fn response_content(&self) -> String {
        self.state.render_state().display.response_content.clone()
    }

    pub fn is_streaming(&self) -> bool {
        self.state.render_state().display.is_streaming
    }

    pub fn status(&self) -> StatusInfo {
        self.state.render_state().status
    }

    pub fn pending_permission(&self) -> Option<PermissionRequest> {
        self.state.render_state().dialog.as_permission().cloned()
    }

    pub fn show_permission_request(
        &mut self,
        permission_type: super::widgets::permission::PermissionType,
    ) {
        self.state.show_permission_request(permission_type);
    }

    pub fn show_bash_permission(&mut self, command: String, description: Option<String>) {
        self.state.show_bash_permission(command, description);
    }

    pub fn show_edit_permission(
        &mut self,
        file_path: String,
        diff_lines: Vec<super::widgets::permission::DiffLine>,
    ) {
        self.state.show_edit_permission(file_path, diff_lines);
    }

    pub fn show_write_permission(&mut self, file_path: String, content_lines: Vec<String>) {
        self.state.show_write_permission(file_path, content_lines);
    }
}

#[cfg(test)]
#[path = "app_exit_tests.rs"]
mod exit_tests;

#[cfg(test)]
#[path = "app_keybinding_tests.rs"]
mod keybinding_tests;

#[cfg(test)]
#[path = "app_menu_tests.rs"]
mod menu_tests;

#[cfg(test)]
#[path = "app_permission_tests.rs"]
mod permission_tests;
