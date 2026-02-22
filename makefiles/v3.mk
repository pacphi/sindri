# ============================================================================
# makefiles/v3.mk — V3 (Rust) Targets
# ============================================================================

.PHONY: v3-build v3-build-debug v3-check \
	v3-test v3-test-verbose v3-test-crate \
	v3-validate v3-validate-yaml v3-validate-rust \
	v3-clippy v3-fmt v3-fmt-check \
	v3-audit v3-audit-fix \
	v3-coverage v3-coverage-html v3-coverage-lcov \
	v3-doc v3-run v3-run-debug v3-install \
	v3-docker-build v3-docker-build-from-binary v3-docker-build-from-source \
	v3-docker-build-latest v3-docker-build-nocache \
	v3-docker-build-base v3-docker-build-fast v3-docker-build-fast-nocache \
	v3-cache-status v3-cache-clear-soft v3-cache-clear-medium \
	v3-cache-clear-hard v3-cache-nuke \
	v3-cycle-fast v3-cycle-clean v3-cycle-nuclear v3-cycle \
	v3-ext-test v3-ext-test-serial v3-ext-test-parallel \
	v3-ext-test-profile v3-ext-test-quick v3-ext-test-unit \
	v3-packer-test v3-packer-test-unit v3-packer-validate \
	v3-inspec-check v3-inspec-exec-local \
	v3-clean v3-ci v3-quality

# ============================================================================
# V3 Build Targets
# ============================================================================

v3-build:
	@echo "$(BLUE)Building v3 Rust binary (release mode)...$(RESET)"
	cd $(V3_DIR) && cargo build --release
	@echo "$(GREEN)✓ v3 build complete: $(V3_BINARY)$(RESET)"

v3-build-debug:
	@echo "$(BLUE)Building v3 Rust binary (debug mode)...$(RESET)"
	cd $(V3_DIR) && cargo build
	@echo "$(GREEN)✓ v3 debug build complete: $(V3_DEBUG_BINARY)$(RESET)"

v3-check:
	@echo "$(BLUE)Checking v3 Rust code (fast compile check)...$(RESET)"
	cd $(V3_DIR) && cargo check --workspace
	@echo "$(GREEN)✓ v3 check passed$(RESET)"

# ============================================================================
# V3 Testing Targets
# ============================================================================

v3-test:
	@echo "$(BLUE)Running v3 Rust tests...$(RESET)"
	cd $(V3_DIR) && cargo test --workspace
	@echo "$(GREEN)✓ v3 tests passed$(RESET)"

v3-test-verbose:
	@echo "$(BLUE)Running v3 Rust tests (verbose output)...$(RESET)"
	cd $(V3_DIR) && cargo test --workspace -- --nocapture
	@echo "$(GREEN)✓ v3 verbose tests passed$(RESET)"

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

v3-validate: v3-validate-yaml v3-validate-rust
	@echo "$(GREEN)$(BOLD)✓ All v3 validation passed$(RESET)"

v3-validate-yaml:
	@echo "$(BLUE)Validating v3 YAML files...$(RESET)"
	pnpm v3:validate:yaml
	@echo "$(GREEN)✓ v3 YAML validation passed$(RESET)"

v3-validate-rust: v3-fmt-check v3-clippy
	@echo "$(GREEN)✓ v3 Rust validation passed$(RESET)"

# ============================================================================
# V3 Linting Targets
# ============================================================================

v3-clippy:
	@echo "$(BLUE)Running clippy linter on v3...$(RESET)"
	cd $(V3_DIR) && cargo clippy --workspace --all-targets --all-features -- -D warnings
	@echo "$(GREEN)✓ clippy passed$(RESET)"

v3-fmt:
	@echo "$(BLUE)Formatting v3 Rust code...$(RESET)"
	cd $(V3_DIR) && cargo fmt --all
	@echo "$(GREEN)✓ Rust formatting complete$(RESET)"

v3-fmt-check:
	@echo "$(BLUE)Checking v3 Rust formatting...$(RESET)"
	cd $(V3_DIR) && cargo fmt --all -- --check
	@echo "$(GREEN)✓ Rust formatting check passed$(RESET)"

