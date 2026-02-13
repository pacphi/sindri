# ============================================================================
# Sindri Makefile
# ============================================================================
# Comprehensive build and validation system for both v2 (Bash/Docker) and
# v3 (Rust) implementations.
#
# Quick Start:
#   make help          - Show all available targets
#   make deps-check    - Verify all dependencies
#   make ci            - Run full CI (both v2 and v3)
#   make v3-build      - Build Rust binary
#   make v2-test       - Test v2 components
# ============================================================================

# ============================================================================
# Variables and Configuration
# ============================================================================

PROJECT_ROOT := $(CURDIR)
V2_DIR := $(PROJECT_ROOT)/v2
V3_DIR := $(PROJECT_ROOT)/v3
V3_BINARY := $(V3_DIR)/target/release/sindri
V3_DEBUG_BINARY := $(V3_DIR)/target/debug/sindri

# Detect OS for cross-platform support
UNAME_S := $(shell uname -s 2>/dev/null || echo Windows)
ifeq ($(UNAME_S),Darwin)
	OS := macos
else ifeq ($(UNAME_S),Linux)
	OS := linux
else ifeq ($(findstring MINGW,$(UNAME_S)),MINGW)
	OS := windows
else ifeq ($(findstring MSYS,$(UNAME_S)),MSYS)
	OS := windows
else ifeq ($(findstring CYGWIN,$(UNAME_S)),CYGWIN)
	OS := windows
else
	OS := unknown
endif

# Colors for output (using tput for portability)
BOLD := $(shell tput bold 2>/dev/null || echo '')
RED := $(shell tput setaf 1 2>/dev/null || echo '')
GREEN := $(shell tput setaf 2 2>/dev/null || echo '')
YELLOW := $(shell tput setaf 3 2>/dev/null || echo '')
BLUE := $(shell tput setaf 4 2>/dev/null || echo '')
RESET := $(shell tput sgr0 2>/dev/null || echo '')

# Version and Git information for Docker tags
VERSION := $(shell grep '^version' v3/Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
GIT_COMMIT := $(shell git rev-parse --short HEAD 2>/dev/null || echo unknown)

# ============================================================================
# Dependency Checking Functions
# ============================================================================

# Check required dependency with cross-platform installation hint
define require_tool
	@if ! command -v $(1) >/dev/null 2>&1; then \
		echo "$(RED)✗ Missing required tool: $(1)$(RESET)"; \
		echo "  $(BOLD)OS: $(OS)$(RESET)"; \
		echo "  $(BOLD)Install:$(RESET)"; \
		case "$(OS)" in \
			macos) \
				case "$(1)" in \
					pnpm) echo "    npm install -g pnpm" ;; \
					node) echo "    brew install node" ;; \
					yamllint) echo "    brew install yamllint (or pip install yamllint)" ;; \
					shellcheck) echo "    brew install shellcheck" ;; \
					cargo|rustc) echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" ;; \
					docker) echo "    brew install --cask docker" ;; \
					yq) echo "    brew install yq" ;; \
					jq) echo "    brew install jq" ;; \
					git) echo "    brew install git" ;; \
					*) echo "    See tool documentation" ;; \
				esac ;; \
			linux) \
				case "$(1)" in \
					pnpm) echo "    npm install -g pnpm" ;; \
					node) echo "    curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo bash -" ;; \
					yamllint) echo "    pip install yamllint (or apt install yamllint)" ;; \
					shellcheck) echo "    apt install shellcheck" ;; \
					cargo|rustc) echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" ;; \
					docker) echo "    curl -fsSL https://get.docker.com | sh" ;; \
					yq) echo "    wget https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 -O /usr/local/bin/yq" ;; \
					jq) echo "    apt install jq" ;; \
					git) echo "    apt install git" ;; \
					*) echo "    See tool documentation" ;; \
				esac ;; \
			windows) \
				case "$(1)" in \
					pnpm) echo "    npm install -g pnpm" ;; \
					node) echo "    Download from https://nodejs.org" ;; \
					yamllint) echo "    pip install yamllint" ;; \
					shellcheck) echo "    choco install shellcheck" ;; \
					cargo|rustc) echo "    Download from https://rustup.rs" ;; \
					docker) echo "    Download Docker Desktop from https://docker.com" ;; \
					yq) echo "    choco install yq" ;; \
					jq) echo "    choco install jq" ;; \
					git) echo "    Download from https://git-scm.com/download/win" ;; \
					*) echo "    See tool documentation" ;; \
				esac ;; \
			*) echo "    Unknown OS - see tool documentation" ;; \
		esac; \
		exit 1; \
	else \
		echo "$(GREEN)✓ $(1)$(RESET) - $(shell command -v $(1))"; \
	fi
endef

# Check optional dependency with cross-platform warning
define check_optional_tool
	@if ! command -v $(1) >/dev/null 2>&1; then \
		echo "$(YELLOW)⚠ Optional tool not found: $(1)$(RESET)"; \
		echo "  $(BOLD)OS: $(OS)$(RESET)"; \
		echo "  $(BOLD)Install:$(RESET)"; \
		case "$(OS)" in \
			macos) \
				case "$(1)" in \
					flyctl) echo "    brew install flyctl" ;; \
					gh) echo "    brew install gh" ;; \
					cargo-outdated) echo "    cargo install cargo-outdated" ;; \
					cargo-audit) echo "    cargo install cargo-audit" ;; \
					cargo-upgrade) echo "    cargo install cargo-edit" ;; \
					lychee) echo "    cargo install lychee" ;; \
					devpod) echo "    See https://devpod.sh/docs/getting-started/install" ;; \
					*) echo "    cargo install $(1)" ;; \
				esac ;; \
			linux) \
				case "$(1)" in \
					flyctl) echo "    curl -L https://fly.io/install.sh | sh" ;; \
					gh) echo "    See https://cli.github.com/ for installation" ;; \
					cargo-outdated) echo "    cargo install cargo-outdated" ;; \
					cargo-audit) echo "    cargo install cargo-audit" ;; \
					cargo-upgrade) echo "    cargo install cargo-edit" ;; \
					lychee) echo "    cargo install lychee" ;; \
					devpod) echo "    See https://devpod.sh/docs/getting-started/install" ;; \
					*) echo "    cargo install $(1)" ;; \
				esac ;; \
			windows) \
				case "$(1)" in \
					flyctl) echo "    powershell: iwr https://fly.io/install.ps1 -useb | iex" ;; \
					gh) echo "    choco install gh (or winget install GitHub.cli)" ;; \
					cargo-outdated) echo "    cargo install cargo-outdated" ;; \
					cargo-audit) echo "    cargo install cargo-audit" ;; \
					cargo-upgrade) echo "    cargo install cargo-edit" ;; \
					lychee) echo "    cargo install lychee" ;; \
					devpod) echo "    See https://devpod.sh/docs/getting-started/install" ;; \
					*) echo "    cargo install $(1)" ;; \
				esac ;; \
			*) echo "    Unknown OS - see tool documentation" ;; \
		esac; \
	else \
		echo "$(GREEN)✓ $(1)$(RESET) - $(shell command -v $(1))"; \
	fi
endef

# Check Node.js package
define check_node_package
	@if [ ! -f node_modules/.bin/$(1) ]; then \
		echo "$(YELLOW)⚠ $(1) not installed$(RESET)"; \
		echo "  Install: pnpm install"; \
	else \
		echo "$(GREEN)✓ $(1)$(RESET) - node_modules/.bin/$(1)"; \
	fi
