SHELL := /bin/bash

.PHONY: check install license outdated lint lint-shell lint-policy capture capture-experimental

# Run all CI checks
check: lint
	cargo fmt --all
	cargo clippy --all-targets --all-features -- -D warnings
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

# Lint all files
lint: lint-shell lint-policy

# Lint shell scripts with ShellCheck
lint-shell:
	@if ! command -v shellcheck >/dev/null 2>&1; then \
		echo "Error: shellcheck not found. Install with: brew install shellcheck"; \
		exit 1; \
	fi
	@shellcheck -x -S warning scripts/* tests/capture/*.sh tests/capture/lib/*.sh

# Check policy enforcement (allow attributes, deny.toml, shellcheck exceptions)
lint-policy:
	@scripts/lint-policy

# Add/update license headers in source files
license:
	@scripts/license

# Capture TUI fixtures from real Claude CLI (reliable scripts only)
capture:
	./tests/capture/capture.sh --skip-requires-config

# Capture all TUI fixtures including experimental (may fail)
capture-experimental:
	RUN_EXPERIMENTAL=1 ./tests/capture/capture.sh