# ============================================================================
# V3 Security Targets
# ============================================================================

v3-audit:
	@echo "$(BLUE)Running v3 security audit...$(RESET)"
	.github/scripts/v3/cargo-audit.sh
	@echo "$(GREEN)✓ Security audit complete$(RESET)"

v3-audit-fix:
	@echo "$(BLUE)Fixing v3 security vulnerabilities...$(RESET)"
	cd $(V3_DIR) && cargo audit fix
	@echo "$(GREEN)✓ Security fixes applied$(RESET)"

# ============================================================================
# V3 Code Coverage (cargo-llvm-cov)
# ============================================================================

v3-coverage:
	@echo "$(BLUE)Running v3 code coverage (summary)...$(RESET)"
	@if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo "$(YELLOW)cargo-llvm-cov not installed.$(RESET)"; \
		echo "  Install: rustup component add llvm-tools-preview && cargo install cargo-llvm-cov"; \
		exit 1; \
	fi
	cd $(V3_DIR) && cargo llvm-cov --workspace
	@echo "$(GREEN)✓ Coverage report complete$(RESET)"

v3-coverage-html:
	@echo "$(BLUE)Generating v3 HTML coverage report...$(RESET)"
	@if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo "$(YELLOW)cargo-llvm-cov not installed.$(RESET)"; \
		echo "  Install: rustup component add llvm-tools-preview && cargo install cargo-llvm-cov"; \
		exit 1; \
	fi
	cd $(V3_DIR) && cargo llvm-cov --workspace --html --output-dir coverage/
	@echo "$(GREEN)✓ HTML report: $(V3_DIR)/coverage/html/index.html$(RESET)"

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

v3-doc:
	@echo "$(BLUE)Generating v3 Rust documentation...$(RESET)"
	cd $(V3_DIR) && cargo doc --workspace --no-deps --all-features --open
	@echo "$(GREEN)✓ Documentation generated and opened$(RESET)"

# ============================================================================
# V3 Binary Execution
# ============================================================================

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

v3-run-debug: v3-build-debug
	@if [ -z "$(ARGS)" ]; then \
		echo "$(YELLOW)Usage: make v3-run-debug ARGS=\"<command>\"$(RESET)"; \
		exit 1; \
	fi
	@echo "$(BLUE)Running (debug): $(V3_DEBUG_BINARY) $(ARGS)$(RESET)"
	@$(V3_DEBUG_BINARY) $(ARGS)

v3-install:
	@echo "$(BLUE)Installing sindri v3 to ~/.cargo/bin$(RESET)"
	cd $(V3_DIR) && cargo install --path crates/sindri
	@echo "$(GREEN)✓ Installed: $(shell which sindri 2>/dev/null || echo '~/.cargo/bin/sindri')$(RESET)"

# ============================================================================
# V3 Docker Build Targets
# ============================================================================

v3-docker-build: v3-docker-build-from-binary

v3-docker-build-from-binary:
	@echo "$(BLUE)Building v3 production Docker image from pre-compiled binary (local tag)...$(RESET)"
	@echo "$(BLUE)Using Dockerfile (production) - downloads binary from GitHub releases (~5 min)$(RESET)"
	docker build -t sindri:v3-local \
		-f $(V3_DIR)/Dockerfile \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ v3 Docker build complete: sindri:v3-local$(RESET)"

v3-docker-build-from-source:
	@echo "$(BLUE)Building v3 development Docker image from Rust source (~8 min)...$(RESET)"
	@echo "$(BLUE)Using Dockerfile.dev (development) - builds from source with bundled extensions$(RESET)"
	docker build -t sindri:v3-dev \
		-f $(V3_DIR)/Dockerfile.dev \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ v3 Docker build complete: sindri:v3-dev$(RESET)"

v3-docker-build-latest:
	@echo "$(BLUE)Building v3 production Docker image (latest tag)...$(RESET)"
	@echo "$(BLUE)Using Dockerfile (production)$(RESET)"
	docker build -t sindri:v3-latest \
		-f $(V3_DIR)/Dockerfile \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ v3 Docker build complete: sindri:v3-latest$(RESET)"