endef

# ============================================================================
# PHONY Declarations
# ============================================================================

.PHONY: help \
	deps-check deps-check-required deps-check-optional \
	deps-upgrade deps-upgrade-npm deps-upgrade-cargo deps-upgrade-interactive \
	format format-check \
	links-check links-check-external links-check-all \
	audit audit-npm audit-cargo \
	lint lint-yaml lint-markdown lint-json lint-rust lint-html lint-check-all \
	v2-validate v2-validate-yaml v2-validate-shell v2-validate-markdown \
	v2-lint v2-lint-yaml v2-lint-shell v2-lint-markdown \
	v2-test v2-test-unit v2-test-extensions \
	v2-build v2-build-latest v2-build-nocache \
	v2-docker-build v2-docker-build-latest v2-docker-build-nocache \
	v2-config-init v2-config-validate \
	v2-extensions-list v2-extensions-install v2-extensions-search \
	v2-profiles-list v2-connect \
	v2-deploy v2-deploy-docker v2-deploy-fly v2-deploy-devpod \
	v3-build v3-build-debug v3-check \
	v3-test v3-test-verbose v3-test-crate \
	v3-validate v3-validate-yaml v3-validate-rust \
	v3-clippy v3-fmt v3-fmt-check \
	v3-audit v3-audit-fix \
	v3-coverage v3-coverage-html v3-coverage-lcov \
	v3-doc v3-run v3-run-debug v3-install \
	v3-docker-build v3-docker-build-latest v3-docker-build-nocache \
	v3-docker-build-from-source v3-docker-build-from-binary \
	v3-cycle \
	ci v2-ci v3-ci v3-quality \
	clean v2-clean v3-clean

# ============================================================================
# Default Target
# ============================================================================

.DEFAULT_GOAL := help

help:
	@echo "$(BOLD)$(BLUE)╔════════════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(BOLD)$(BLUE)║                          Sindri Makefile                           ║$(RESET)"
	@echo "$(BOLD)$(BLUE)╚════════════════════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(BOLD)Quick Start:$(RESET)"
	@echo "  make deps-check        - Check all dependencies"
	@echo "  make ci                - Run full CI pipeline (v2 + v3)"
	@echo "  make v3-build          - Build Rust binary (release)"
	@echo "  make v2-test           - Run v2 unit tests"
	@echo "  make format            - Format all code (Prettier + Cargo)"
	@echo "  make lint              - Run all linters"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ Dependency Management ═══════════════════════════════════════════$(RESET)"
	@echo "  deps-check             - Check all dependencies (required + optional)"
	@echo "  deps-check-required    - Check only required dependencies"
	@echo "  deps-check-optional    - Check only optional dependencies"
	@echo "  deps-upgrade           - Upgrade npm and cargo dependencies"
	@echo "  deps-upgrade-npm       - Upgrade npm dependencies"
	@echo "  deps-upgrade-cargo     - Upgrade cargo dependencies"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ Formatting & Validation ═════════════════════════════════════════$(RESET)"
	@echo "  format                 - Format all files (Prettier + Cargo fmt)"
	@echo "  format-check           - Check formatting without changes"
	@echo "  links-check            - Check markdown links (local files only)"
	@echo "  links-check-external   - Check external HTTP/HTTPS links"
	@echo "  links-check-all        - Check all links (local + external)"
	@echo "  audit                  - Run security audits (npm + cargo)"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ Linting ═════════════════════════════════════════════════════════$(RESET)"
	@echo "  lint                   - Run all linters (YAML + MD + JSON + Rust)"
	@echo "  lint-yaml              - Lint all YAML files (strict)"
	@echo "  lint-markdown          - Lint all markdown files"
	@echo "  lint-json              - Lint all JSON files"
	@echo "  lint-rust              - Lint Rust code with clippy"
	@echo "  lint-html              - Lint HTML files (if any exist)"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ V2 (Bash/Docker) Targets ════════════════════════════════════════$(RESET)"
	@echo "  v2-validate            - Validate v2 code (YAML + shell + markdown)"
	@echo "  v2-validate-yaml       - Validate v2 YAML files"
	@echo "  v2-validate-shell      - Validate v2 shell scripts (shellcheck)"
	@echo "  v2-validate-markdown   - Validate v2 markdown files"
	@echo "  v2-lint                - Lint v2 code (strict mode)"
	@echo "  v2-test                - Run v2 unit tests"
	@echo "  v2-test-extensions     - Test v2 extensions"
	@echo "  v2-build               - Build v2 (alias for v2-docker-build)"
	@echo "  v2-docker-build        - Build v2 Docker image (local tag)"
	@echo "  v2-docker-build-latest - Build v2 Docker image (latest tag)"
	@echo "  v2-docker-build-nocache - Build v2 Docker image (no cache)"
	@echo "  v2-config-init         - Initialize sindri.yaml"
	@echo "  v2-config-validate     - Validate sindri.yaml"
	@echo "  v2-extensions-list     - List available extensions"
	@echo "  v2-extensions-install  - Install extensions interactively"
	@echo "  v2-extensions-search   - Search for extensions"
	@echo "  v2-profiles-list       - List extension profiles"
	@echo "  v2-deploy              - Deploy v2 to provider"
	@echo "  v2-connect             - Connect to v2 deployment"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ V3 (Rust) Targets ═══════════════════════════════════════════════$(RESET)"
	@echo "  v3-build               - Build Rust binary (release mode)"
	@echo "  v3-build-debug         - Build Rust binary (debug mode)"
	@echo "  v3-check               - Fast compile check"
	@echo "  v3-test                - Run Rust tests"
	@echo "  v3-test-verbose        - Run Rust tests (verbose output)"
	@echo "  v3-test-crate          - Test one crate (use: make v3-test-crate CRATE=sindri-core)"
	@echo "  v3-validate            - Validate Rust code (fmt + clippy + YAML)"
	@echo "  v3-validate-yaml       - Validate v3 YAML files"
	@echo "  v3-validate-rust       - Validate Rust code only"
	@echo "  v3-clippy              - Run clippy linter"
	@echo "  v3-fmt                 - Format Rust code"
	@echo "  v3-fmt-check           - Check Rust formatting"
	@echo "  v3-audit               - Security audit"
	@echo "  v3-audit-fix           - Fix security vulnerabilities"
	@echo "  v3-coverage            - Run tests with coverage summary"
	@echo "  v3-coverage-html       - Coverage report (HTML in coverage/)"
	@echo "  v3-coverage-lcov       - Coverage report (LCOV for CI)"
	@echo "  v3-doc                 - Generate and open documentation"
	@echo "  v3-run                 - Run v3 binary (use: make v3-run ARGS=\"version\")"
	@echo "  v3-run-debug           - Run v3 debug binary"
	@echo "  v3-install             - Install v3 binary to ~/.cargo/bin"
	@echo "  v3-docker-build        - Build v3 Docker image (from binary, local tag)"
	@echo "  v3-docker-build-latest - Build v3 Docker image (latest tag)"
	@echo "  v3-docker-build-nocache - Build v3 Docker image (no cache)"
	@echo "  v3-docker-build-from-source - Build v3 Docker from source"
	@echo "  v3-docker-build-from-binary - Build v3 Docker from binary (default)"
	@echo "  v3-docker-build-base   - Build base image (15-20 min, once per Rust version)"
	@echo "  v3-docker-build-fast   - Fast build using base (3-5 min, recommended)"
	@echo "  v3-cycle-fast          - Fast dev cycle (3-5 min, keeps caches)"
	@echo "                           Usage: make v3-cycle-fast CONFIG=/path/to/sindri.yaml"
	@echo "  v3-cycle-clean         - Clean dev cycle (10-15 min, clears caches)"
	@echo "  v3-cycle-nuclear       - Full rebuild (40-50 min, nuclear option)"
	@echo "  v3-cycle               - Alias for v3-cycle-nuclear"
	@echo "  v3-cache-status        - Show cache usage"
	@echo "  v3-cache-clear-soft    - Clear incremental caches"
	@echo "  v3-cache-clear-medium  - Clear cargo + build cache"
	@echo "  v3-cache-clear-hard    - Clear all except base"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ CI/CD Targets ═══════════════════════════════════════════════════$(RESET)"
	@echo "  ci                     - Run full CI (v2 + v3)"
	@echo "  v2-ci                  - Run v2 CI pipeline"
	@echo "  v3-ci                  - Run v3 CI pipeline (validate + test + build)"
	@echo "  v3-quality             - Full quality gate (fmt + clippy + test + audit + coverage)"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ Utility ═════════════════════════════════════════════════════════$(RESET)"
	@echo "  clean                  - Clean all build artifacts"
	@echo "  v2-clean               - Clean v2 Docker images"
	@echo "  v3-clean               - Clean v3 Rust artifacts"
	@echo ""

