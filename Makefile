# ============================================================================
# Sindri v3 Makefile
# ============================================================================
# v3 (Rust) implementation Makefile. After the April 2026 reorg, this branch
# carries only v3 source. The v2-* targets remain temporarily (FIXME: trim
# them in a future PR — see issue tracker for follow-up).
#
# Quick Start:
#   make help          - Show all available targets
#   make v3-build      - Build the Rust binary
#   make v3-test       - Run tests
#   make v3-ci         - Run full v3 CI suite
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
# Multi-Distro Configuration
# ============================================================================

# Default distro for local builds — matches Dockerfile ARG default.
# Override: make v3-docker-build DISTRO=fedora
DISTRO ?= ubuntu

# Validated distro values (guard used in distro-aware targets)
VALID_DISTROS := ubuntu fedora opensuse

# Version pins for distro base images (keep in sync with Dockerfile ARGs)
UBUNTU_VERSION   ?= 24.04
FEDORA_VERSION   ?= 41
OPENSUSE_VERSION ?= 15.6

# Computed local image name including distro suffix
# Examples: sindri:v3-ubuntu-local, sindri:v3-fedora-local
DISTRO_IMAGE_LOCAL     := sindri:v3-$(DISTRO)-local
DISTRO_IMAGE_VERSIONED := sindri:$(VERSION)-$(GIT_COMMIT)-$(DISTRO)

# Target registry (for remote push targets)
REGISTRY ?= ghcr.io/pacphi

# Base image for dev builds.  Defaults to GHCR; set to local tag for offline builds:
#   make v3-docker-build-dev-ubuntu BASE_IMAGE=sindri:base-ubuntu-latest
BASE_IMAGE ?= $(REGISTRY)/sindri:base-$(DISTRO)-latest

# ─── Guard macro: aborts if DISTRO is not in VALID_DISTROS ──────────────────
define assert_valid_distro
	@if ! echo " $(VALID_DISTROS) " | grep -q " $(DISTRO) "; then \
		echo "$(RED)✗ Unknown DISTRO: $(DISTRO)$(RESET)"; \
		echo "  Valid values: $(VALID_DISTROS)"; \
		exit 1; \
	fi
