SHELL := /bin/bash

.PHONY: check build test install clean license outdated lint lint-shell lint-policy

# Run all CI checks
check: lint
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test --all
	cargo build --all
	@if [ -n "$$CI" ]; then \
		mv .cargo/audit.toml .cargo/audit.toml.bak 2>/dev/null || true; \
		cargo audit; \
		mv .cargo/audit.toml.bak .cargo/audit.toml 2>/dev/null || true; \
	else \
		cargo audit; \
	fi
	cargo deny check licenses bans sources

# Build release binary
build:
	cargo build --release

# Run tests
test:
	cargo test --all

# Install to ~/.local/bin
install:
	@scripts/install

# Clean build artifacts
clean:
	cargo clean

# Check for outdated dependencies
outdated:
	cargo outdated

# Lint all files
lint: lint-shell lint-policy

# Lint shell scripts with ShellCheck
lint-shell:
	@if ! command -v shellcheck >/dev/null 2>&1; then \
		echo "Error: shellcheck not found. Install with: brew install shellcheck"; \
		exit 1; \
	fi
	@shellcheck -x -S warning scripts/*
	@echo "All scripts pass ShellCheck!"

# Check policy enforcement (allow attributes, deny.toml, shellcheck exceptions)
lint-policy:
	@scripts/lint-policy

# Add/update license headers in source files
license:
	@scripts/license