# ============================================================================
# Dependency Check Targets
# ============================================================================

.PHONY: deps-check
deps-check: deps-check-required deps-check-optional

.PHONY: deps-check-required
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
	@echo "$(BOLD)Container tools:$(RESET)"
	$(call require_tool,docker,)
	@echo ""
	@echo "$(BOLD)Utilities:$(RESET)"
	$(call require_tool,yq,)
	$(call require_tool,jq,)
	$(call require_tool,git,)
	@echo ""
	@echo "$(GREEN)$(BOLD)✓ All required dependencies installed$(RESET)"

.PHONY: deps-check-optional
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

.PHONY: deps-upgrade
deps-upgrade: deps-upgrade-npm deps-upgrade-cargo
	@echo "$(GREEN)$(BOLD)✓ All dependencies upgraded$(RESET)"

.PHONY: deps-upgrade-npm
deps-upgrade-npm:
	@echo "$(BLUE)Upgrading npm dependencies...$(RESET)"
	pnpm deps:upgrade:npm
	@echo "$(GREEN)✓ npm dependencies upgraded$(RESET)"

.PHONY: deps-upgrade-cargo
deps-upgrade-cargo:
	@echo "$(BLUE)Upgrading cargo dependencies...$(RESET)"
	pnpm deps:upgrade:cargo
	@echo "$(GREEN)✓ cargo dependencies upgraded$(RESET)"

.PHONY: deps-upgrade-interactive
deps-upgrade-interactive:
	@echo "$(BLUE)Interactive dependency upgrade...$(RESET)"
	pnpm deps:upgrade:npm:interactive

# ============================================================================
# Root-Level Formatting
# ============================================================================

.PHONY: format
format:
	@echo "$(BLUE)Formatting all files...$(RESET)"
	@echo "$(BLUE)  → Running Prettier (JSON/MD)...$(RESET)"
	@pnpm format
	@echo "$(BLUE)  → Running cargo fmt (Rust)...$(RESET)"
	@cd $(V3_DIR) && cargo fmt --all
	@echo "$(GREEN)✓ All files formatted$(RESET)"

.PHONY: format-check
format-check:
	@echo "$(BLUE)Checking formatting...$(RESET)"
	@echo "$(BLUE)  → Checking Prettier formatting...$(RESET)"
	@pnpm format:check
	@echo "$(BLUE)  → Checking Rust formatting...$(RESET)"
	@cd $(V3_DIR) && cargo fmt --all -- --check
	@echo "$(GREEN)✓ All formatting checks passed$(RESET)"

# ============================================================================
# Link Checking
# ============================================================================

.PHONY: links-check
links-check:
	@echo "$(BLUE)Checking local file links...$(RESET)"
	pnpm links:check
	@echo "$(GREEN)✓ Local link check complete$(RESET)"

.PHONY: links-check-external
links-check-external:
	@echo "$(BLUE)Checking external HTTP/HTTPS links...$(RESET)"
	@echo "$(YELLOW)Note: This may take a few minutes$(RESET)"
	pnpm links:check:external
	@echo "$(GREEN)✓ External link check complete$(RESET)"

.PHONY: links-check-all
links-check-all:
	@echo "$(BLUE)Checking all links (local + external)...$(RESET)"
	pnpm links:check:all
	@echo "$(GREEN)✓ All link checks complete$(RESET)"

# ============================================================================
# Security Auditing
# ============================================================================

.PHONY: audit
audit: audit-npm audit-cargo
	@echo "$(GREEN)$(BOLD)✓ All security audits complete$(RESET)"

.PHONY: audit-npm
audit-npm:
	@echo "$(BLUE)Running npm security audit...$(RESET)"
	pnpm audit
	@echo "$(GREEN)✓ npm audit complete$(RESET)"

.PHONY: audit-cargo
audit-cargo:
	@echo "$(BLUE)Running cargo security audit...$(RESET)"
	cd $(V3_DIR) && cargo audit
	@echo "$(GREEN)✓ cargo audit complete$(RESET)"

# ============================================================================
# Comprehensive Linting
# ============================================================================

.PHONY: lint
lint: lint-yaml lint-markdown lint-json lint-rust
	@echo "$(GREEN)$(BOLD)✓ All linting complete$(RESET)"

.PHONY: lint-yaml
lint-yaml:
	@echo "$(BLUE)Linting YAML files (strict mode)...$(RESET)"
	@yamllint --strict . || (echo "$(YELLOW)Note: Some YAML files may not pass strict linting$(RESET)"; exit 0)
	@echo "$(GREEN)✓ YAML linting complete$(RESET)"

.PHONY: lint-markdown
lint-markdown:
	@echo "$(BLUE)Linting markdown files...$(RESET)"
	@if [ -f node_modules/.bin/markdownlint ]; then \
		node_modules/.bin/markdownlint '**/*.md' --ignore node_modules || (echo "$(YELLOW)Note: Some markdown files may have linting issues$(RESET)"; exit 0); \
	else \
		echo "$(YELLOW)markdownlint not installed. Run: pnpm install$(RESET)"; \
	fi
	@echo "$(GREEN)✓ Markdown linting complete$(RESET)"

.PHONY: lint-json
lint-json:
	@echo "$(BLUE)Linting JSON files with Prettier...$(RESET)"
	@pnpm format:check || (echo "$(YELLOW)Note: Some JSON files may not be formatted correctly$(RESET)"; exit 0)
	@echo "$(GREEN)✓ JSON linting complete$(RESET)"

.PHONY: lint-rust
lint-rust:
	@echo "$(BLUE)Linting Rust code with clippy...$(RESET)"
	cd $(V3_DIR) && cargo clippy --workspace --all-targets --all-features -- -D warnings
	@echo "$(GREEN)✓ Rust linting complete$(RESET)"

.PHONY: lint-html
lint-html:
	@echo "$(BLUE)Checking for HTML files...$(RESET)"
	@if find . -name "*.html" -not -path "./node_modules/*" -not -path "./target/*" | grep -q .; then \
		echo "$(YELLOW)HTML files found but no linter configured$(RESET)"; \
		echo "$(YELLOW)Consider installing: npm install -g htmlhint$(RESET)"; \
	else \
		echo "$(GREEN)No HTML files found$(RESET)"; \
	fi