endef

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
	v3-validate v3-validate-yaml v3-validate-rust v3-validate-compat \
	v3-clippy v3-fmt v3-fmt-check \
	v3-audit v3-audit-fix \
	v3-coverage v3-coverage-html v3-coverage-lcov \
	v3-doc v3-run v3-run-debug v3-install \
	v3-docker-build v3-docker-build-latest v3-docker-build-nocache \
	v3-docker-build-from-source v3-docker-build-from-binary \
	v3-docker-build-ubuntu v3-docker-build-fedora \
	v3-docker-build-opensuse v3-docker-build-all \
	v3-docker-build-dev v3-docker-build-dev-ubuntu v3-docker-build-dev-fedora \
	v3-docker-build-dev-opensuse v3-docker-build-dev-all \
	v3-docker-build-base-ubuntu v3-docker-build-base-fedora \
	v3-docker-build-base-opensuse v3-docker-build-base-all \
	v3-distro-test v3-distro-test-ubuntu v3-distro-test-fedora \
	v3-distro-test-opensuse v3-distro-test-all \
	v3-pkg-manager-test \
	v3-cache-clear-distro \
	_v3-docker-build-impl _v3-docker-build-dev-impl _v3-docker-build-base-impl \
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
	@echo "  v3-validate            - Validate Rust code (fmt + clippy + YAML + compat)"
	@echo "  v3-validate-yaml       - Validate v3 YAML files"
	@echo "  v3-validate-rust       - Validate Rust code only"
	@echo "  v3-validate-compat     - Validate compatibility matrix"
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
	@echo "$(BOLD)$(BLUE)═══ V3 Multi-Distro Build Targets ══════════════════════════════════$(RESET)"
	@echo "  v3-docker-build           - Build image (DISTRO=ubuntu|fedora|opensuse, default: ubuntu)"
	@echo "  v3-docker-build-ubuntu    - Build Ubuntu image locally"
	@echo "  v3-docker-build-fedora    - Build Fedora 41 image locally"
	@echo "  v3-docker-build-opensuse  - Build openSUSE Leap 15.6 image locally"
	@echo "  v3-docker-build-all       - Build all three distro images sequentially"
	@echo "  v3-docker-build-dev       - Build DEV image (DISTRO=..., from source)"
	@echo "  v3-docker-build-dev-ubuntu    - Build Ubuntu dev image"
	@echo "  v3-docker-build-dev-fedora    - Build Fedora dev image"
	@echo "  v3-docker-build-dev-opensuse  - Build openSUSE dev image"
	@echo "  v3-docker-build-dev-all       - Build all three dev distro images"
	@echo "  v3-docker-build-base      - Build base image (DISTRO=...)"
	@echo "  v3-docker-build-base-all  - Build all three base images"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ V3 Distro Testing ════════════════════════════════════════════════$(RESET)"
	@echo "  v3-distro-test            - Smoke test local image (DISTRO=ubuntu|fedora|opensuse)"
	@echo "  v3-distro-test-ubuntu     - Smoke test Ubuntu local image"
	@echo "  v3-distro-test-fedora     - Smoke test Fedora local image"
	@echo "  v3-distro-test-opensuse   - Smoke test openSUSE local image"
	@echo "  v3-distro-test-all        - Build and test all three distros"
	@echo "  v3-pkg-manager-test       - Run pkg-manager.sh Docker-based integration tests"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ V3 Cache (distro-aware) ═════════════════════════════════════════$(RESET)"
	@echo "  v3-cache-status           - Show all distro images and cache usage"
	@echo "  v3-cache-clear-distro     - Remove images for one distro (DISTRO=...)"
	@echo "  v3-clean                  - Clean artifacts; add DISTRO= to target one distro"
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
v3-validate: v3-validate-yaml v3-validate-rust v3-validate-compat
	@echo "$(GREEN)$(BOLD)✓ All v3 validation passed$(RESET)"

.PHONY: v3-validate-compat
v3-validate-compat:
	@echo "$(BLUE)Validating compatibility matrix...$(RESET)"
	python3 scripts/validate-compat-matrix.py
	@echo "$(GREEN)✓ Compatibility matrix validation passed$(RESET)"

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
	@command -v cargo-audit >/dev/null 2>&1 || { \
		echo "$(BLUE)Installing cargo-audit...$(RESET)"; \
		cargo install cargo-audit; \
	}
	.github/scripts/v3/cargo-audit.sh
	@echo "$(GREEN)✓ Security audit complete$(RESET)"

.PHONY: v3-audit-fix
v3-audit-fix:
	@echo "$(BLUE)Fixing v3 security vulnerabilities...$(RESET)"
	@if ! cargo audit fix --help >/dev/null 2>&1; then \
		echo "$(BLUE)Installing cargo-audit with fix feature...$(RESET)"; \
		cargo install cargo-audit --features=fix; \
	fi
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
# V3 Docker Build Targets (Multi-Distro)
# ============================================================================

# Internal implementation — do not call directly; use named distro targets.
.PHONY: _v3-docker-build-impl
_v3-docker-build-impl:
	$(call assert_valid_distro)
	@echo "$(BLUE)Building Sindri v3 Docker image [distro=$(DISTRO)]...$(RESET)"
	@echo "$(BLUE)→ Dockerfile: v3/Dockerfile$(RESET)"
	@echo "$(BLUE)→ Tags: $(DISTRO_IMAGE_LOCAL)$(RESET)"
	docker build \
		--build-arg DISTRO=$(DISTRO) \
		--build-arg UBUNTU_VERSION=$(UBUNTU_VERSION) \
		--build-arg FEDORA_VERSION=$(FEDORA_VERSION) \
		--build-arg OPENSUSE_VERSION=$(OPENSUSE_VERSION) \
		-t $(DISTRO_IMAGE_LOCAL) \
		-t $(DISTRO_IMAGE_VERSIONED) \
		-f $(V3_DIR)/Dockerfile \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ Build complete: $(DISTRO_IMAGE_LOCAL)$(RESET)"

