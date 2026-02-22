# ============================================================================
# makefiles/v2.mk — V2 (Bash/Docker) Targets
# ============================================================================

.PHONY: v2-validate v2-validate-yaml v2-validate-shell v2-validate-markdown \
	v2-lint v2-lint-yaml v2-lint-shell v2-lint-markdown \
	v2-test v2-test-unit v2-test-extensions \
	v2-build v2-build-latest v2-build-nocache \
	v2-docker-build v2-docker-build-latest v2-docker-build-nocache \
	v2-config-init v2-config-validate \
	v2-extensions-list v2-extensions-install v2-extensions-search \
	v2-profiles-list v2-connect \
	v2-deploy v2-deploy-docker v2-deploy-fly v2-deploy-devpod \
	v2-clean v2-ci

# ============================================================================
# V2 Validation Targets
# ============================================================================

v2-validate: v2-validate-yaml v2-validate-shell v2-validate-markdown
	@echo "$(GREEN)$(BOLD)✓ All v2 validation passed$(RESET)"

v2-validate-yaml:
	@echo "$(BLUE)Validating v2 YAML files...$(RESET)"
	pnpm v2:validate:yaml
	@echo "$(GREEN)✓ v2 YAML validation passed$(RESET)"

v2-validate-shell:
	@echo "$(BLUE)Validating v2 shell scripts with shellcheck...$(RESET)"
	pnpm v2:validate:shell
	@echo "$(GREEN)✓ v2 shell validation passed$(RESET)"

v2-validate-markdown:
	@echo "$(BLUE)Validating v2 markdown...$(RESET)"
	pnpm v2:validate:markdown
	@echo "$(GREEN)✓ v2 markdown validation passed$(RESET)"

# ============================================================================
# V2 Linting Targets
# ============================================================================

v2-lint: v2-lint-yaml v2-lint-shell v2-lint-markdown
	@echo "$(GREEN)$(BOLD)✓ All v2 linting passed$(RESET)"

v2-lint-yaml:
	@echo "$(BLUE)Linting v2 YAML files (strict mode)...$(RESET)"
	pnpm v2:lint:yaml
	@echo "$(GREEN)✓ v2 YAML linting passed$(RESET)"

v2-lint-shell:
	@echo "$(BLUE)Linting v2 shell scripts (strict mode)...$(RESET)"
	pnpm v2:lint:shell
	@echo "$(GREEN)✓ v2 shell linting passed$(RESET)"

v2-lint-markdown:
	@echo "$(BLUE)Linting v2 markdown...$(RESET)"
	pnpm v2:lint:markdown
	@echo "$(GREEN)✓ v2 markdown linting passed$(RESET)"

# ============================================================================
# V2 Testing Targets
# ============================================================================

v2-test: v2-test-unit
	@echo "$(GREEN)$(BOLD)✓ All v2 tests passed$(RESET)"

v2-test-unit:
	@echo "$(BLUE)Running v2 unit tests...$(RESET)"
	pnpm v2:test:unit
	@echo "$(GREEN)✓ v2 unit tests passed$(RESET)"

v2-test-extensions:
	@echo "$(BLUE)Testing v2 extensions...$(RESET)"
	pnpm v2:test:extensions
	@echo "$(GREEN)✓ v2 extension tests passed$(RESET)"

# ============================================================================
# V2 Build Targets
# ============================================================================

# Aliases for backwards compatibility
v2-build: v2-docker-build

v2-build-latest: v2-docker-build-latest

v2-build-nocache: v2-docker-build-nocache

# ============================================================================
# V2 Docker Build Targets
# ============================================================================

v2-docker-build:
	@echo "$(BLUE)Building v2 Docker image (local tag)...$(RESET)"
	pnpm v2:build
	@echo "$(GREEN)✓ v2 Docker build complete: sindri:v2-local$(RESET)"

v2-docker-build-latest:
	@echo "$(BLUE)Building v2 Docker image (latest tag)...$(RESET)"
	pnpm v2:build:latest
	@echo "$(GREEN)✓ v2 Docker build complete: sindri:v2-latest$(RESET)"

v2-docker-build-nocache:
	@echo "$(BLUE)Building v2 Docker image (no cache)...$(RESET)"
	@echo "$(YELLOW)Warning: This will take longer than normal$(RESET)"
	pnpm v2:build:nocache
	@echo "$(GREEN)✓ v2 Docker build complete: sindri:v2-latest$(RESET)"

# ============================================================================
# V2 Configuration Targets
# ============================================================================

v2-config-init:
	@echo "$(BLUE)Initializing sindri.yaml...$(RESET)"
	pnpm v2:config:init
	@echo "$(GREEN)✓ sindri.yaml initialized$(RESET)"

v2-config-validate:
	@echo "$(BLUE)Validating sindri.yaml...$(RESET)"
	pnpm v2:config:validate
	@echo "$(GREEN)✓ sindri.yaml validation passed$(RESET)"

# ============================================================================
# V2 Extension Management Targets
# ============================================================================

v2-extensions-list:
	@echo "$(BLUE)Listing available extensions...$(RESET)"
	pnpm v2:extensions:list

v2-extensions-install:
	@echo "$(BLUE)Installing extensions interactively...$(RESET)"
	pnpm v2:extensions:install

v2-extensions-search:
	@echo "$(BLUE)Searching for extensions...$(RESET)"
	pnpm v2:extensions:search

v2-profiles-list:
	@echo "$(BLUE)Listing extension profiles...$(RESET)"
	pnpm v2:profiles:list

# ============================================================================
# V2 Deployment Targets
# ============================================================================

v2-deploy:
	@echo "$(BLUE)Deploying v2...$(RESET)"
	pnpm v2:deploy

v2-deploy-docker:
	@echo "$(BLUE)Deploying v2 to Docker...$(RESET)"
	pnpm v2:deploy:docker

v2-deploy-fly:
	@echo "$(BLUE)Deploying v2 to Fly.io...$(RESET)"
	pnpm v2:deploy:fly

v2-deploy-devpod:
	@echo "$(BLUE)Deploying v2 to DevPod...$(RESET)"
	pnpm v2:deploy:devpod

v2-connect:
	@echo "$(BLUE)Connecting to v2 deployment...$(RESET)"
	pnpm v2:connect

# ============================================================================
# V2 Clean & CI Targets
# ============================================================================

v2-clean:
	@echo "$(BLUE)Cleaning v2 Docker images...$(RESET)"
	@docker images | grep sindri | awk '{print $$3}' | xargs -r docker rmi -f 2>/dev/null || true
	@echo "$(GREEN)✓ v2 artifacts cleaned$(RESET)"

v2-ci: v2-validate v2-build
	@echo "$(GREEN)$(BOLD)✓ v2 CI pipeline passed$(RESET)"