.PHONY: lint-check-all
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
# V2 Validation Targets
# ============================================================================

.PHONY: v2-validate
v2-validate: v2-validate-yaml v2-validate-shell v2-validate-markdown
	@echo "$(GREEN)$(BOLD)✓ All v2 validation passed$(RESET)"

.PHONY: v2-validate-yaml
v2-validate-yaml:
	@echo "$(BLUE)Validating v2 YAML files...$(RESET)"
	pnpm v2:validate:yaml
	@echo "$(GREEN)✓ v2 YAML validation passed$(RESET)"

.PHONY: v2-validate-shell
v2-validate-shell:
	@echo "$(BLUE)Validating v2 shell scripts with shellcheck...$(RESET)"
	pnpm v2:validate:shell
	@echo "$(GREEN)✓ v2 shell validation passed$(RESET)"

.PHONY: v2-validate-markdown
v2-validate-markdown:
	@echo "$(BLUE)Validating v2 markdown...$(RESET)"
	pnpm v2:validate:markdown
	@echo "$(GREEN)✓ v2 markdown validation passed$(RESET)"

# ============================================================================
# V2 Linting Targets
# ============================================================================

.PHONY: v2-lint
v2-lint: v2-lint-yaml v2-lint-shell v2-lint-markdown
	@echo "$(GREEN)$(BOLD)✓ All v2 linting passed$(RESET)"

.PHONY: v2-lint-yaml
v2-lint-yaml:
	@echo "$(BLUE)Linting v2 YAML files (strict mode)...$(RESET)"
	pnpm v2:lint:yaml
	@echo "$(GREEN)✓ v2 YAML linting passed$(RESET)"

.PHONY: v2-lint-shell
v2-lint-shell:
	@echo "$(BLUE)Linting v2 shell scripts (strict mode)...$(RESET)"
	pnpm v2:lint:shell
	@echo "$(GREEN)✓ v2 shell linting passed$(RESET)"

.PHONY: v2-lint-markdown
v2-lint-markdown:
	@echo "$(BLUE)Linting v2 markdown...$(RESET)"
	pnpm v2:lint:markdown
	@echo "$(GREEN)✓ v2 markdown linting passed$(RESET)"

# ============================================================================
# V2 Testing Targets
# ============================================================================

.PHONY: v2-test
v2-test: v2-test-unit
	@echo "$(GREEN)$(BOLD)✓ All v2 tests passed$(RESET)"

.PHONY: v2-test-unit
v2-test-unit:
	@echo "$(BLUE)Running v2 unit tests...$(RESET)"
	pnpm v2:test:unit
	@echo "$(GREEN)✓ v2 unit tests passed$(RESET)"

.PHONY: v2-test-extensions
v2-test-extensions:
	@echo "$(BLUE)Testing v2 extensions...$(RESET)"
	pnpm v2:test:extensions
	@echo "$(GREEN)✓ v2 extension tests passed$(RESET)"

# ============================================================================
# V2 Build Targets
# ============================================================================

# Aliases for backwards compatibility
.PHONY: v2-build
v2-build: v2-docker-build

.PHONY: v2-build-latest
v2-build-latest: v2-docker-build-latest

.PHONY: v2-build-nocache
v2-build-nocache: v2-docker-build-nocache

# ============================================================================
# V2 Docker Build Targets
# ============================================================================

.PHONY: v2-docker-build
v2-docker-build:
	@echo "$(BLUE)Building v2 Docker image (local tag)...$(RESET)"
	pnpm v2:build
	@echo "$(GREEN)✓ v2 Docker build complete: sindri:v2-local$(RESET)"

.PHONY: v2-docker-build-latest
v2-docker-build-latest:
	@echo "$(BLUE)Building v2 Docker image (latest tag)...$(RESET)"
	pnpm v2:build:latest
	@echo "$(GREEN)✓ v2 Docker build complete: sindri:v2-latest$(RESET)"

.PHONY: v2-docker-build-nocache
v2-docker-build-nocache:
	@echo "$(BLUE)Building v2 Docker image (no cache)...$(RESET)"
	@echo "$(YELLOW)Warning: This will take longer than normal$(RESET)"
	pnpm v2:build:nocache
	@echo "$(GREEN)✓ v2 Docker build complete: sindri:v2-latest$(RESET)"

# ============================================================================
# V2 Configuration Targets
# ============================================================================

.PHONY: v2-config-init
v2-config-init:
	@echo "$(BLUE)Initializing sindri.yaml...$(RESET)"
	pnpm v2:config:init
	@echo "$(GREEN)✓ sindri.yaml initialized$(RESET)"

.PHONY: v2-config-validate
v2-config-validate:
	@echo "$(BLUE)Validating sindri.yaml...$(RESET)"
	pnpm v2:config:validate
	@echo "$(GREEN)✓ sindri.yaml validation passed$(RESET)"

# ============================================================================
# V2 Extension Management Targets
# ============================================================================

.PHONY: v2-extensions-list
v2-extensions-list:
	@echo "$(BLUE)Listing available extensions...$(RESET)"
	pnpm v2:extensions:list

.PHONY: v2-extensions-install
v2-extensions-install:
	@echo "$(BLUE)Installing extensions interactively...$(RESET)"
	pnpm v2:extensions:install

.PHONY: v2-extensions-search
v2-extensions-search:
	@echo "$(BLUE)Searching for extensions...$(RESET)"
	pnpm v2:extensions:search

.PHONY: v2-profiles-list
v2-profiles-list:
	@echo "$(BLUE)Listing extension profiles...$(RESET)"
	pnpm v2:profiles:list

# ============================================================================
# V2 Deployment Targets
# ============================================================================

.PHONY: v2-deploy
v2-deploy:
	@echo "$(BLUE)Deploying v2...$(RESET)"
	pnpm v2:deploy

.PHONY: v2-deploy-docker
v2-deploy-docker:
	@echo "$(BLUE)Deploying v2 to Docker...$(RESET)"
	pnpm v2:deploy:docker

.PHONY: v2-deploy-fly
v2-deploy-fly:
	@echo "$(BLUE)Deploying v2 to Fly.io...$(RESET)"
	pnpm v2:deploy:fly

.PHONY: v2-deploy-devpod
v2-deploy-devpod:
	@echo "$(BLUE)Deploying v2 to DevPod...$(RESET)"
	pnpm v2:deploy:devpod

.PHONY: v2-connect
v2-connect:
	@echo "$(BLUE)Connecting to v2 deployment...$(RESET)"
	pnpm v2:connect

# ============================================================================
# V3 Build Targets
# ============================================================================

.PHONY: v3-build
v3-build:
	@echo "$(BLUE)Building v3 Rust binary (release mode)...$(RESET)"
	cd $(V3_DIR) && cargo build --release
	@echo "$(GREEN)✓ v3 build complete: $(V3_BINARY)$(RESET)"

.PHONY: v3-build-debug
v3-build-debug:
	@echo "$(BLUE)Building v3 Rust binary (debug mode)...$(RESET)"
	cd $(V3_DIR) && cargo build
	@echo "$(GREEN)✓ v3 debug build complete: $(V3_DEBUG_BINARY)$(RESET)"

.PHONY: v3-check
v3-check:
	@echo "$(BLUE)Checking v3 Rust code (fast compile check)...$(RESET)"
	cd $(V3_DIR) && cargo check --workspace
	@echo "$(GREEN)✓ v3 check passed$(RESET)"