# ── Convenience aliases ───────────────────────────────────────────────────────
.PHONY: v3-docker-build-ubuntu
v3-docker-build-ubuntu:
	@$(MAKE) _v3-docker-build-impl DISTRO=ubuntu

.PHONY: v3-docker-build-fedora
v3-docker-build-fedora:
	@$(MAKE) _v3-docker-build-impl DISTRO=fedora

.PHONY: v3-docker-build-opensuse
v3-docker-build-opensuse:
	@$(MAKE) _v3-docker-build-impl DISTRO=opensuse

# ── Build all three distros sequentially ─────────────────────────────────────
.PHONY: v3-docker-build-all
v3-docker-build-all: v3-docker-build-ubuntu v3-docker-build-fedora v3-docker-build-opensuse
	@echo ""
	@echo "$(GREEN)$(BOLD)✓ All three distro images built:$(RESET)"
	@docker images --filter="reference=sindri:v3-*-local" \
		--format "table {{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}"

# ── DISTRO-parameterised generic target (for CI scripts) ─────────────────────
#   Usage: make v3-docker-build DISTRO=fedora
.PHONY: v3-docker-build
v3-docker-build:
	@$(MAKE) _v3-docker-build-impl

# ── Backward compatibility aliases ───────────────────────────────────────────
.PHONY: v3-docker-build-from-binary
v3-docker-build-from-binary: v3-docker-build

.PHONY: v3-docker-build-latest
v3-docker-build-latest:
	@echo "$(YELLOW)Note: v3-docker-build-latest now builds ubuntu variant$(RESET)"
	@$(MAKE) _v3-docker-build-impl DISTRO=ubuntu

.PHONY: v3-docker-build-nocache
v3-docker-build-nocache:
	$(call assert_valid_distro)
	@echo "$(BLUE)Building Sindri v3 Docker image (no cache) [distro=$(DISTRO)]...$(RESET)"
	@echo "$(YELLOW)Warning: This will take longer than normal$(RESET)"
	docker build --no-cache \
		--build-arg DISTRO=$(DISTRO) \
		--build-arg UBUNTU_VERSION=$(UBUNTU_VERSION) \
		--build-arg FEDORA_VERSION=$(FEDORA_VERSION) \
		--build-arg OPENSUSE_VERSION=$(OPENSUSE_VERSION) \
		-t $(DISTRO_IMAGE_LOCAL) \
		-f $(V3_DIR)/Dockerfile \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ Build complete (no cache): $(DISTRO_IMAGE_LOCAL)$(RESET)"

# ─────────────────────────────────────────────────────────────────────────────
# Dev image (Dockerfile.dev — bundled extensions, from source)
# ─────────────────────────────────────────────────────────────────────────────

