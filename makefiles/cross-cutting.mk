# ============================================================================
# makefiles/cross-cutting.mk — Cross-Cutting Targets
# ============================================================================
# Covers: deps-*, format, links-check, audit, lint, ci aggregate, clean aggregate

.PHONY: deps-check deps-check-required deps-check-optional \
	deps-upgrade deps-upgrade-npm deps-upgrade-cargo deps-upgrade-interactive \
	format format-check \
	links-check links-check-external links-check-all \
	audit audit-npm audit-cargo \
	lint lint-yaml lint-markdown lint-json lint-rust lint-html lint-check-all \
	ci clean

# ============================================================================
# Dependency Check Targets
# ============================================================================

deps-check: deps-check-required deps-check-optional

deps-check-required:
	@echo "$(BOLD)$(BLUE)Checking required dependencies...$(RESET)"
	@echo "$(BOLD)OS Detected: $(OS)$(RESET)"
	@echo ""
	@echo "$(BOLD)Node.js tooling:$(RESET)"
	$(call require_tool,pnpm,)
	$(call require_tool,node,)
	@echo ""
	@echo "$(BOLD)Validation tools:$(RESET)"
	$(call require_tool,yamllint,)
	$(call require_tool,shellcheck,)
	@echo ""
	@echo "$(BOLD)Rust tooling:$(RESET)"
	$(call require_tool,cargo,)
	$(call require_tool,rustc,)
	@echo ""
	@echo "$(BOLD)Go tooling:$(RESET)"
	$(call require_tool,go,)
	$(call require_tool,gofmt,)
	@echo ""
	@echo "$(BOLD)Container tools:$(RESET)"
	$(call require_tool,docker,)
	@echo ""
	@echo "$(BOLD)Utilities:$(RESET)"
	$(call require_tool,yq,)
	$(call require_tool,jq,)
	$(call require_tool,git,)
	@echo ""
	@echo "$(GREEN)$(BOLD)✓ All required dependencies installed$(RESET)"