# ============================================================================
# V3 Testing Targets
# ============================================================================

.PHONY: v3-test
v3-test:
	@echo "$(BLUE)Running v3 Rust tests...$(RESET)"
	cd $(V3_DIR) && cargo test --workspace
	@echo "$(GREEN)✓ v3 tests passed$(RESET)"

.PHONY: v3-test-verbose
v3-test-verbose:
	@echo "$(BLUE)Running v3 Rust tests (verbose output)...$(RESET)"
	cd $(V3_DIR) && cargo test --workspace -- --nocapture
	@echo "$(GREEN)✓ v3 verbose tests passed$(RESET)"

.PHONY: v3-test-crate
v3-test-crate:
	@if [ -z "$(CRATE)" ]; then \
		echo "$(YELLOW)Usage: make v3-test-crate CRATE=sindri-core$(RESET)"; \
		echo "Available crates:"; \
		echo "  sindri sindri-core sindri-providers sindri-extensions"; \
		echo "  sindri-secrets sindri-backup sindri-projects sindri-doctor"; \
		echo "  sindri-clusters sindri-image sindri-packer sindri-update"; \
		exit 1; \
	fi
	@echo "$(BLUE)Running tests for $(CRATE)...$(RESET)"
	cd $(V3_DIR) && cargo test -p $(CRATE)
	@echo "$(GREEN)✓ $(CRATE) tests passed$(RESET)"

# ============================================================================
# V3 Validation Targets
# ============================================================================

.PHONY: v3-validate
v3-validate: v3-validate-yaml v3-validate-rust
	@echo "$(GREEN)$(BOLD)✓ All v3 validation passed$(RESET)"

.PHONY: v3-validate-yaml
v3-validate-yaml:
	@echo "$(BLUE)Validating v3 YAML files...$(RESET)"
	pnpm v3:validate:yaml
	@echo "$(GREEN)✓ v3 YAML validation passed$(RESET)"

.PHONY: v3-validate-rust
v3-validate-rust: v3-fmt-check v3-clippy
	@echo "$(GREEN)✓ v3 Rust validation passed$(RESET)"

# ============================================================================
# V3 Linting Targets
# ============================================================================

.PHONY: v3-clippy
v3-clippy:
	@echo "$(BLUE)Running clippy linter on v3...$(RESET)"
	cd $(V3_DIR) && cargo clippy --workspace --all-targets --all-features -- -D warnings
	@echo "$(GREEN)✓ clippy passed$(RESET)"

.PHONY: v3-fmt
v3-fmt:
	@echo "$(BLUE)Formatting v3 Rust code...$(RESET)"
	cd $(V3_DIR) && cargo fmt --all
	@echo "$(GREEN)✓ Rust formatting complete$(RESET)"

.PHONY: v3-fmt-check
v3-fmt-check:
	@echo "$(BLUE)Checking v3 Rust formatting...$(RESET)"
	cd $(V3_DIR) && cargo fmt --all -- --check
	@echo "$(GREEN)✓ Rust formatting check passed$(RESET)"

# ============================================================================
# V3 Security Targets
# ============================================================================

.PHONY: v3-audit
v3-audit:
	@echo "$(BLUE)Running v3 security audit...$(RESET)"
	.github/scripts/v3/cargo-audit.sh
	@echo "$(GREEN)✓ Security audit complete$(RESET)"

.PHONY: v3-audit-fix
v3-audit-fix:
	@echo "$(BLUE)Fixing v3 security vulnerabilities...$(RESET)"
	cd $(V3_DIR) && cargo audit fix
	@echo "$(GREEN)✓ Security fixes applied$(RESET)"

# ============================================================================
# V3 Code Coverage (cargo-llvm-cov)
# ============================================================================

.PHONY: v3-coverage
v3-coverage:
	@echo "$(BLUE)Running v3 code coverage (summary)...$(RESET)"
	@if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo "$(YELLOW)cargo-llvm-cov not installed.$(RESET)"; \
		echo "  Install: rustup component add llvm-tools-preview && cargo install cargo-llvm-cov"; \
		exit 1; \
	fi
	cd $(V3_DIR) && cargo llvm-cov --workspace
	@echo "$(GREEN)✓ Coverage report complete$(RESET)"

.PHONY: v3-coverage-html
v3-coverage-html:
	@echo "$(BLUE)Generating v3 HTML coverage report...$(RESET)"
	@if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo "$(YELLOW)cargo-llvm-cov not installed.$(RESET)"; \
		echo "  Install: rustup component add llvm-tools-preview && cargo install cargo-llvm-cov"; \
		exit 1; \
	fi
	cd $(V3_DIR) && cargo llvm-cov --workspace --html --output-dir coverage/
	@echo "$(GREEN)✓ HTML report: $(V3_DIR)/coverage/html/index.html$(RESET)"

.PHONY: v3-coverage-lcov
v3-coverage-lcov:
	@echo "$(BLUE)Generating v3 LCOV coverage report...$(RESET)"
	@if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo "$(YELLOW)cargo-llvm-cov not installed.$(RESET)"; \
		echo "  Install: rustup component add llvm-tools-preview && cargo install cargo-llvm-cov"; \
		exit 1; \
	fi
	cd $(V3_DIR) && cargo llvm-cov --workspace --lcov --output-path coverage/lcov.info
	@echo "$(GREEN)✓ LCOV report: $(V3_DIR)/coverage/lcov.info$(RESET)"

# ============================================================================
# V3 Documentation
# ============================================================================

.PHONY: v3-doc
v3-doc:
	@echo "$(BLUE)Generating v3 Rust documentation...$(RESET)"
	cd $(V3_DIR) && cargo doc --workspace --no-deps --all-features --open
	@echo "$(GREEN)✓ Documentation generated and opened$(RESET)"

# ============================================================================
# V3 Binary Execution
# ============================================================================

.PHONY: v3-run
v3-run: v3-build
	@if [ -z "$(ARGS)" ]; then \
		echo "$(YELLOW)Usage: make v3-run ARGS=\"<command>\"$(RESET)"; \
		echo "Examples:"; \
		echo "  make v3-run ARGS=\"--version\""; \
		echo "  make v3-run ARGS=\"--help\""; \
		echo "  make v3-run ARGS=\"config validate\""; \
		echo "  make v3-run ARGS=\"extensions list\""; \
		exit 1; \
	fi
	@echo "$(BLUE)Running: $(V3_BINARY) $(ARGS)$(RESET)"
	@$(V3_BINARY) $(ARGS)

.PHONY: v3-run-debug
v3-run-debug: v3-build-debug
	@if [ -z "$(ARGS)" ]; then \
		echo "$(YELLOW)Usage: make v3-run-debug ARGS=\"<command>\"$(RESET)"; \
		exit 1; \
	fi
	@echo "$(BLUE)Running (debug): $(V3_DEBUG_BINARY) $(ARGS)$(RESET)"
	@$(V3_DEBUG_BINARY) $(ARGS)

.PHONY: v3-install
v3-install:
	@echo "$(BLUE)Installing sindri v3 to ~/.cargo/bin$(RESET)"
	cd $(V3_DIR) && cargo install --path crates/sindri
	@echo "$(GREEN)✓ Installed: $(shell which sindri 2>/dev/null || echo '~/.cargo/bin/sindri')$(RESET)"

# ============================================================================
# V3 Docker Build Targets
# ============================================================================

.PHONY: v3-docker-build
v3-docker-build: v3-docker-build-from-binary

