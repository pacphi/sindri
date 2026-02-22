#!/usr/bin/env bash
# ============================================================================
# Makefile Console Target Test Script
# ============================================================================
# Tests all Makefile console-agent-* and console-* targets to verify they
# resolve correctly and execute as expected.
#
# Usage:
#   ./scripts/test-makefile-targets.sh              # Run all tests
#   ./scripts/test-makefile-targets.sh --agent-only # Only agent targets
#   ./scripts/test-makefile-targets.sh --ts-only    # Only TypeScript targets
#   ./scripts/test-makefile-targets.sh --dry-run    # Dry-run only (fast)
#
# Requirements:
#   - Go 1.22+ (for console-agent targets)
#   - pnpm 10+ with node_modules installed (for console-* targets)
#   - make
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# ── Colours ──────────────────────────────────────────────────────────────────
if command -v tput >/dev/null 2>&1 && tput colors >/dev/null 2>&1; then
    BOLD=$(tput bold)
    GREEN=$(tput setaf 2)
    RED=$(tput setaf 1)
    YELLOW=$(tput setaf 3)
    BLUE=$(tput setaf 4)
    RESET=$(tput sgr0)
else
    BOLD='' GREEN='' RED='' YELLOW='' BLUE='' RESET=''
fi

# ── Counters ─────────────────────────────────────────────────────────────────
PASS=0
FAIL=0
SKIP=0

# ── Option parsing ────────────────────────────────────────────────────────────
RUN_AGENT=true
RUN_TS=true
DRY_RUN_ONLY=false

for arg in "$@"; do
    case "$arg" in
        --agent-only) RUN_TS=false ;;
        --ts-only)    RUN_AGENT=false ;;
        --dry-run)    DRY_RUN_ONLY=true ;;
        --help|-h)
            echo "Usage: $0 [--agent-only|--ts-only|--dry-run]"
            exit 0
            ;;
    esac
done

# ── Helpers ───────────────────────────────────────────────────────────────────
pass() {
    echo "${GREEN}✓ PASS${RESET}  $1"
    PASS=$((PASS + 1))
}

fail() {
    echo "${RED}✗ FAIL${RESET}  $1"
    echo "         ${RED}$2${RESET}"
    FAIL=$((FAIL + 1))
}

skip() {
    echo "${YELLOW}⚠ SKIP${RESET}  $1  ($2)"
    SKIP=$((SKIP + 1))
}

# Run make target and report result
check_target() {
    local target="$1"
    local description="$2"
    local extra_args="${3:-}"

    if $DRY_RUN_ONLY; then
        if make --dry-run "$target" $extra_args >/dev/null 2>&1; then
            pass "$target — $description (dry-run)"
        else
            fail "$target — $description (dry-run)" "make --dry-run $target failed"
        fi
        return
    fi

    local output
    if output=$(make -C "$REPO_ROOT" "$target" $extra_args 2>&1); then
        pass "$target — $description"
    else
        fail "$target — $description" "$(echo "$output" | tail -3)"
    fi
}

# Check target resolves (syntax check only — no execution)
check_resolves() {
    local target="$1"
    local description="$2"

    if make -C "$REPO_ROOT" --dry-run "$target" >/dev/null 2>&1; then
        pass "$target resolves — $description"
    else
        fail "$target resolves — $description" "make --dry-run $target failed (check PHONY declaration)"
    fi
}

# ── Preflight ─────────────────────────────────────────────────────────────────
echo ""
echo "${BOLD}${BLUE}╔════════════════════════════════════════════════════════════════════╗${RESET}"
echo "${BOLD}${BLUE}║            Sindri Makefile Console Target Tests                    ║${RESET}"
echo "${BOLD}${BLUE}╚════════════════════════════════════════════════════════════════════╝${RESET}"
echo ""
echo "  Repo root: ${REPO_ROOT}"
echo "  Dry-run only: ${DRY_RUN_ONLY}"
echo "  Test agent targets: ${RUN_AGENT}"
echo "  Test TypeScript targets: ${RUN_TS}"
echo ""

GO_AVAILABLE=false
PNPM_AVAILABLE=false

if command -v go >/dev/null 2>&1; then
    GO_AVAILABLE=true
    echo "  ${GREEN}✓ go$(go version | awk '{print $3}')${RESET}"
else
    echo "  ${YELLOW}⚠ go not found — console-agent tests will be skipped${RESET}"
fi

if command -v pnpm >/dev/null 2>&1; then
    PNPM_AVAILABLE=true
    echo "  ${GREEN}✓ pnpm $(pnpm --version)${RESET}"
else
    echo "  ${YELLOW}⚠ pnpm not found — console TypeScript tests will be skipped${RESET}"
fi

echo ""

# ── Section: Resolve checks (always run) ─────────────────────────────────────
echo "${BOLD}${BLUE}═══ Target Resolution Checks (syntax/PHONY) ════════════════════════${RESET}"
echo ""

# All console-agent targets must resolve
for target in \
    console-agent-build \
    console-agent-build-all \
    console-agent-test \
    console-agent-fmt \
    console-agent-fmt-check \
    console-agent-vet \
    console-agent-lint \
    console-agent-audit \
    console-agent-install \
    console-agent-clean \
    console-agent-ci; do
    check_resolves "$target" "console agent target"
done

