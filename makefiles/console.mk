# ============================================================================
# makefiles/console.mk — Console (TypeScript/pnpm) Targets
# ============================================================================

.PHONY: console-install console-build console-dev console-dev-full \
	console-test console-test-coverage \
	console-lint console-typecheck \
	console-fmt console-fmt-check \
	console-audit console-audit-fix \
	console-upgrade console-upgrade-interactive \
	console-infra-up console-infra-down console-infra-reset \
	console-infra-logs console-infra-status \
	console-stack-build console-stack-up console-stack-down \
	console-stack-logs console-stack-status console-stack-rebuild \
	console-db-migrate console-db-migrate-deploy \
	console-db-generate console-db-seed console-db-reset console-db-studio \
	console-clean console-ci

# ============================================================================
# Console Install & Build Targets
# ============================================================================

console-install:
	@echo "$(BLUE)Installing Console Node.js dependencies...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm install
	@echo "$(GREEN)✓ Console dependencies installed$(RESET)"

console-build:
	@echo "$(BLUE)Building Console (API + web)...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm build
	@echo "$(GREEN)✓ Console build complete$(RESET)"

console-dev:
	@echo "$(BLUE)Starting Console in development mode (API + Web, parallel)...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm dev

console-dev-full: console-infra-up
	@echo "$(BLUE)Starting Console dev servers (infra already up)...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm dev

# ============================================================================
# Console Test Targets
# ============================================================================

console-test:
	@echo "$(BLUE)Running Console test suite...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm test
	@echo "$(GREEN)✓ Console tests passed$(RESET)"

console-test-coverage:
	@echo "$(BLUE)Running Console tests with coverage...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm test:coverage
	@echo "$(GREEN)✓ Console test coverage report generated$(RESET)"

# ============================================================================
# Console Lint & Format Targets
# ============================================================================

console-lint:
	@echo "$(BLUE)Linting Console TypeScript code (ESLint)...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm lint
	@echo "$(GREEN)✓ Console lint passed$(RESET)"

console-fmt:
	@echo "$(BLUE)Formatting Console code (Prettier)...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm format
	@echo "$(GREEN)✓ Console code formatted$(RESET)"

console-fmt-check:
	@echo "$(BLUE)Checking Console code formatting (Prettier)...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm format:check
	@echo "$(GREEN)✓ Console format check passed$(RESET)"

console-typecheck:
	@echo "$(BLUE)Running TypeScript type checks on Console...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR)/apps/api && pnpm db:generate
	cd $(CONSOLE_DIR) && pnpm typecheck
	@echo "$(GREEN)✓ Console type checks passed$(RESET)"

# ============================================================================
# Console Audit Targets
# ============================================================================

console-audit:
	@echo "$(BLUE)Running Console pnpm security audit...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm audit
	@echo "$(GREEN)✓ Console security audit complete$(RESET)"

console-audit-fix:
	@echo "$(BLUE)Attempting to auto-fix Console audit issues...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm audit --fix
	@echo "$(GREEN)✓ Console audit fixes applied$(RESET)"

# ============================================================================
# Console Upgrade Targets
# ============================================================================

console-upgrade:
	@echo "$(BLUE)Upgrading Console dependencies to latest...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm update --latest
	@echo "$(GREEN)✓ Console dependencies upgraded$(RESET)"

console-upgrade-interactive:
	@echo "$(BLUE)Interactive Console dependency upgrade...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm update --interactive --latest

# ============================================================================
# Console Infrastructure Targets (postgres + redis only)
# ============================================================================
# Use these during local TypeScript development — infra runs in Docker,
# app code runs natively via `pnpm dev` for fast HMR and debugging.

console-infra-up:
	@echo "$(BLUE)Starting Console infrastructure (postgres + redis)...$(RESET)"
	$(call require_tool,docker,)
	cd $(CONSOLE_DIR) && pnpm infra:up
	@echo "$(GREEN)✓ Infrastructure up: postgres + redis$(RESET)"
	@echo "  DATABASE_URL: postgresql://sindri:sindri@localhost:5432/sindri_console"
	@echo "  REDIS_URL:    redis://localhost:6379"

console-infra-down:
	@echo "$(BLUE)Stopping Console infrastructure...$(RESET)"
	$(call require_tool,docker,)
	cd $(CONSOLE_DIR) && pnpm infra:down
	@echo "$(GREEN)✓ Infrastructure stopped$(RESET)"