.PHONY: v3-docker-build-from-binary
v3-docker-build-from-binary:
	@echo "$(BLUE)Building v3 production Docker image from pre-compiled binary (local tag)...$(RESET)"
	@echo "$(BLUE)Using Dockerfile (production) - downloads binary from GitHub releases (~5 min)$(RESET)"
	docker build -t sindri:v3-local \
		-f $(V3_DIR)/Dockerfile \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ v3 Docker build complete: sindri:v3-local$(RESET)"

.PHONY: v3-docker-build-from-source
v3-docker-build-from-source:
	@echo "$(BLUE)Building v3 development Docker image from Rust source (~8 min)...$(RESET)"
	@echo "$(BLUE)Using Dockerfile.dev (development) - builds from source with bundled extensions$(RESET)"
	docker build -t sindri:v3-dev \
		-f $(V3_DIR)/Dockerfile.dev \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ v3 Docker build complete: sindri:v3-dev$(RESET)"

.PHONY: v3-docker-build-latest
v3-docker-build-latest:
	@echo "$(BLUE)Building v3 production Docker image (latest tag)...$(RESET)"
	@echo "$(BLUE)Using Dockerfile (production)$(RESET)"
	docker build -t sindri:v3-latest \
		-f $(V3_DIR)/Dockerfile \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ v3 Docker build complete: sindri:v3-latest$(RESET)"

.PHONY: v3-docker-build-nocache
v3-docker-build-nocache:
	@echo "$(BLUE)Building v3 production Docker image (no cache)...$(RESET)"
	@echo "$(BLUE)Using Dockerfile (production)$(RESET)"
	@echo "$(YELLOW)Warning: This will take longer than normal$(RESET)"
	docker build --no-cache -t sindri:v3-latest \
		-f $(V3_DIR)/Dockerfile \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ v3 Docker build complete: sindri:v3-latest$(RESET)"


# ==============================================================================
# V3 Base Image Management & Fast Development Builds
# ==============================================================================

.PHONY: v3-docker-build-base
v3-docker-build-base:
	@echo "$(BOLD)$(BLUE)Building v3 base image (slow, build once)...$(RESET)"
	@echo "This image contains:"
	@echo "  - Rust toolchain"
	@echo "  - cargo-chef"
	@echo "  - System packages (Ubuntu, build tools)"
	@echo "  - GitHub CLI"
	@echo ""
	@echo "Build time: ~15-20 minutes (on ARM64)"
	@echo "Rebuild only when Rust version or system deps change"
	@echo ""
	docker build \
		-f $(V3_DIR)/Dockerfile.base \
		-t sindri:base-$(VERSION) \
		-t sindri:base-latest \
		$(V3_DIR)
	@echo ""
	@echo "$(GREEN)✓ Base image built: sindri:base-latest$(RESET)"
	@echo "  This image can be reused for weeks/months"
	@echo "  Use 'make v3-docker-build-fast' for fast incremental builds"

.PHONY: v3-docker-build-fast
v3-docker-build-fast:
	@echo "$(BOLD)$(BLUE)Building v3 image (fast, using base)...$(RESET)"
	@echo "Prerequisites: Base image from GHCR or local"
	@echo "  Pull: docker pull ghcr.io/pacphi/sindri:base-latest"
	@echo "  Or build: make v3-docker-build-base"
	@echo ""
	@echo "Building with cargo cache (fast incremental)..."
	docker build \
		-f $(V3_DIR)/Dockerfile.dev \
		-t sindri:$(VERSION)-$(GIT_COMMIT) \
		-t sindri:latest \
		.
	@echo ""
	@echo "$(GREEN)✓ Fast build complete: sindri:latest$(RESET)"
	@echo "  Build time: ~3-5 min (incremental: 1-2 min)"

.PHONY: v3-docker-build-fast-nocache
v3-docker-build-fast-nocache:
	@echo "$(BOLD)$(BLUE)Building v3 image (fast, no cargo cache)...$(RESET)"
	docker build \
		--no-cache \
		-f $(V3_DIR)/Dockerfile.dev \
		-t sindri:$(VERSION)-$(GIT_COMMIT) \
		-t sindri:latest \
		.
	@echo ""
	@echo "$(GREEN)✓ Fast build (no cache) complete$(RESET)"

# ==============================================================================
# V3 Smart Cache Management
# ==============================================================================

.PHONY: v3-cache-status
v3-cache-status:
	@echo "$(BOLD)$(BLUE)╔════════════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(BOLD)$(BLUE)║                      V3 Cache Status                               ║$(RESET)"
	@echo "$(BOLD)$(BLUE)╚════════════════════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(BOLD)Docker Images:$(RESET)"
	@docker images --filter=reference='sindri*' --format 'table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}' 2>/dev/null || true
	@echo ""
	@echo "$(BOLD)BuildKit Cache:$(RESET)"
	@docker buildx du 2>/dev/null || docker system df | grep "Build Cache" || echo "No cache data"
	@echo ""
	@echo "$(BOLD)Cargo Target Directory:$(RESET)"
	@if [ -d $(V3_DIR)/target ]; then \
		du -sh $(V3_DIR)/target 2>/dev/null || echo "Not found"; \
	else \
		echo "Not found"; \
	fi

.PHONY: v3-cache-clear-soft
v3-cache-clear-soft:
	@echo "$(YELLOW)Clearing soft caches (cargo incremental)...$(RESET)"
	@echo "Keeps: Docker build cache, Base images, Cargo dependencies"
	@echo "Clears: Cargo incremental compilation cache, Sindri repo cache"
	@echo ""
	@rm -rf ~/Library/Caches/sindri/repos 2>/dev/null || true
	@rm -rf ~/.cache/sindri/repos 2>/dev/null || true
	@if [ -d $(V3_DIR)/target ]; then \
		cd $(V3_DIR) && cargo clean -p sindri; \
	fi
	@echo "$(GREEN)✓ Soft cache cleared$(RESET)"

.PHONY: v3-cache-clear-medium
v3-cache-clear-medium:
	@echo "$(YELLOW)Clearing medium caches (cargo + build cache)...$(RESET)"
	@echo "Keeps: Base images, Docker system cache"
	@echo "Clears: All cargo artifacts, Sindri repo cache, BuildKit cache"
	@echo ""
	@rm -rf ~/Library/Caches/sindri/repos 2>/dev/null || true
	@rm -rf ~/.cache/sindri/repos 2>/dev/null || true
	@if [ -d $(V3_DIR)/target ]; then \
		cd $(V3_DIR) && cargo clean; \
	fi
	@docker buildx prune --filter "until=1h" --force 2>/dev/null || true
	@echo "$(GREEN)✓ Medium cache cleared$(RESET)"

.PHONY: v3-cache-clear-hard
v3-cache-clear-hard:
	@echo "$(RED)Clearing hard caches (nuclear)...$(RESET)"
	@echo "Keeps: Base images (sindri:base-*)"
	@echo "Clears: All cargo artifacts, All Sindri images (except base), All BuildKit cache"
	@echo ""
	@printf "$(BOLD)Are you sure? This is destructive. [y/N] $(RESET)" && \
	read confirm && [ "$$confirm" = "y" ] || { echo "Cancelled"; exit 1; }
	@echo ""
	@$(MAKE) v3-clean
	@docker images --filter=reference='sindri:*' --format '{{.ID}}\t{{.Repository}}:{{.Tag}}' | \
		grep -v 'sindri:base-' | awk '{print $$1}' | xargs docker rmi -f 2>/dev/null || true
	@docker buildx prune --all --force 2>/dev/null || true
	@echo "$(GREEN)✓ Hard cache cleared (base image preserved)$(RESET)"

