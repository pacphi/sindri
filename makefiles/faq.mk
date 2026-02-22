# ============================================================================
# makefiles/faq.mk — FAQ Static Site Targets
# ============================================================================
# Pipeline: validate → build → [preview | docker-build → docker-run] → deploy
#
# Source layout:
#   docs/faq/src/index.html        HTML template
#   docs/faq/src/faq.js            Interactive UI (667 lines)
#   docs/faq/src/v3-faq-data.json  Question data (169 questions, schema v3)
#
# Build output:
#   docs/faq/index.html            Self-contained file (JS + data embedded)
#
# Deployment:
#   Target: https://sindri-faq.fly.dev  (nginx:alpine on Fly.io)
# ============================================================================

FAQ_DIR         := $(PROJECT_ROOT)/docs/faq
FAQ_IMAGE       := sindri-faq:local
FAQ_CONTAINER   := sindri-faq
FAQ_LOCAL_PORT  ?= 8080

.PHONY: faq-validate faq-build faq-preview \
	faq-docker-build faq-docker-run faq-docker-stop \
	faq-deploy faq-status faq-logs faq-open \
	faq-ci

# ============================================================================
# FAQ Validation
# ============================================================================

faq-validate:
	@echo "$(BLUE)Validating FAQ data schema...$(RESET)"
	$(call require_tool,jq,)
	@chmod +x $(FAQ_DIR)/validate-faq.sh
	@cd $(PROJECT_ROOT) && $(FAQ_DIR)/validate-faq.sh
	@echo "$(GREEN)✓ FAQ data validation passed$(RESET)"

# ============================================================================
# FAQ Build
# ============================================================================

faq-build:
	@echo "$(BLUE)Building FAQ self-contained index.html...$(RESET)"
	$(call require_tool,node,)
	@cd $(PROJECT_ROOT) && node $(FAQ_DIR)/build.mjs
	@echo "$(GREEN)✓ FAQ built: $(FAQ_DIR)/index.html$(RESET)"

# ============================================================================
# FAQ Local Preview
# ============================================================================

faq-preview: faq-build
	@echo "$(BLUE)Opening FAQ in browser...$(RESET)"
	@if [ ! -f "$(FAQ_DIR)/index.html" ]; then \
		echo "$(RED)✗ index.html not found — run: make faq-build$(RESET)"; \
		exit 1; \
	fi
	@case "$(OS)" in \
		macos)  open "$(FAQ_DIR)/index.html" ;; \
		linux)  xdg-open "$(FAQ_DIR)/index.html" 2>/dev/null || \
		        echo "$(YELLOW)Open manually: $(FAQ_DIR)/index.html$(RESET)" ;; \
		*)      echo "$(YELLOW)Open manually: $(FAQ_DIR)/index.html$(RESET)" ;; \
	esac

# ============================================================================
# FAQ Docker Targets
# ============================================================================

faq-docker-build: faq-build
	@echo "$(BLUE)Building FAQ Docker image ($(FAQ_IMAGE))...$(RESET)"
	$(call require_tool,docker,)
	docker build -t $(FAQ_IMAGE) -f $(FAQ_DIR)/Dockerfile $(FAQ_DIR)
	@echo "$(GREEN)✓ FAQ Docker image built: $(FAQ_IMAGE)$(RESET)"

faq-docker-run: faq-docker-build
	@echo "$(BLUE)Running FAQ locally on http://localhost:$(FAQ_LOCAL_PORT)...$(RESET)"
	$(call require_tool,docker,)
	@docker rm -f $(FAQ_CONTAINER) 2>/dev/null || true
	docker run -d --name $(FAQ_CONTAINER) -p $(FAQ_LOCAL_PORT):80 $(FAQ_IMAGE)
	@echo "$(GREEN)✓ FAQ running: http://localhost:$(FAQ_LOCAL_PORT)$(RESET)"
	@echo "  Stop with: make faq-docker-stop"
	@case "$(OS)" in \
		macos) sleep 1 && open "http://localhost:$(FAQ_LOCAL_PORT)" ;; \
		linux) sleep 1 && xdg-open "http://localhost:$(FAQ_LOCAL_PORT)" 2>/dev/null || true ;; \
	esac

faq-docker-stop:
	@echo "$(BLUE)Stopping FAQ container...$(RESET)"
	$(call require_tool,docker,)
	@docker rm -f $(FAQ_CONTAINER) 2>/dev/null || true
	@echo "$(GREEN)✓ FAQ container stopped$(RESET)"

# ============================================================================
# FAQ Deployment (Fly.io)
# ============================================================================

faq-deploy: faq-validate faq-build
	@echo "$(BLUE)Deploying FAQ to Fly.io...$(RESET)"
	$(call check_optional_tool,flyctl,)
	@if ! command -v flyctl >/dev/null 2>&1; then \
		echo "$(RED)✗ flyctl is required for deployment$(RESET)"; \
		exit 1; \
	fi
	cd $(FAQ_DIR) && flyctl deploy
	@echo "$(GREEN)✓ FAQ deployed: https://sindri-faq.fly.dev$(RESET)"

faq-status:
	@echo "$(BOLD)$(BLUE)FAQ Fly.io App Status:$(RESET)"
	$(call check_optional_tool,flyctl,)
	@if command -v flyctl >/dev/null 2>&1; then \
		cd $(FAQ_DIR) && flyctl status; \
	else \
		echo "$(YELLOW)flyctl not installed — cannot check status$(RESET)"; \
	fi

faq-logs:
	@echo "$(BLUE)Following FAQ Fly.io logs (Ctrl-C to stop)...$(RESET)"
	$(call check_optional_tool,flyctl,)
	@if command -v flyctl >/dev/null 2>&1; then \
		cd $(FAQ_DIR) && flyctl logs; \
	else \
		echo "$(YELLOW)flyctl not installed — cannot stream logs$(RESET)"; \
	fi

faq-open:
	@echo "$(BLUE)Opening deployed FAQ...$(RESET)"
	@case "$(OS)" in \
		macos) open "https://sindri-faq.fly.dev" ;; \
		linux) xdg-open "https://sindri-faq.fly.dev" 2>/dev/null || \
		       echo "$(GREEN)→ https://sindri-faq.fly.dev$(RESET)" ;; \
		*)     echo "$(GREEN)→ https://sindri-faq.fly.dev$(RESET)" ;; \
	esac

# ============================================================================
# FAQ CI Target
# ============================================================================

faq-ci: faq-validate faq-build
	@echo "$(GREEN)$(BOLD)✓ FAQ CI passed (validate + build)$(RESET)"
