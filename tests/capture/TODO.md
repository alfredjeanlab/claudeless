# Missing Fixtures

Fixtures not yet captured for v2.1.29.

## Usage Commands

- `usage_dialog` - /usage command showing usage limits panel

## Setup Flow (requires manual OAuth)

These require `# Config: empty` and manual browser auth.

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
