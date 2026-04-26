# ============================================================================
# Sindri main Makefile
# ============================================================================
# After the April 2026 reorg, main carries no product source. This Makefile
# is a thin meta-router with repo-wide hygiene and per-branch delegations.
#
# For per-version build/test, switch to the v* branch and use its Makefile:
#   git checkout v3 && make v3-build
# ============================================================================

.PHONY: help lint lint-markdown lint-yaml lint-json links-check format format-check audit-npm

# Repo-wide hygiene config
MARKDOWN_GLOBS := README.md CHANGELOG.md SECURITY.md CONTRIBUTING.md docs/**/*.md
YAML_GLOBS := .github/**/*.yml .github/**/*.yaml *.yml *.yaml

# Colors
BOLD := $(shell tput bold 2>/dev/null || echo '')
GREEN := $(shell tput setaf 2 2>/dev/null || echo '')
RESET := $(shell tput sgr0 2>/dev/null || echo '')

help:
	@echo "$(BOLD)Sindri main Makefile (meta-router)$(RESET)"
	@echo ""
	@echo "main carries no product source. To build/test a version, check it out:"
	@echo ""
	@echo "    git checkout v2 && make v2-ci"
	@echo "    git checkout v3 && make v3-ci"
	@echo "    git checkout v4 && make v4-ci"
	@echo ""
	@echo "$(BOLD)Available targets on main:$(RESET)"
	@echo "  lint           Lint markdown + yaml + json on this branch"
	@echo "  lint-markdown  markdownlint over governance docs"
	@echo "  lint-yaml      yamllint over .github/**/*.yml"
	@echo "  lint-json      Validate JSON files"
	@echo "  links-check    Run lychee on README/docs"
	@echo "  format         Run prettier --write"
	@echo "  format-check   Verify formatting only"
	@echo "  audit-npm      pnpm audit on root package.json (if any)"

# ============================================================================
# Hygiene (governance docs and centralized .github/)
# ============================================================================

lint: lint-markdown lint-yaml lint-json

lint-markdown:
	@command -v markdownlint-cli2 >/dev/null 2>&1 || npm install -g markdownlint-cli2
	@markdownlint-cli2 $(MARKDOWN_GLOBS) || true

lint-yaml:
	@command -v yamllint >/dev/null 2>&1 || pip install yamllint
	@yamllint -c .yamllint.yml .github/

lint-json:
	@for f in $$(find . -maxdepth 4 -name '*.json' -not -path './node_modules/*'); do \
		jq empty "$$f" >/dev/null && echo "✓ $$f" || echo "✗ $$f"; \
	done

links-check:
	@command -v lychee >/dev/null 2>&1 || cargo install lychee
	@lychee --no-progress README.md CHANGELOG.md SECURITY.md CONTRIBUTING.md docs/

format:
	@npx prettier --write '**/*.{json,md,yaml,yml}'

format-check:
	@npx prettier --check '**/*.{json,md,yaml,yml}'

audit-npm:
	@if [ -f package.json ]; then pnpm audit --prod; else echo "no package.json on main"; fi
