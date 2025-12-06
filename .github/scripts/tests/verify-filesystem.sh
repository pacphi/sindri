#!/usr/bin/env bash
# Phase 6: Verify filesystem paths and permissions
# Usage: verify-filesystem.sh <provider> <target-id> <extensions-list>
# Returns: Sets filesystem verification results in GitHub Actions output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test-helpers.sh"

PROVIDER="${1:?Provider required}"
TARGET_ID="${2:?Target ID required}"
EXTENSIONS="${3:-}"

print_phase "6" "FILESYSTEM CHECKS"

CHECKS_PASSED=0
CHECKS_FAILED=0

# Check mise configuration
if "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
    "test -f ~/.config/mise/config.toml || test -d ~/.config/mise/conf.d" &>/dev/null; then
    echo "✅ mise configuration exists"
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
else
    echo "❌ mise configuration not found"
    CHECKS_FAILED=$((CHECKS_FAILED + 1))
fi

# Check for Docker socket if docker extension is installed
if echo "$EXTENSIONS" | grep -q "docker"; then
    if "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
        "test -S /var/run/docker.sock" &>/dev/null; then
        echo "✅ docker.sock exists"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
    else
        echo "❌ docker.sock not found"
        CHECKS_FAILED=$((CHECKS_FAILED + 1))
    fi
fi

# Check workspace directory
if "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
    "test -d \$WORKSPACE" &>/dev/null; then
    echo "✅ WORKSPACE directory exists"
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
else
    echo "❌ WORKSPACE directory not found"
    CHECKS_FAILED=$((CHECKS_FAILED + 1))
fi

echo ""
echo "----------------------------------------"
echo "Filesystem Checks: $CHECKS_PASSED passed, $CHECKS_FAILED failed"
echo "----------------------------------------"

echo "filesystem-checks-passed=$CHECKS_PASSED" >> "${GITHUB_OUTPUT:-/dev/null}"
echo "filesystem-checks-failed=$CHECKS_FAILED" >> "${GITHUB_OUTPUT:-/dev/null}"

[[ $CHECKS_FAILED -eq 0 ]] && exit 0 || exit 1