# All console TypeScript targets must resolve
for target in \
    console-install \
    console-build \
    console-dev \
    console-test \
    console-test-coverage \
    console-lint \
    console-fmt \
    console-fmt-check \
    console-typecheck \
    console-audit \
    console-audit-fix \
    console-upgrade \
    console-upgrade-interactive \
    console-db-migrate \
    console-db-generate \
    console-clean \
    console-ci; do
    check_resolves "$target" "console TypeScript target"
done

# Aggregate targets must include console
echo ""
echo "${BOLD}${BLUE}═══ Aggregate Target Integration ════════════════════════════════════${RESET}"
echo ""

# ci should depend on console-ci (check Makefile definition, not execution output)
if grep -q "^ci:.*console" "$REPO_ROOT/Makefile"; then
    pass "ci — includes console targets"
else
    fail "ci — includes console targets" "ci target does not depend on any console targets in Makefile"
fi

# clean should depend on console-clean or console-agent-clean
if grep -q "^clean:.*console" "$REPO_ROOT/Makefile"; then
    pass "clean — includes console clean targets"
else
    fail "clean — includes console clean targets" "clean target does not clean console artifacts"
fi

echo ""

# ── Section: Console Agent execution tests ────────────────────────────────────
if $RUN_AGENT; then
    echo "${BOLD}${BLUE}═══ Console Agent (Go) Execution Tests ══════════════════════════════${RESET}"
    echo ""

    if ! $GO_AVAILABLE; then
        skip "console-agent targets" "go not installed"
    else
        # go vet
        check_target "console-agent-vet" "go vet passes"

        # fmt-check
        check_target "console-agent-fmt-check" "Go formatting valid" || true  # formatting issues are informational

        # Unit tests
        check_target "console-agent-test" "unit tests pass"

        # Single-platform build
        check_target "console-agent-build" "single-platform build succeeds"

        # Verify binary exists after build (skip in dry-run — no actual build occurs)
        if ! $DRY_RUN_ONLY; then
            AGENT_BIN="${REPO_ROOT}/v3/console/agent/dist/sindri-agent"
            if [[ -f "${AGENT_BIN}" ]]; then
                pass "console-agent-build — binary artifact exists at dist/sindri-agent"
            else
                fail "console-agent-build — binary artifact exists at dist/sindri-agent" "File not found: ${AGENT_BIN}"
            fi
        fi

        # Build-all (cross-compile)
        check_target "console-agent-build-all" "cross-compile all platforms"

        # Verify cross-compiled binaries exist (skip in dry-run — no actual build occurs)
        if ! $DRY_RUN_ONLY; then
            DIST="${REPO_ROOT}/v3/console/agent/dist"
            for binary in sindri-agent-linux-amd64 sindri-agent-linux-arm64 sindri-agent-darwin-amd64 sindri-agent-darwin-arm64; do
                if [[ -f "${DIST}/${binary}" ]]; then
                    pass "console-agent-build-all — ${binary} exists"
                else
                    fail "console-agent-build-all — ${binary} exists" "File not found: ${DIST}/${binary}"
                fi
            done
        fi

        # Clean
        check_target "console-agent-clean" "clean removes dist/"
        if ! $DRY_RUN_ONLY; then
            if [[ ! -d "${DIST}" ]]; then
                pass "console-agent-clean — dist/ directory removed"
            else
                fail "console-agent-clean — dist/ directory removed" "dist/ still exists after clean"
            fi
        fi

        # Audit (optional tool — just check it runs)
        check_target "console-agent-audit" "vulnerability scan completes"
    fi

    echo ""
fi

# ── Section: Console TypeScript execution tests ───────────────────────────────
if $RUN_TS; then
    echo "${BOLD}${BLUE}═══ Console TypeScript Execution Tests ══════════════════════════════${RESET}"
    echo ""

    CONSOLE_DIR="${REPO_ROOT}/v3/console"
    MODULES_INSTALLED=false
    if [[ -d "${CONSOLE_DIR}/node_modules" ]]; then
        MODULES_INSTALLED=true
    fi

    if ! $PNPM_AVAILABLE; then
        skip "console TypeScript targets" "pnpm not installed"
    elif ! $MODULES_INSTALLED; then
        skip "console-build/test/lint" "node_modules not installed — run: make console-install"
        # Still verify fmt-check resolves (only needs pnpm, not modules)
        check_target "console-fmt-check" "Prettier format check"
    else
        check_target "console-fmt-check" "Prettier format check"
        check_target "console-typecheck" "TypeScript type check"
        check_target "console-lint" "ESLint passes"
        check_target "console-test" "Vitest tests pass"
        check_target "console-build" "production build succeeds"
    fi

    echo ""
fi

# ── Summary ───────────────────────────────────────────────────────────────────
TOTAL=$((PASS + FAIL + SKIP))

echo "${BOLD}${BLUE}═══════════════════════════════════════════════════════════════════════${RESET}"
echo ""
echo "  Results: ${GREEN}${PASS} passed${RESET}  ${RED}${FAIL} failed${RESET}  ${YELLOW}${SKIP} skipped${RESET}  (${TOTAL} total)"
echo ""

if [[ $FAIL -gt 0 ]]; then
    echo "${RED}${BOLD}✗ Some tests failed — review output above${RESET}"
    exit 1
else
    echo "${GREEN}${BOLD}✓ All tests passed${RESET}"
    exit 0
fi