.PHONY: _v3-docker-build-dev-impl
_v3-docker-build-dev-impl:
	$(call assert_valid_distro)
	@echo "$(BLUE)Building Sindri v3 DEV image [distro=$(DISTRO)] from source...$(RESET)"
	@echo "$(BLUE)→ Dockerfile: v3/Dockerfile.dev  (~3-5 min)$(RESET)"
	@echo "$(BLUE)→ Base image: $(BASE_IMAGE)$(RESET)"
	@# Auto-build base image if not available locally or remotely, then build dev.
	@# Everything runs in a single shell so 'exit 0' prevents the fallthrough build.
	@EFFECTIVE_BASE="$(BASE_IMAGE)"; \
	if ! docker image inspect "$$EFFECTIVE_BASE" >/dev/null 2>&1; then \
		echo "$(YELLOW)Base image $$EFFECTIVE_BASE not found locally.$(RESET)"; \
		if ! docker manifest inspect "$$EFFECTIVE_BASE" >/dev/null 2>&1; then \
			echo "$(YELLOW)Base image $$EFFECTIVE_BASE not found in remote registry either.$(RESET)"; \
			echo "$(BLUE)Auto-building base image for $(DISTRO) (this may take 15-20 min on first run)...$(RESET)"; \
			$(MAKE) _v3-docker-build-base-impl DISTRO=$(DISTRO); \
			EFFECTIVE_BASE="sindri:base-$(DISTRO)-latest"; \
			echo "$(GREEN)✓ Base image built locally. Using $$EFFECTIVE_BASE$(RESET)"; \
		fi; \
	fi; \
	docker build \
		--build-arg DISTRO=$(DISTRO) \
		--build-arg BASE_IMAGE=$$EFFECTIVE_BASE \
		-t sindri:v3-$(DISTRO)-dev \
		-t sindri:$(VERSION)-$(GIT_COMMIT)-$(DISTRO)-dev \
		-f $(V3_DIR)/Dockerfile.dev \
		$(PROJECT_ROOT); \
	echo "$(GREEN)✓ Dev image complete: sindri:v3-$(DISTRO)-dev$(RESET)"

.PHONY: v3-docker-build-dev-ubuntu
v3-docker-build-dev-ubuntu:
	@$(MAKE) _v3-docker-build-dev-impl DISTRO=ubuntu

.PHONY: v3-docker-build-dev-fedora
v3-docker-build-dev-fedora:
	@$(MAKE) _v3-docker-build-dev-impl DISTRO=fedora

.PHONY: v3-docker-build-dev-opensuse
v3-docker-build-dev-opensuse:
	@$(MAKE) _v3-docker-build-dev-impl DISTRO=opensuse

.PHONY: v3-docker-build-dev
v3-docker-build-dev:
	@$(MAKE) _v3-docker-build-dev-impl

# ── Build all three dev distros sequentially ─────────────────────────────────
.PHONY: v3-docker-build-dev-all
v3-docker-build-dev-all: v3-docker-build-dev-ubuntu v3-docker-build-dev-fedora v3-docker-build-dev-opensuse
	@echo ""
	@echo "$(GREEN)$(BOLD)✓ All three dev distro images built:$(RESET)"
	@docker images --filter="reference=sindri:v3-*-dev" \
		--format "table {{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}"

# Backward compat aliases
.PHONY: v3-docker-build-from-source
v3-docker-build-from-source: v3-docker-build-dev

.PHONY: v3-docker-build-fast
v3-docker-build-fast:
	@echo "$(YELLOW)Note: v3-docker-build-fast is now v3-docker-build-dev$(RESET)"
	@$(MAKE) _v3-docker-build-dev-impl

.PHONY: v3-docker-build-fast-nocache
v3-docker-build-fast-nocache:
	$(call assert_valid_distro)
	@echo "$(BLUE)Building v3 DEV image (no cache) [distro=$(DISTRO)]...$(RESET)"
	docker build --no-cache \
		--build-arg DISTRO=$(DISTRO) \
		--build-arg BASE_IMAGE=$(BASE_IMAGE) \
		-t sindri:v3-$(DISTRO)-dev \
		-f $(V3_DIR)/Dockerfile.dev \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ Dev build (no cache) complete$(RESET)"

# ─────────────────────────────────────────────────────────────────────────────
# Base image builds (one per distro)
# ─────────────────────────────────────────────────────────────────────────────

