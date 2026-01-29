# Requires-Config Scripts

Scripts in this directory require special configuration to run properly.

## Requirements

### setup-flow.capsh

Captures the Claude CLI setup/onboarding flow. Requires:

- Fresh `CLAUDE_CONFIG_DIR` (no existing configuration)
- Internet access for OAuth/authentication

To run manually:

```bash
# Create temporary config directory
export CLAUDE_CONFIG_DIR=$(mktemp -d)

# Run the capture
capsh --frames /tmp/setup-frames -- claude < setup-flow.capsh

# Cleanup
rm -rf "$CLAUDE_CONFIG_DIR"
```

## Adding New Scripts

When adding scripts that require special setup:

1. Document the requirements in this README
2. Add a `# Requires:` comment in the script header
3. Consider if it can be made reliable with environment tricks
