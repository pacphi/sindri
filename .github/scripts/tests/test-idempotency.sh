#!/usr/bin/env bash
# Phase 5: Test idempotent reinstallation
# Usage: test-idempotency.sh <provider> <profile> <target-id>
# Returns: Sets idempotency test results in GitHub Actions output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test-helpers.sh"

PROVIDER="${1:?Provider required}"
PROFILE="${2:-minimal}"
TARGET_ID="${3:?Target ID required}"

print_phase "5" "IDEMPOTENCY TESTS"

echo "Testing idempotent profile reinstallation..."
echo ""

# Reinstall the same profile
set +e
"$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
    "extension-manager install-profile $PROFILE"
REINSTALL_EXIT=$?
set -e

if [[ $REINSTALL_EXIT -eq 0 ]]; then
    echo "✅ Profile reinstallation succeeded"
    log_result "idempotency_reinstall" "passed"
else
    echo "❌ Profile reinstallation failed (exit code: $REINSTALL_EXIT)"
    log_result "idempotency_reinstall" "failed"
    echo "idempotency-result=failed" >> "${GITHUB_OUTPUT:-/dev/null}"
    exit 1
fi

# Revalidate all extensions
echo ""
echo "Revalidating all extensions..."
if "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
    "extension-manager validate-all" &>/dev/null; then
    echo "✅ All extensions still valid after reinstall"
    log_result "idempotency_revalidate" "passed"
else
    echo "❌ Extension validation failed after reinstall"
    log_result "idempotency_revalidate" "failed"
    echo "idempotency-result=failed" >> "${GITHUB_OUTPUT:-/dev/null}"
    exit 1
fi

echo ""
echo "----------------------------------------"
echo "Idempotency Tests: PASSED"
echo "----------------------------------------"

echo "idempotency-result=passed" >> "${GITHUB_OUTPUT:-/dev/null}"
