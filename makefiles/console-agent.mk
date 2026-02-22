# ============================================================================
# makefiles/console-agent.mk — Console Agent (Go) Targets
# ============================================================================

.PHONY: console-agent-build console-agent-build-all \
	console-agent-test \
	console-agent-fmt console-agent-fmt-check \
	console-agent-vet console-agent-lint \
	console-agent-audit \
	console-agent-install \
	console-agent-clean \
	console-agent-ci

# ============================================================================
# Console Agent Build Targets
# ============================================================================

console-agent-build:
	@echo "$(BLUE)Building console agent for current platform (static)...$(RESET)"
	$(call require_tool,go,)
	@mkdir -p $(CONSOLE_AGENT_DIR)/dist
	cd $(CONSOLE_AGENT_DIR) && CGO_ENABLED=0 $(GO_BIN) build \
		-ldflags "-s -w" \
		-o dist/sindri-agent \
		./cmd/agent
	@echo "$(GREEN)✓ Agent built: $(CONSOLE_AGENT_DIR)/dist/sindri-agent$(RESET)"

console-agent-build-all:
	@echo "$(BLUE)Cross-compiling console agent for all platforms...$(RESET)"
	$(call require_tool,go,)
	@mkdir -p $(CONSOLE_AGENT_DIR)/dist
	GOOS=linux  GOARCH=amd64 CGO_ENABLED=0 $(GO_BIN) build \
		-C $(CONSOLE_AGENT_DIR) \
		-ldflags "-s -w" \
		-o dist/sindri-agent-linux-amd64 ./cmd/agent
	GOOS=linux  GOARCH=arm64 CGO_ENABLED=0 $(GO_BIN) build \
		-C $(CONSOLE_AGENT_DIR) \
		-ldflags "-s -w" \
		-o dist/sindri-agent-linux-arm64 ./cmd/agent
	GOOS=darwin GOARCH=amd64 CGO_ENABLED=0 $(GO_BIN) build \
		-C $(CONSOLE_AGENT_DIR) \
		-ldflags "-s -w" \
		-o dist/sindri-agent-darwin-amd64 ./cmd/agent
	GOOS=darwin GOARCH=arm64 CGO_ENABLED=0 $(GO_BIN) build \
		-C $(CONSOLE_AGENT_DIR) \
		-ldflags "-s -w" \
		-o dist/sindri-agent-darwin-arm64 ./cmd/agent
	@echo "$(GREEN)✓ Cross-compiled binaries:$(RESET)"
	@ls -lh $(CONSOLE_AGENT_DIR)/dist/

# ============================================================================
# Console Agent Test & Quality Targets
# ============================================================================

console-agent-test:
	@echo "$(BLUE)Running console agent unit tests...$(RESET)"
	$(call require_tool,go,)
	cd $(CONSOLE_AGENT_DIR) && $(GO_BIN) test ./... -count=1 -timeout 120s -race
	@echo "$(GREEN)✓ Console agent tests passed$(RESET)"

console-agent-fmt:
	@echo "$(BLUE)Formatting console agent Go code (gofmt)...$(RESET)"
	$(call require_tool,go,)
	cd $(CONSOLE_AGENT_DIR) && gofmt -w .
	@echo "$(GREEN)✓ Console agent code formatted$(RESET)"

console-agent-fmt-check:
	@echo "$(BLUE)Checking console agent Go formatting...$(RESET)"
	$(call require_tool,go,)
	@UNFORMATTED=$$(cd $(CONSOLE_AGENT_DIR) && gofmt -l .); \
	if [ -n "$$UNFORMATTED" ]; then \
		echo "$(RED)✗ Unformatted files:$(RESET)"; \
		echo "$$UNFORMATTED"; \
		echo "  Run: make console-agent-fmt"; \
		exit 1; \
	fi
	@echo "$(GREEN)✓ Console agent formatting check passed$(RESET)"

console-agent-vet:
	@echo "$(BLUE)Running go vet on console agent...$(RESET)"
	$(call require_tool,go,)
	cd $(CONSOLE_AGENT_DIR) && $(GO_BIN) vet ./...
	@echo "$(GREEN)✓ go vet passed$(RESET)"

console-agent-lint:
	@echo "$(BLUE)Running golangci-lint on console agent...$(RESET)"
	@if ! command -v golangci-lint >/dev/null 2>&1; then \
		echo "$(YELLOW)golangci-lint not installed. Falling back to go vet.$(RESET)"; \
		echo "  Install: curl -sSfL https://raw.githubusercontent.com/golangci/golangci-lint/master/install.sh | sh -s -- -b $$(go env GOPATH)/bin"; \
		cd $(CONSOLE_AGENT_DIR) && $(GO_BIN) vet ./...; \
	else \
		cd $(CONSOLE_AGENT_DIR) && golangci-lint run ./...; \
	fi
	@echo "$(GREEN)✓ Console agent lint passed$(RESET)"

console-agent-audit:
	@echo "$(BLUE)Running vulnerability scan on console agent (govulncheck)...$(RESET)"
	@if ! command -v govulncheck >/dev/null 2>&1; then \
		echo "$(YELLOW)govulncheck not installed.$(RESET)"; \
		echo "  Install: go install golang.org/x/vuln/cmd/govulncheck@latest"; \
		echo "  $(YELLOW)Skipping vulnerability scan.$(RESET)"; \
	else \
		cd $(CONSOLE_AGENT_DIR) && govulncheck ./...; \
		echo "$(GREEN)✓ Console agent vulnerability scan passed$(RESET)"; \
	fi

# ============================================================================
# Console Agent Install & Clean Targets
# ============================================================================

console-agent-install: console-agent-build
	@echo "$(BLUE)Installing sindri-agent to ~/.local/bin...$(RESET)"
	@mkdir -p ~/.local/bin
	@cp $(CONSOLE_AGENT_DIR)/dist/sindri-agent ~/.local/bin/sindri-agent
	@chmod +x ~/.local/bin/sindri-agent
	@echo "$(GREEN)✓ Installed: ~/.local/bin/sindri-agent$(RESET)"
	@echo "  Ensure ~/.local/bin is in your PATH"

console-agent-clean:
	@echo "$(BLUE)Cleaning console agent build artifacts...$(RESET)"
	@rm -rf $(CONSOLE_AGENT_DIR)/dist
	@echo "$(GREEN)✓ Console agent artifacts cleaned$(RESET)"

console-agent-ci: console-agent-vet console-agent-test console-agent-build-all
	@echo "$(GREEN)$(BOLD)✓ Console agent CI passed (vet + test + build-all)$(RESET)"