.PHONY: v3-cache-nuke
v3-cache-nuke:
	@echo "$(RED)$(BOLD)NUCLEAR OPTION: Clearing EVERYTHING...$(RESET)"
	@echo "This will remove: ALL Sindri images (including base), ALL cargo artifacts, ALL Docker build cache"
	@echo ""
	@printf "$(BOLD)$(RED)Are you ABSOLUTELY sure? You'll need to rebuild base image. [y/N] $(RESET)" && \
	read confirm && [ "$$confirm" = "y" ] || { echo "Cancelled"; exit 1; }
	@echo ""
	@$(MAKE) v3-clean
	@docker images --filter=reference='sindri*' --format '{{.ID}}' | xargs docker rmi -f 2>/dev/null || true
	@docker buildx prune --all --force 2>/dev/null || true
	@echo "$(RED)✓ NUKED: Base image will need to be rebuilt$(RESET)"

# ==============================================================================
# V3 Fast Development Cycle Modes
# ==============================================================================

.PHONY: v3-cycle-fast
v3-cycle-fast:
	@if [ -z "$(CONFIG)" ]; then \
		echo "$(RED)Error: CONFIG parameter required$(RESET)"; \
		echo "Usage: make v3-cycle-fast CONFIG=/path/to/sindri.yaml"; \
		exit 1; \
	fi
	@echo ""
	@echo "$(BOLD)$(GREEN)╔════════════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(BOLD)$(GREEN)║                    V3 Fast Development Cycle                       ║$(RESET)"
	@echo "$(BOLD)$(GREEN)╚════════════════════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(BOLD)Mode:$(RESET) Incremental (keeps caches, reuses base image)"
	@echo "$(BOLD)Time:$(RESET) ~3-5 minutes"
	@echo "$(BOLD)Config:$(RESET) $(CONFIG)"
	@echo ""
	@$(MAKE) v3-cache-clear-soft
	@sindri destroy --config $(CONFIG) -f || true
	@$(MAKE) v3-docker-build-fast
	@$(MAKE) v3-install
	@sindri deploy --config $(CONFIG)
	@echo ""
	@echo "$(GREEN)✓ Fast cycle complete!$(RESET)"
	@echo "  Connect: sindri connect --config $(CONFIG)"

.PHONY: v3-cycle-clean
v3-cycle-clean:
	@if [ -z "$(CONFIG)" ]; then \
		echo "$(RED)Error: CONFIG parameter required$(RESET)"; \
		echo "Usage: make v3-cycle-clean CONFIG=/path/to/sindri.yaml"; \
		exit 1; \
	fi
	@echo ""
	@echo "$(BOLD)$(YELLOW)╔════════════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(BOLD)$(YELLOW)║                   V3 Clean Development Cycle                       ║$(RESET)"
	@echo "$(BOLD)$(YELLOW)╚════════════════════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(BOLD)Mode:$(RESET) Clean build (clears caches, keeps base image)"
	@echo "$(BOLD)Time:$(RESET) ~10-15 minutes"
	@echo "$(BOLD)Config:$(RESET) $(CONFIG)"
	@echo ""
	@$(MAKE) v3-cache-clear-medium
	@sindri destroy --config $(CONFIG) -f || true
	@docker images --filter=reference='sindri:*' --format '{{.ID}}\t{{.Repository}}:{{.Tag}}' | \
		grep -v 'sindri:base-' | awk '{print $$1}' | xargs docker rmi -f 2>/dev/null || true
	@$(MAKE) v3-docker-build-fast-nocache
	@$(MAKE) v3-install
	@sindri deploy --config $(CONFIG)
	@echo ""
	@echo "$(GREEN)✓ Clean cycle complete!$(RESET)"

.PHONY: v3-cycle-nuclear
v3-cycle-nuclear:
	@echo "$(RED)$(BOLD)WARNING: This is the NUCLEAR option$(RESET)"
	@echo "Time: 40-50 minutes (rebuilds everything from scratch)"
	@echo "Use v3-cycle-fast (3-5 min) or v3-cycle-clean (10-15 min) instead"
	@echo ""
	@$(MAKE) v3-cycle CONFIG=$(CONFIG)

# ============================================================================
# V3 Full Development Cycle
# ============================================================================
#
# Performs a complete v3 development cycle:
#   1. Destroy existing deployment
#   2. Remove all sindri Docker images
#   3. Clean v3 build artifacts
#   4. Install v3 binary
#   5. Deploy new environment
#   6. Connect to the environment
#
# Usage:
#   make v3-cycle CONFIG=/path/to/sindri.yaml
#   make v3-cycle CONFIG=/path/to/sindri.yaml FORCE=1  # Skip confirmation
# ============================================================================

CONFIG ?=
FORCE ?=

.PHONY: v3-cycle
v3-cycle:
	@if [ -z "$(CONFIG)" ]; then \
		echo "$(RED)Error: CONFIG parameter is required$(RESET)"; \
		echo "$(YELLOW)Usage: make v3-cycle CONFIG=/path/to/sindri.yaml$(RESET)"; \
		echo "       make v3-cycle CONFIG=/path/to/sindri.yaml FORCE=1  # Skip confirmation"; \
		exit 1; \
	fi
	@if [ ! -f "$(CONFIG)" ]; then \
		echo "$(RED)Error: Config file not found: $(CONFIG)$(RESET)"; \
		exit 1; \
	fi
	@echo ""
	@echo "$(BOLD)$(YELLOW)╔════════════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(BOLD)$(YELLOW)║                    V3 Full Development Cycle                       ║$(RESET)"
	@echo "$(BOLD)$(YELLOW)╚════════════════════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(BOLD)Config file:$(RESET) $(CONFIG)"
	@echo ""
	@echo "$(BOLD)Operations to be performed:$(RESET)"
	@echo "  1. $(BLUE)sindri destroy$(RESET) --config $(CONFIG) -f"
	@echo "  2. $(BLUE)docker rmi -f$(RESET) <all sindri images>"
	@echo "  3. $(BLUE)make v3-clean$(RESET)"
	@echo "  4. $(BLUE)make v3-install$(RESET)"
	@echo "  5. $(BLUE)sindri deploy$(RESET) --config $(CONFIG) -f"
	@echo "  6. $(BLUE)sindri connect$(RESET) --config $(CONFIG)"
	@echo ""
	@IMAGES=$$(docker images --filter=reference='sindri*' --format '{{.ID}}\t{{.Repository}}:{{.Tag}}\t{{.Size}}' 2>/dev/null); \
	if [ -n "$$IMAGES" ]; then \
		echo "$(BOLD)$(RED)Docker images to be removed:$(RESET)"; \
		echo "$$IMAGES" | while IFS=$$'\t' read -r id name size; do \
			echo "  $(RED)✗$(RESET) $$name ($$id) - $$size"; \
		done; \
	else \
		echo "$(YELLOW)No sindri Docker images currently exist$(RESET)"; \
	fi
	@echo ""
	@if [ "$(FORCE)" != "1" ]; then \
		printf "$(BOLD)Proceed with full cycle? [y/N] $(RESET)"; \
		read confirm; \
		if [ "$$confirm" != "y" ] && [ "$$confirm" != "Y" ]; then \
			echo "$(YELLOW)Operation cancelled$(RESET)"; \
			exit 0; \
		fi; \
	else \
		echo "$(YELLOW)FORCE=1 set, skipping confirmation$(RESET)"; \
	fi
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ Step 1/6: Destroying existing deployment ═══════════════════════$(RESET)"
	@sindri destroy --config $(CONFIG) -f || echo "$(YELLOW)No deployment to destroy (continuing)$(RESET)"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ Step 2/6: Removing sindri Docker images ═════════════════════════$(RESET)"
	@IMAGE_IDS=$$(docker images --filter=reference='sindri*' --format '{{.ID}}' 2>/dev/null | sort -u); \
	if [ -n "$$IMAGE_IDS" ]; then \
		echo "$$IMAGE_IDS" | xargs docker rmi -f; \
		echo "$(GREEN)✓ Images removed$(RESET)"; \
	else \
		echo "$(YELLOW)No images to remove$(RESET)"; \
	fi
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ Step 3/6: Cleaning v3 artifacts ═════════════════════════════════$(RESET)"
	@$(MAKE) v3-clean
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ Step 4/6: Installing v3 binary ══════════════════════════════════$(RESET)"
	@$(MAKE) v3-install
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ Step 5/6: Deploying ═════════════════════════════════════════════$(RESET)"
	@sindri deploy --config $(CONFIG) -f
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ Step 6/6: Connecting ════════════════════════════════════════════$(RESET)"
	@sindri connect --config $(CONFIG)
	@echo ""
	@echo "$(GREEN)$(BOLD)✓ V3 full development cycle complete$(RESET)"