.PHONY: _v3-docker-build-base-impl
_v3-docker-build-base-impl:
	$(call assert_valid_distro)
	@echo "$(BOLD)$(BLUE)Building v3 base image [distro=$(DISTRO)]...$(RESET)"
	@echo "Build time: ~15-20 min (arm64). Rebuild on Rust version change."
	docker build \
		--build-arg DISTRO=$(DISTRO) \
		--build-arg UBUNTU_VERSION=$(UBUNTU_VERSION) \
		--build-arg FEDORA_VERSION=$(FEDORA_VERSION) \
		--build-arg OPENSUSE_VERSION=$(OPENSUSE_VERSION) \
		-t sindri:base-$(DISTRO)-$(VERSION) \
		-t sindri:base-$(DISTRO)-latest \
		-f $(V3_DIR)/Dockerfile.base \
		$(V3_DIR)
	@echo "$(GREEN)✓ Base image built: sindri:base-$(DISTRO)-latest$(RESET)"

.PHONY: v3-docker-build-base-ubuntu
v3-docker-build-base-ubuntu:
	@$(MAKE) _v3-docker-build-base-impl DISTRO=ubuntu

.PHONY: v3-docker-build-base-fedora
v3-docker-build-base-fedora:
	@$(MAKE) _v3-docker-build-base-impl DISTRO=fedora

.PHONY: v3-docker-build-base-opensuse
v3-docker-build-base-opensuse:
	@$(MAKE) _v3-docker-build-base-impl DISTRO=opensuse

# Usage: make v3-docker-build-base DISTRO=fedora
.PHONY: v3-docker-build-base
v3-docker-build-base:
	@$(MAKE) _v3-docker-build-base-impl

.PHONY: v3-docker-build-base-all
v3-docker-build-base-all: v3-docker-build-base-ubuntu v3-docker-build-base-fedora v3-docker-build-base-opensuse
	@echo "$(GREEN)$(BOLD)✓ All three base images built$(RESET)"

# ==============================================================================
# V3 Smart Cache Management
# ==============================================================================

.PHONY: v3-cache-status
v3-cache-status:
	@echo "$(BOLD)$(BLUE)╔══════════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(BOLD)$(BLUE)║                     V3 Cache Status                               ║$(RESET)"
	@echo "$(BOLD)$(BLUE)╚══════════════════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(BOLD)Base Images:$(RESET)"
	@docker images --filter="reference=sindri:base*" \
		--format "table {{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}" 2>/dev/null || true
	@echo ""
	@echo "$(BOLD)Local Build Images (by distro):$(RESET)"
	@for DISTRO in ubuntu fedora opensuse; do \
		echo "  [$$DISTRO]:"; \
		docker images --filter="reference=sindri:*$$DISTRO*" \
			--format "    {{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}" 2>/dev/null || true; \
	done
	@echo ""
	@echo "$(BOLD)BuildKit Cache (per distro scope):$(RESET)"
	@docker buildx du 2>/dev/null | head -20 || docker system df | grep "Build Cache" || echo "No cache data"
	@echo ""
	@echo "$(BOLD)Cargo Target:$(RESET)"
	@du -sh $(V3_DIR)/target 2>/dev/null || echo "Not built"

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