deps-check-optional:
	@echo ""
	@echo "$(BOLD)$(BLUE)Checking optional dependencies...$(RESET)"
	@echo "$(BOLD)OS Detected: $(OS)$(RESET)"
	@echo ""
	@echo "$(BOLD)Deployment tools:$(RESET)"
	$(call check_optional_tool,flyctl,)
	$(call check_optional_tool,devpod,https://devpod.sh/docs/getting-started/install)
	@echo ""
	@echo "$(BOLD)GitHub CLI:$(RESET)"
	$(call check_optional_tool,gh,)
	@echo ""
	@echo "$(BOLD)Rust cargo extensions:$(RESET)"
	$(call check_optional_tool,cargo-outdated,)
	$(call check_optional_tool,cargo-audit,)
	$(call check_optional_tool,cargo-upgrade,)
	@echo ""
	@echo "$(BOLD)Go optional tools:$(RESET)"
	$(call check_optional_tool,golangci-lint,)
	$(call check_optional_tool,govulncheck,)
	@echo ""
	@echo "$(BOLD)Link checking:$(RESET)"
	$(call check_optional_tool,lychee,)
	@echo ""
	@echo "$(BOLD)Node.js validation:$(RESET)"
	$(call check_node_package,markdownlint)
	$(call check_node_package,prettier)
	@echo ""

# ============================================================================
# Dependency Management
# ============================================================================

deps-upgrade: deps-upgrade-npm deps-upgrade-cargo console-upgrade
	@echo "$(GREEN)$(BOLD)✓ All dependencies upgraded$(RESET)"

deps-upgrade-npm:
	@echo "$(BLUE)Upgrading npm dependencies...$(RESET)"
	pnpm deps:upgrade:npm
	@echo "$(GREEN)✓ npm dependencies upgraded$(RESET)"

deps-upgrade-cargo:
	@echo "$(BLUE)Upgrading cargo dependencies...$(RESET)"
	@if ! cargo upgrade --version >/dev/null 2>&1; then \
		echo "$(YELLOW)cargo-edit not installed. Installing...$(RESET)"; \
		cargo install cargo-edit; \
	fi
	pnpm deps:upgrade:cargo
	@echo "$(GREEN)✓ cargo dependencies upgraded$(RESET)"

deps-upgrade-interactive:
	@echo "$(BLUE)Interactive dependency upgrade...$(RESET)"
	pnpm deps:upgrade:npm:interactive

# ============================================================================
# Root-Level Formatting
# ============================================================================

format:
	@echo "$(BLUE)Formatting all files...$(RESET)"
	@echo "$(BLUE)  → Running Prettier (JSON/MD)...$(RESET)"
	@pnpm format
	@echo "$(BLUE)  → Running cargo fmt (Rust)...$(RESET)"
	@cd $(V3_DIR) && cargo fmt --all
	@echo "$(BLUE)  → Running Prettier on Console (TypeScript)...$(RESET)"
	@$(MAKE) console-fmt
	@echo "$(BLUE)  → Running gofmt on Console agent (Go)...$(RESET)"
	@$(MAKE) console-agent-fmt
	@echo "$(GREEN)✓ All files formatted$(RESET)"

format-check:
	@echo "$(BLUE)Checking formatting...$(RESET)"
	@echo "$(BLUE)  → Checking Prettier formatting...$(RESET)"
	@pnpm format:check
	@echo "$(BLUE)  → Checking Rust formatting...$(RESET)"
	@cd $(V3_DIR) && cargo fmt --all -- --check
	@echo "$(BLUE)  → Checking Console TypeScript formatting...$(RESET)"
	@$(MAKE) console-fmt-check
	@echo "$(BLUE)  → Checking Console agent Go formatting...$(RESET)"
	@$(MAKE) console-agent-fmt-check
	@echo "$(GREEN)✓ All formatting checks passed$(RESET)"

# ============================================================================
# Link Checking
# ============================================================================

links-check:
	@echo "$(BLUE)Checking local file links...$(RESET)"
	pnpm links:check
	@echo "$(GREEN)✓ Local link check complete$(RESET)"

links-check-external:
	@echo "$(BLUE)Checking external HTTP/HTTPS links...$(RESET)"
	@echo "$(YELLOW)Note: This may take a few minutes$(RESET)"
	pnpm links:check:external
	@echo "$(GREEN)✓ External link check complete$(RESET)"

links-check-all:
	@echo "$(BLUE)Checking all links (local + external)...$(RESET)"
	pnpm links:check:all
	@echo "$(GREEN)✓ All link checks complete$(RESET)"

# ============================================================================
# Security Auditing
# ============================================================================

audit: audit-npm audit-cargo console-audit console-agent-audit
	@echo "$(GREEN)$(BOLD)✓ All security audits complete$(RESET)"

audit-npm:
	@echo "$(BLUE)Running npm security audit...$(RESET)"
	pnpm audit
	@echo "$(GREEN)✓ npm audit complete$(RESET)"

audit-cargo:
	@echo "$(BLUE)Running cargo security audit...$(RESET)"
	cd $(V3_DIR) && cargo audit
	@echo "$(GREEN)✓ cargo audit complete$(RESET)"

# ============================================================================
# Comprehensive Linting
# ============================================================================

lint: lint-yaml lint-markdown lint-json lint-rust console-lint console-agent-lint
	@echo "$(GREEN)$(BOLD)✓ All linting complete$(RESET)"

lint-yaml:
	@echo "$(BLUE)Linting YAML files (strict mode)...$(RESET)"
	@yamllint --strict . || (echo "$(YELLOW)Note: Some YAML files may not pass strict linting$(RESET)"; exit 0)
	@echo "$(GREEN)✓ YAML linting complete$(RESET)"

lint-markdown:
	@echo "$(BLUE)Linting markdown files...$(RESET)"
	@if [ -f node_modules/.bin/markdownlint ]; then \
		node_modules/.bin/markdownlint '**/*.md' --ignore node_modules || (echo "$(YELLOW)Note: Some markdown files may have linting issues$(RESET)"; exit 0); \
	else \
		echo "$(YELLOW)markdownlint not installed. Run: pnpm install$(RESET)"; \
	fi
	@echo "$(GREEN)✓ Markdown linting complete$(RESET)"

lint-json:
	@echo "$(BLUE)Linting JSON files with Prettier...$(RESET)"
	@pnpm format:check || (echo "$(YELLOW)Note: Some JSON files may not be formatted correctly$(RESET)"; exit 0)
	@echo "$(GREEN)✓ JSON linting complete$(RESET)"

lint-rust:
	@echo "$(BLUE)Linting Rust code with clippy...$(RESET)"
	cd $(V3_DIR) && cargo clippy --workspace --all-targets --all-features -- -D warnings
	@echo "$(GREEN)✓ Rust linting complete$(RESET)"

lint-html:
	@echo "$(BLUE)Checking for HTML files...$(RESET)"
	@if find . -name "*.html" -not -path "./node_modules/*" -not -path "./target/*" | grep -q .; then \
		echo "$(YELLOW)HTML files found but no linter configured$(RESET)"; \
		echo "$(YELLOW)Consider installing: npm install -g htmlhint$(RESET)"; \
	else \
		echo "$(GREEN)No HTML files found$(RESET)"; \
	fi

lint-check-all:
	@echo "$(BOLD)$(BLUE)Running all linters with detailed status...$(RESET)"
	@echo ""
	@echo "$(BOLD)YAML Linting:$(RESET)"
	@make lint-yaml
	@echo ""
	@echo "$(BOLD)Markdown Linting:$(RESET)"
	@make lint-markdown
	@echo ""
	@echo "$(BOLD)JSON Linting:$(RESET)"
	@make lint-json
	@echo ""
	@echo "$(BOLD)Rust Linting:$(RESET)"
	@make lint-rust
	@echo ""
	@echo "$(GREEN)$(BOLD)✓ All linting checks complete$(RESET)"

# ============================================================================
# CI Aggregate Target
# ============================================================================

ci: v2-ci v3-ci console-ci
	@echo "$(GREEN)$(BOLD)✓ Full CI pipeline passed$(RESET)"

# ============================================================================
# Clean Aggregate Target
# ============================================================================

clean: v2-clean v3-clean console-clean console-agent-clean
	@echo "$(GREEN)✓ All build artifacts cleaned$(RESET)"
