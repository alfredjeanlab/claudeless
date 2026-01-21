# Claude TUI Snapshots (v2.1.14)

Captured from real Claude CLI for comparison testing.

**Behavior observed with:** claude --version 2.1.14 (Claude Code)

## Setup Flow Captures

### Theme Selection

#### setup_01_select_theme_dark.txt
Initial setup screen with dark mode selected (default):
- Shows "Welcome to Claude Code v2.1.14" header
- ASCII art logo (dark mode variant with stars and moon)
- "Let's get started." message
- "Choose the text style that looks best with your terminal"
- Six theme options:
  1. Dark mode (selected by default, marked with ✔)
  2. Light mode
  3. Dark mode (colorblind-friendly)
  4. Light mode (colorblind-friendly)
  5. Dark mode (ANSI colors only)
  6. Light mode (ANSI colors only)
- Syntax highlighting preview showing a diff
- "Syntax theme: Monokai Extended (ctrl+t to disable)"

#### setup_01_select_theme_light.txt
Setup screen with light mode selected:
- Different ASCII art logo (light mode variant with sun)
- Cursor on option 2 (Light mode)
- "Syntax theme: GitHub (ctrl+t to disable)"

#### setup_01a_syntax_highlighting_disabled.txt
Theme selection with syntax highlighting toggled off via Ctrl+T:
- Same layout as theme selection
- Shows "Syntax highlighting disabled (ctrl+t to enable)"

### Syntax Theme Variants
- **Dark modes** (1, 3): Use "Monokai Extended" syntax theme
- **Light modes** (2, 4): Use "GitHub" syntax theme
- **ANSI modes** (5, 6): Use "ansi" syntax theme

### Login Method Selection

#### setup_02_login_method.txt
Login method selection screen:
- Same welcome header and logo
- "Claude Code can be used with your Claude subscription or billed based on API usage"
- Two options:
  1. Claude account with subscription · Pro, Max, Team, or Enterprise
  2. Anthropic Console account · API usage billing

### Browser Login

#### setup_03_login_browser.txt
Browser login screen after selecting Claude subscription:
- "Browser didn't open? Use the url below to sign in (c to copy)"
- OAuth URL with placeholders for client_id, redirect_uri, code_challenge, etc.
- "Paste code here if prompted >" input field

### Security Notes

#### setup_03_security_notes.txt
Security warning screen shown after successful login:
- "Security notes:" header
- "Claude can make mistakes"
- "You should always review Claude's responses, especially when running code."
- "Due to prompt injection risks, only use it with code you trust"
- Link to security documentation
- "Press Enter to continue..."

### Login Success

#### setup_04_login_success.txt
Login confirmation screen:
- "Logged in as kevin@alfredjean.org"
- "Login successful. Press Enter to continue..."

### Terminal Setup

#### setup_05_use_terminal_setup.txt
Terminal configuration screen:
- "Use Claude Code's terminal setup?"
- "For the optimal coding experience, enable the recommended settings for your terminal: Option+Enter for newlines and visual bell"
- Two options:
  1. Yes, use recommended settings
  2. No, maybe later with /terminal-setup
- "Enter to confirm · Esc to skip"

## Normal TUI State

### initial_state.txt
The initial TUI state after setup completes:
- Small logo with version info
- Model name (e.g., "Opus 4.5 · Claude Max")
- Working directory
- Placeholder prompt hint (e.g., 'Try "create a util logging.py that..."')
- Help shortcut hint ("? for shortcuts")

### api_usage_billing.txt
TUI state when logged in with API Usage Billing (Anthropic Console) instead of Claude Max:
- Shows "Haiku 4.5 · API Usage Billing" in header
- Same TUI layout as Claude Max subscription
- Only the subscription type indicator differs

## Slash Commands

### slash_logout.txt
Output after running /logout command:
- Shows the /logout command with "❯" prefix
- "Successfully logged out from your Anthropic account."
- Exits Claude Code immediately (returns to shell prompt)

## Error States

### failed_to_open_socket.txt
Error when Claude Code cannot connect (no internet):
- Shows welcome header and logo
- "Unable to connect to Anthropic services"
- "Failed to connect to api.anthropic.com: FailedToOpenSocket"
- "Please check your internet connection and network settings."
- "Note: Claude Code might not be available in your country. Check supported countries at https://anthropic.com/supported-countries"
- Exits immediately (returns to shell prompt)

### failed_to_open_socket_no_version.txt
Same as above but showing "Claudeless v0.1.0" instead of "Claude Code v2.1.14":
- Used to test that claudeless produces the same error output format

## Capture Method

Captured using tmux:
```bash
tmux new-session -d -s claude-tui -x 120 -y 40
tmux send-keys -t claude-tui 'claude' Enter
sleep 3
tmux capture-pane -t claude-tui -p
```

For setup flow, Claude Code was started without existing authentication to trigger the setup wizard.