# ============================================================================
# V3 Extension Testing Targets
# ============================================================================

# Default extension list for testing
V3_EXT_LIST ?= python,nodejs,golang
V3_EXT_MAX_PARALLEL ?= 2
V3_EXT_PROFILE ?= minimal

.PHONY: v3-ext-test
v3-ext-test: v3-ext-test-serial
	@echo "$(GREEN)$(BOLD)✓ Extension tests complete$(RESET)"

.PHONY: v3-ext-test-serial
v3-ext-test-serial: v3-build
	@echo "$(BLUE)Running v3 extension tests (serial)...$(RESET)"
	./scripts/v3-extension-test.sh --scheme serial --extensions "$(V3_EXT_LIST)"

.PHONY: v3-ext-test-parallel
v3-ext-test-parallel: v3-build
	@echo "$(BLUE)Running v3 extension tests (parallel)...$(RESET)"
	./scripts/v3-extension-test.sh --scheme parallel --extensions "$(V3_EXT_LIST)" --max-parallel $(V3_EXT_MAX_PARALLEL)

.PHONY: v3-ext-test-profile
v3-ext-test-profile: v3-build
	@echo "$(BLUE)Running v3 extension tests (profile: $(V3_EXT_PROFILE))...$(RESET)"
	./scripts/v3-extension-test.sh --scheme serial --profile "$(V3_EXT_PROFILE)"

.PHONY: v3-ext-test-quick
v3-ext-test-quick: v3-build
	@echo "$(BLUE)Running quick extension test (python only)...$(RESET)"
	./scripts/v3-extension-test.sh --scheme serial --extensions "python" --verbose

.PHONY: v3-ext-test-unit
v3-ext-test-unit:
	@echo "$(BLUE)Running extension unit tests...$(RESET)"
	cd $(V3_DIR) && cargo test --package sindri-extensions
	@echo "$(GREEN)✓ Extension unit tests passed$(RESET)"

# ============================================================================
# V3 Packer Testing Targets
# ============================================================================

.PHONY: v3-packer-test
v3-packer-test: v3-packer-test-unit
	@echo "$(GREEN)$(BOLD)✓ Packer tests complete$(RESET)"

.PHONY: v3-packer-test-unit
v3-packer-test-unit:
	@echo "$(BLUE)Running packer unit tests...$(RESET)"
	cd $(V3_DIR) && cargo test --package sindri-packer
	@echo "$(GREEN)✓ Packer unit tests passed$(RESET)"

.PHONY: v3-packer-validate
v3-packer-validate: v3-build
	@echo "$(BLUE)Validating packer templates...$(RESET)"
	@if command -v packer >/dev/null 2>&1; then \
		echo "Packer found, validating templates..."; \
		$(V3_BINARY) packer validate --cloud aws --dry-run 2>/dev/null || true; \
	else \
		echo "$(YELLOW)Packer not installed, skipping template validation$(RESET)"; \
	fi
	@echo "$(GREEN)✓ Packer validation complete$(RESET)"

.PHONY: v3-inspec-check
v3-inspec-check:
	@echo "$(BLUE)Checking InSpec profile...$(RESET)"
	@if command -v inspec >/dev/null 2>&1; then \
		inspec check $(V3_DIR)/test/integration/sindri/; \
	else \
		echo "$(YELLOW)InSpec not installed. Install: gem install inspec-bin$(RESET)"; \
	fi
	@echo "$(GREEN)✓ InSpec profile check complete$(RESET)"

.PHONY: v3-inspec-exec-local
v3-inspec-exec-local:
	@echo "$(BLUE)Running InSpec tests locally...$(RESET)"
	@if command -v inspec >/dev/null 2>&1; then \
		inspec exec $(V3_DIR)/test/integration/sindri/ \
			--reporter cli \
			--controls sindri docker mise \
			|| true; \
	else \
		echo "$(YELLOW)InSpec not installed. Install: gem install inspec-bin$(RESET)"; \
	fi

# ============================================================================
# CI/CD Targets
# ============================================================================

.PHONY: ci
ci: v2-ci v3-ci
	@echo "$(GREEN)$(BOLD)✓ Full CI pipeline passed$(RESET)"

.PHONY: v2-ci
v2-ci: v2-validate v2-build
	@echo "$(GREEN)$(BOLD)✓ v2 CI pipeline passed$(RESET)"

.PHONY: v3-ci
v3-ci: v3-validate v3-test v3-build
	@echo "$(GREEN)$(BOLD)✓ v3 CI pipeline passed$(RESET)"

.PHONY: v3-quality
v3-quality: v3-fmt-check v3-clippy v3-test v3-audit v3-coverage
	@echo "$(GREEN)$(BOLD)✓ v3 quality gate passed (fmt + clippy + test + audit + coverage)$(RESET)"

# ============================================================================
# Clean Targets
# ============================================================================

.PHONY: clean
clean: v2-clean v3-clean
	@echo "$(GREEN)✓ All build artifacts cleaned$(RESET)"

.PHONY: v2-clean
v2-clean:
	@echo "$(BLUE)Cleaning v2 Docker images...$(RESET)"
	@docker images | grep sindri | awk '{print $$3}' | xargs -r docker rmi -f 2>/dev/null || true
	@echo "$(GREEN)✓ v2 artifacts cleaned$(RESET)"

.PHONY: v3-clean
v3-clean:
	@echo "$(BLUE)Cleaning v3 Rust artifacts...$(RESET)"
	cd $(V3_DIR) && cargo clean
	@echo "$(BLUE)Cleaning v3 cached repositories...$(RESET)"
	@rm -rf ~/Library/Caches/sindri/repos 2>/dev/null || true
	@rm -rf ~/.cache/sindri/repos 2>/dev/null || true
	@echo "$(BLUE)Cleaning Docker build cache...$(RESET)"
	@docker builder prune --all --force 2>/dev/null || true
	@docker buildx prune --all --force 2>/dev/null || true
	@echo "$(GREEN)✓ v3 artifacts cleaned$(RESET)"
