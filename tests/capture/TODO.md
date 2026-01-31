# Missing Fixtures

Fixtures present in `crates/cli/tests/fixtures/tui/` but not yet captured for v2.1.27.

## TODO

- `thinking_dialog_mid_conversation` - Thinking dialog during conversation

## TODO (with API calls)

Use Haiku for cheap 1-3 turn conversations to capture these states.

- `after_response` - After Claude responds (send simple prompt, wait for response)
- `clear_before` / `clear_after` - /clear before/after (need conversation first)
- `thinking_dialog_disabled_selected` - Thinking dialog with disabled selected
- `thinking_dialog_enabled_selected` - Thinking dialog with enabled selected
- `compact_before` / `compact_during` / `compact_after` - Use `/compact` command to trigger

## Permission Dialogs (TODO)

- `permission_default` - Default permission prompt
- `permission_accept_edits` - Accept edits permission
- `permission_plan` - Plan permission

## Setup Flow (use `# Config: empty`)

These require `# Config: empty` in the script header (no OAuth token, shows full onboarding).
Only the pre-OAuth steps can be fully automated; login requires manual browser auth.

**Pre-OAuth (automatable):**
- `setup_01_select_theme_dark`
- `setup_01_select_theme_light`
- `setup_01a_syntax_highlighting_disabled`
- `setup_02_login_method`

**Requires manual OAuth:**
- `setup_03_login_browser` - Opens browser, needs manual login
- `setup_03_security_notes` - Shown after successful auth
- `setup_04_login_success` - After OAuth completes
- `setup_05_use_terminal_setup` - Final onboarding step
- `slash_logout` - Needs authenticated session first

## Error States

- `failed_to_open_socket`
- `failed_to_open_socket_no_version`

## Blocked (special requirements)

- `api_usage_billing` - Requires Anthropic Console account login (different auth flow)

## Finished

| Old Name | New Name |
|----------|----------|
| `initial_state` | `initial_state` |
| `model_haiku` | `model_haiku` |
| `model_opus` | `model_opus` |
| `model_picker` | `model_picker` |
| `model_sonnet` | `model_sonnet` |
| `shell_mode_command` | `shell_mode_command` |
| `shell_mode_prefix` | `shell_mode_prefix` |
| `thinking_dialog` | `thinking_dialog` |
| `escape_clear_hint` | `escape_clear_hint` |
| `help_general_tab` | `help_response` |
| `slash_search_menu` | `slash_menu` |
| `slash_search_filter` | `slash_menu_filtered` |
| `with_input` | `with_input` |
| `shortcuts_display` | `shortcuts_display` |
| `slash_search_tab_complete` | `slash_search_tab_complete` |
| `exit_autocomplete` | `exit_autocomplete` |
| `help_autocomplete` | `help_autocomplete` |
| `help_commands_tab` | `help_commands_tab` |
| `hooks_autocomplete` | `hooks_autocomplete` |
| `hooks_matcher_dialog` | `hooks_matcher_dialog` |
| `export_autocomplete` | `export_autocomplete` |
| `export_filename_dialog` | `export_filename_dialog` |
| `export_method_dialog` | `export_method_dialog` |
| `context_autocomplete` | `context_autocomplete` |
| `context_usage` | `context_usage` |
| `fork_no_conversation` | `fork_no_conversation` |
| `status_bar_extended` | `status_bar_extended` |
| `tasks_empty_dialog` | `tasks_empty_dialog` |
| `todos_empty` | `todos_empty` |
| `thinking_off_status` | `thinking_off_status` |
| `ctrl_z_suspend` | `ctrl_z_suspend` |
| `ctrl_s_stash_active` | `ctrl_s_stash_active` |
| `hooks_dialog` | `hooks_dialog` |
| `permission_bash_command` | `permission_bash_command` |
| `permission_edit_file` | `permission_edit_file` |
| `permission_write_file` | `permission_write_file` |
| `permission_bypass` | `permission_bypass` |
| `permission_trust_folder` | `permission_trust_folder` |
| `trust_prompt` | `trust_prompt` |
| `ctrl_c_exit_hint` | `ctrl_c_exit_hint` |
| `ctrl_d_exit_hint` | `ctrl_d_exit_hint` |
