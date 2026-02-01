SHELL := /bin/bash

.PHONY: check install license outdated capture capture-retry capture-skipped

# Run all CI checks
check: lint
	cargo fmt --all
	cargo clippy --all -- -D warnings
	quench check --fix
	cargo test --all
	cargo build --all
	cargo publish --dry-run --allow-dirty -p claudeless
	@if [ -n "$$CI" ]; then \
		mv .cargo/audit.toml .cargo/audit.toml.bak 2>/dev/null || true; \
		cargo audit; \
		mv .cargo/audit.toml.bak .cargo/audit.toml 2>/dev/null || true; \
	else \
		cargo audit; \
	fi
	cargo deny check licenses bans sources

# Install to ~/.local/bin
install:
	@scripts/install

# Check for outdated dependencies (root deps only, not transitive)
outdated:
	cargo outdated --depth 1

# Add/update license headers in source files
license:
	@quench check --ci --fix --license

# Capture TUI fixtures from real Claude CLI
capture:
	bun run tests/capture/capture.ts

# Retry only failed capture scripts
capture-retry:
	bun run tests/capture/capture.ts --retry

# Capture all TUI fixtures including skipped (may fail)
capture-skipped:
	RUN_SKIPPED=1 bun run tests/capture/capture.ts
