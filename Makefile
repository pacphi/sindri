# ============================================================================
# Sindri v4 Makefile
# ============================================================================
# v4 (Rust, redesigned) Makefile. Promoted from research/v4 during the
# April 2026 reorg. Targets mirror the v3 conventions where applicable.
# ============================================================================

V4_DIR := $(CURDIR)/v4

# Colors
BOLD := $(shell tput bold 2>/dev/null || echo '')
GREEN := $(shell tput setaf 2 2>/dev/null || echo '')
YELLOW := $(shell tput setaf 3 2>/dev/null || echo '')
RESET := $(shell tput sgr0 2>/dev/null || echo '')

.PHONY: help v4-build v4-build-debug v4-test v4-fmt v4-fmt-check v4-clippy v4-validate v4-audit v4-doc v4-clean v4-ci

# ============================================================================
# Help
# ============================================================================

help:
	@echo "$(BOLD)Sindri v4 Makefile$(RESET)"
	@echo ""
	@echo "$(BOLD)Build targets:$(RESET)"
	@echo "  v4-build           Build release binary"
	@echo "  v4-build-debug     Build debug binary"
	@echo "  v4-clean           Clean build artifacts"
	@echo ""
	@echo "$(BOLD)Test & validate:$(RESET)"
	@echo "  v4-test            Run all tests"
	@echo "  v4-fmt             Apply rustfmt"
	@echo "  v4-fmt-check       Check rustfmt without modifying"
	@echo "  v4-clippy          Run clippy lints (fails on warnings)"
	@echo "  v4-validate        fmt-check + clippy"
	@echo "  v4-audit           cargo-audit security check"
	@echo ""
	@echo "$(BOLD)Composite:$(RESET)"
	@echo "  v4-ci              v4-validate + v4-test + v4-build"
	@echo "  v4-doc             cargo doc"

# ============================================================================
# Build
# ============================================================================

v4-build:
	@cd $(V4_DIR) && cargo build --release --workspace

v4-build-debug:
	@cd $(V4_DIR) && cargo build --workspace

v4-clean:
	@cd $(V4_DIR) && cargo clean

# ============================================================================
# Test & Validate
# ============================================================================

v4-test:
	@cd $(V4_DIR) && cargo test --workspace

v4-fmt:
	@cd $(V4_DIR) && cargo fmt --all

v4-fmt-check:
	@cd $(V4_DIR) && cargo fmt --all --check

v4-clippy:
	@cd $(V4_DIR) && cargo clippy --workspace --all-targets -- -D warnings

v4-validate: v4-fmt-check v4-clippy

v4-audit:
	@command -v cargo-audit >/dev/null 2>&1 || cargo install --locked cargo-audit
	@cd $(V4_DIR) && cargo audit

v4-doc:
	@cd $(V4_DIR) && cargo doc --workspace --no-deps

# ============================================================================
# Composite
# ============================================================================

v4-ci: v4-validate v4-test v4-build
	@echo "$(GREEN)✓ v4 CI passed$(RESET)"
