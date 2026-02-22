# ============================================================================
# makefiles/common.mk — Shared Variables, OS Detection, Colors, Macros
# ============================================================================

PROJECT_ROOT := $(CURDIR)
V2_DIR := $(PROJECT_ROOT)/v2
V3_DIR := $(PROJECT_ROOT)/v3
V3_BINARY := $(V3_DIR)/target/release/sindri
V3_DEBUG_BINARY := $(V3_DIR)/target/debug/sindri

# Console Agent (Go binary)
CONSOLE_AGENT_DIR := $(V3_DIR)/console/agent
GO_BIN := go

# Console TypeScript monorepo
CONSOLE_DIR := $(V3_DIR)/console

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

# v3-cycle parameters
CONFIG ?=
FORCE ?=

# Default extension vars
V3_EXT_LIST ?= python,nodejs,golang
V3_EXT_MAX_PARALLEL ?= 2
V3_EXT_PROFILE ?= minimal

# ============================================================================
# Dependency Checking Macros
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