v3-docker-build-nocache:
	@echo "$(BLUE)Building v3 production Docker image (no cache)...$(RESET)"
	@echo "$(BLUE)Using Dockerfile (production)$(RESET)"
	@echo "$(YELLOW)Warning: This will take longer than normal$(RESET)"
	docker build --no-cache -t sindri:v3-latest \
		-f $(V3_DIR)/Dockerfile \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ v3 Docker build complete: sindri:v3-latest$(RESET)"

# ============================================================================
# V3 Base Image Management & Fast Development Builds
# ============================================================================

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

# ============================================================================
# V3 Smart Cache Management
# ============================================================================

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

# ============================================================================
# V3 Fast Development Cycle Modes
# ============================================================================

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

v3-ext-test: v3-ext-test-serial
	@echo "$(GREEN)$(BOLD)✓ Extension tests complete$(RESET)"

v3-ext-test-serial: v3-build
	@echo "$(BLUE)Running v3 extension tests (serial)...$(RESET)"
	./scripts/v3-extension-test.sh --scheme serial --extensions "$(V3_EXT_LIST)"

v3-ext-test-parallel: v3-build
	@echo "$(BLUE)Running v3 extension tests (parallel)...$(RESET)"
	./scripts/v3-extension-test.sh --scheme parallel --extensions "$(V3_EXT_LIST)" --max-parallel $(V3_EXT_MAX_PARALLEL)

v3-ext-test-profile: v3-build
	@echo "$(BLUE)Running v3 extension tests (profile: $(V3_EXT_PROFILE))...$(RESET)"
	./scripts/v3-extension-test.sh --scheme serial --profile "$(V3_EXT_PROFILE)"

v3-ext-test-quick: v3-build
	@echo "$(BLUE)Running quick extension test (python only)...$(RESET)"
	./scripts/v3-extension-test.sh --scheme serial --extensions "python" --verbose

v3-ext-test-unit:
	@echo "$(BLUE)Running extension unit tests...$(RESET)"
	cd $(V3_DIR) && cargo test --package sindri-extensions
	@echo "$(GREEN)✓ Extension unit tests passed$(RESET)"

# ============================================================================
# V3 Packer Testing Targets
# ============================================================================

v3-packer-test: v3-packer-test-unit
	@echo "$(GREEN)$(BOLD)✓ Packer tests complete$(RESET)"

v3-packer-test-unit:
	@echo "$(BLUE)Running packer unit tests...$(RESET)"
	cd $(V3_DIR) && cargo test --package sindri-packer
	@echo "$(GREEN)✓ Packer unit tests passed$(RESET)"

v3-packer-validate: v3-build
	@echo "$(BLUE)Validating packer templates...$(RESET)"
	@if command -v packer >/dev/null 2>&1; then \
		echo "Packer found, validating templates..."; \
		$(V3_BINARY) packer validate --cloud aws --dry-run 2>/dev/null || true; \
	else \
		echo "$(YELLOW)Packer not installed, skipping template validation$(RESET)"; \
	fi
	@echo "$(GREEN)✓ Packer validation complete$(RESET)"

v3-inspec-check:
	@echo "$(BLUE)Checking InSpec profile...$(RESET)"
	@if command -v inspec >/dev/null 2>&1; then \
		inspec check $(V3_DIR)/test/integration/sindri/; \
	else \
		echo "$(YELLOW)InSpec not installed. Install: gem install inspec-bin$(RESET)"; \
	fi
	@echo "$(GREEN)✓ InSpec profile check complete$(RESET)"

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
# V3 Clean & CI Targets
# ============================================================================

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

v3-ci: v3-validate v3-test v3-build
	@echo "$(GREEN)$(BOLD)✓ v3 CI pipeline passed$(RESET)"

v3-quality: v3-fmt-check v3-clippy v3-test v3-audit v3-coverage
	@echo "$(GREEN)$(BOLD)✓ v3 quality gate passed (fmt + clippy + test + audit + coverage)$(RESET)"