# Per-distro image cleanup — removes only images for one distro
# Usage: make v3-cache-clear-distro DISTRO=fedora
.PHONY: v3-cache-clear-distro
v3-cache-clear-distro:
	$(call assert_valid_distro)
	@echo "$(YELLOW)Removing all local images for distro=$(DISTRO)...$(RESET)"
	@docker images --filter="reference=sindri:*$(DISTRO)*" \
		--format "{{.ID}}" | sort -u | xargs docker rmi -f 2>/dev/null || true
	@echo "$(GREEN)✓ Removed all $(DISTRO) images$(RESET)"

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
		echo "$(RED)Error: CONFIG is required$(RESET)"; \
		echo "Usage: make v3-cycle-fast CONFIG=/path/to/sindri.yaml [DISTRO=ubuntu|fedora|opensuse]"; \
		exit 1; \
	fi
	$(call assert_valid_distro)
	@echo ""
	@echo "$(BOLD)$(GREEN)╔══════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(BOLD)$(GREEN)║         V3 Fast Development Cycle [$(DISTRO)]                    ║$(RESET)"
	@echo "$(BOLD)$(GREEN)╚══════════════════════════════════════════════════════════════╝$(RESET)"
	@echo "$(BOLD)Mode:$(RESET) Incremental  $(BOLD)Distro:$(RESET) $(DISTRO)  $(BOLD)Time:$(RESET) ~3-5 min"
	@echo ""
	@$(MAKE) v3-cache-clear-soft
	@sindri destroy --config $(CONFIG) -f || true
	@$(MAKE) _v3-docker-build-dev-impl
	@$(MAKE) v3-install
	@sindri deploy --config $(CONFIG)
	@echo ""
	@echo "$(GREEN)✓ Fast cycle [$(DISTRO)] complete — Connect: sindri connect --config $(CONFIG)$(RESET)"

.PHONY: v3-cycle-clean
v3-cycle-clean:
	@if [ -z "$(CONFIG)" ]; then \
		echo "$(RED)Error: CONFIG is required$(RESET)"; exit 1; \
	fi
	$(call assert_valid_distro)
	@echo ""
	@echo "$(BOLD)$(YELLOW)╔══════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(BOLD)$(YELLOW)║        V3 Clean Development Cycle [$(DISTRO)]                    ║$(RESET)"
	@echo "$(BOLD)$(YELLOW)╚══════════════════════════════════════════════════════════════╝$(RESET)"
	@echo "$(BOLD)Mode:$(RESET) Clean build  $(BOLD)Distro:$(RESET) $(DISTRO)  $(BOLD)Time:$(RESET) ~10-15 min"
	@echo ""
	@$(MAKE) v3-cache-clear-medium
	@sindri destroy --config $(CONFIG) -f || true
	@docker images --filter="reference=sindri:v3-$(DISTRO)*" \
		--format "{{.ID}}" | xargs docker rmi -f 2>/dev/null || true
	@$(MAKE) _v3-docker-build-dev-impl
	@$(MAKE) v3-install
	@sindri deploy --config $(CONFIG)
	@echo ""
	@echo "$(GREEN)✓ Clean cycle [$(DISTRO)] complete$(RESET)"

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
# V3 Distro Smoke Tests (local — mirrors the CI distro matrix job)
# ============================================================================

# Run smoke test on a locally-built distro image.
# Usage: make v3-distro-test DISTRO=fedora
.PHONY: v3-distro-test
v3-distro-test:
	$(call assert_valid_distro)
	@IMG="$(DISTRO_IMAGE_LOCAL)"; \
	if ! docker image inspect "$$IMG" >/dev/null 2>&1; then \
		echo "$(YELLOW)Image not found: $$IMG$(RESET)"; \
		echo "Build first: make v3-docker-build DISTRO=$(DISTRO)"; \
		exit 1; \
	fi; \
	echo "$(BLUE)Running smoke tests for $(DISTRO)...$(RESET)"; \
	\
	echo "  [1/4] sindri --version"; \
	docker run --rm "$$IMG" sindri --version; \
	\
	echo "  [2/4] distro detection"; \
	DETECTED=$$(docker run --rm "$$IMG" /bin/bash -c \
		"source /docker/lib/pkg-manager.sh && detect_distro"); \
	if [ "$$DETECTED" != "$(DISTRO)" ]; then \
		echo "$(RED)FAIL: expected $(DISTRO), got $$DETECTED$(RESET)"; exit 1; \
	fi; \
	echo "$(GREEN)  ✓ Distro detection: $$DETECTED$(RESET)"; \
	\
	echo "  [3/4] architecture detection"; \
	ARCH=$$(docker run --rm "$$IMG" /bin/bash -c \
		"source /docker/lib/pkg-manager.sh && detect_arch"); \
	echo "$(GREEN)  ✓ Architecture: $$ARCH$(RESET)"; \
	\
	echo "  [4/4] starship --version"; \
	docker run --rm "$$IMG" starship --version; \
	echo "$(GREEN)  ✓ starship available$(RESET)"; \
	\
	echo "$(GREEN)$(BOLD)✓ All smoke tests passed for $(DISTRO)$(RESET)"

