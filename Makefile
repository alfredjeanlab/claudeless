SHELL := /bin/bash

.PHONY: check ci install license outdated capture capture-retry capture-skipped generate-specs

# Quick checks
#
# Excluded:
#   SKIP `cargo audit`
#   SKIP `cargo deny`
#   SKIP `cargo publish --dry-run`
#
check:
	cargo fmt --all
	cargo clippy --all -- -D warnings
	quench check --fix
	cargo build --all
	cargo test --all

# Full pre-release checks
ci:
	cargo fmt --all
	cargo clippy --all -- -D warnings
	quench check --fix
	cargo build --all
	cargo test --all
	cargo publish --dry-run --allow-dirty -p claudeless
	cargo audit
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

# Generate spec tests from capture fixtures
generate-specs:
	bun run tests/capture/generate.ts