console-infra-reset:
	@echo "$(YELLOW)Resetting Console infrastructure (volumes will be destroyed)...$(RESET)"
	$(call require_tool,docker,)
	cd $(CONSOLE_DIR) && pnpm infra:reset
	@echo "$(GREEN)✓ Infrastructure reset: fresh postgres + redis volumes$(RESET)"

console-infra-logs:
	@echo "$(BLUE)Following Console infrastructure logs (Ctrl-C to stop)...$(RESET)"
	$(call require_tool,docker,)
	cd $(CONSOLE_DIR) && pnpm infra:logs

console-infra-status:
	@echo "$(BOLD)$(BLUE)Console Infrastructure Status:$(RESET)"
	$(call require_tool,docker,)
	cd $(CONSOLE_DIR) && docker compose ps postgres redis

# ============================================================================
# Console Full-Stack Targets (all 4 services via Docker Compose)
# ============================================================================
# Use these to run the full console stack (postgres, redis, api, web) in
# Docker — mirrors production topology for integration testing or demos.

console-stack-build:
	@echo "$(BLUE)Building Console Docker images (api + web)...$(RESET)"
	$(call require_tool,docker,)
	cd $(CONSOLE_DIR) && docker compose build
	@echo "$(GREEN)✓ Console Docker images built$(RESET)"

console-stack-up:
	@echo "$(BLUE)Starting full Console stack (postgres + redis + api + web)...$(RESET)"
	$(call require_tool,docker,)
	cd $(CONSOLE_DIR) && docker compose up -d
	@echo "$(GREEN)✓ Console stack up$(RESET)"
	@echo "  Web:  http://localhost:$${WEB_PORT:-5173}"
	@echo "  API:  http://localhost:$${API_PORT:-3001}"

console-stack-down:
	@echo "$(BLUE)Stopping full Console stack...$(RESET)"
	$(call require_tool,docker,)
	cd $(CONSOLE_DIR) && docker compose down
	@echo "$(GREEN)✓ Console stack stopped$(RESET)"

console-stack-logs:
	@echo "$(BLUE)Following full Console stack logs (Ctrl-C to stop)...$(RESET)"
	$(call require_tool,docker,)
	cd $(CONSOLE_DIR) && docker compose logs -f

console-stack-status:
	@echo "$(BOLD)$(BLUE)Console Stack Status:$(RESET)"
	$(call require_tool,docker,)
	cd $(CONSOLE_DIR) && docker compose ps

console-stack-rebuild:
	@echo "$(YELLOW)Rebuilding Console Docker images (no cache) and restarting...$(RESET)"
	$(call require_tool,docker,)
	cd $(CONSOLE_DIR) && docker compose down && docker compose build --no-cache && docker compose up -d
	@echo "$(GREEN)✓ Console stack rebuilt and restarted$(RESET)"

# ============================================================================
# Console Database Targets
# ============================================================================

console-db-migrate:
	@echo "$(BLUE)Running Console database migrations (dev)...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm db:migrate
	@echo "$(GREEN)✓ Console database migrations applied$(RESET)"

console-db-migrate-deploy:
	@echo "$(BLUE)Deploying Console database migrations (production-style)...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm db:migrate:deploy
	@echo "$(GREEN)✓ Console database migrations deployed$(RESET)"

console-db-generate:
	@echo "$(BLUE)Generating Prisma client...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm db:generate
	@echo "$(GREEN)✓ Prisma client generated$(RESET)"

console-db-seed:
	@echo "$(BLUE)Seeding Console database...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm db:seed
	@echo "$(GREEN)✓ Database seeded$(RESET)"

console-db-reset:
	@echo "$(YELLOW)Resetting Console database (all data will be lost)...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm db:reset
	@echo "$(GREEN)✓ Database reset complete$(RESET)"

console-db-studio:
	@echo "$(BLUE)Opening Prisma Studio...$(RESET)"
	$(call require_tool,pnpm,)
	cd $(CONSOLE_DIR) && pnpm db:studio

# ============================================================================
# Console Clean & CI Targets
# ============================================================================

console-clean:
	@echo "$(BLUE)Cleaning Console TypeScript build artifacts...$(RESET)"
	@rm -rf $(CONSOLE_DIR)/apps/api/dist 2>/dev/null || true
	@rm -rf $(CONSOLE_DIR)/apps/web/dist 2>/dev/null || true
	@echo "$(GREEN)✓ Console artifacts cleaned$(RESET)"

console-ci: console-agent-fmt-check console-agent-lint console-agent-test console-agent-build \
            console-fmt-check console-lint console-typecheck console-test console-build
	@echo "$(GREEN)$(BOLD)✓ Console CI pipeline passed (agent + TypeScript)$(RESET)"