# Convenience aliases
.PHONY: v3-distro-test-ubuntu v3-distro-test-fedora v3-distro-test-opensuse
v3-distro-test-ubuntu:   ; @$(MAKE) v3-distro-test DISTRO=ubuntu
v3-distro-test-fedora:   ; @$(MAKE) v3-distro-test DISTRO=fedora
v3-distro-test-opensuse: ; @$(MAKE) v3-distro-test DISTRO=opensuse

# Build and test all three in sequence
.PHONY: v3-distro-test-all
v3-distro-test-all:
	@echo "$(BLUE)Building and testing all distros...$(RESET)"
	@for DISTRO in ubuntu fedora opensuse; do \
		$(MAKE) v3-docker-build DISTRO=$$DISTRO && \
		$(MAKE) v3-distro-test  DISTRO=$$DISTRO || exit 1; \
	done
	@echo "$(GREEN)$(BOLD)✓ All distros built and tested$(RESET)"

# ── pkg-manager.sh integration tests (Docker-based) ─────────────────────────
.PHONY: v3-pkg-manager-test
v3-pkg-manager-test:
	@echo "$(BLUE)Running pkg-manager.sh integration tests (Docker-based)...$(RESET)"
	@if ! command -v docker >/dev/null 2>&1; then \
		echo "$(RED)Docker is required to run these tests$(RESET)"; \
		exit 1; \
	fi
	$(V3_DIR)/tests/pkg-manager-test.sh
	@echo "$(GREEN)✓ pkg-manager.sh integration tests passed$(RESET)"

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

# ============================================================================
# Dead Code Detection
# ============================================================================

.PHONY: v3-deadcode
v3-deadcode:
	@echo "$(BLUE)Scanning for unused Rust dependencies (cargo-machete)...$(RESET)"
	@if ! command -v cargo-machete >/dev/null 2>&1; then \
		echo "$(YELLOW)Installing cargo-machete...$(RESET)"; \
		cargo install cargo-machete; \
	fi
	cd $(V3_DIR) && cargo machete || true
	@echo "$(GREEN)✓ Dead code scan complete$(RESET)"

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
	@echo "$(BLUE)Cleaning Sindri repository caches...$(RESET)"
	@rm -rf ~/Library/Caches/sindri/repos 2>/dev/null || true
	@rm -rf ~/.cache/sindri/repos 2>/dev/null || true
	@if [ -n "$(DISTRO)" ] && [ "$(DISTRO)" != "." ]; then \
		echo "$(BLUE)Removing Docker images for distro=$(DISTRO)...$(RESET)"; \
		docker images --filter="reference=sindri:*$(DISTRO)*" \
			--format "{{.ID}}" | sort -u | xargs docker rmi -f 2>/dev/null || true; \
	else \
		echo "$(BLUE)Removing all non-base sindri Docker images...$(RESET)"; \
		docker images --filter="reference=sindri:*" --format "{{.ID}}\t{{.Tag}}" \
			| grep -v "base-" | awk '{print $$1}' \
			| xargs docker rmi -f 2>/dev/null || true; \
	fi
	@echo "$(BLUE)Clearing BuildKit build caches...$(RESET)"
	@docker builder prune --all --force 2>/dev/null || true
	@docker buildx prune --all --force 2>/dev/null || true
	@echo "$(GREEN)✓ v3 artifacts cleaned$(RESET)"
