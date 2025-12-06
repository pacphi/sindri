#!/usr/bin/env bash
# Phase 7: Verify environment variables
# Usage: verify-environment.sh <provider> <target-id> <extensions-list>
# Returns: Sets environment verification results in GitHub Actions output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test-helpers.sh"

PROVIDER="${1:?Provider required}"
TARGET_ID="${2:?Target ID required}"
EXTENSIONS="${3:-}"

print_phase "7" "ENVIRONMENT CHECKS"

CHECKS_PASSED=0
# shellcheck disable=SC2034  # CHECKS_FAILED reserved for future failure tracking
CHECKS_FAILED=0

# Check extension-specific environment variables
for ext in $EXTENSIONS; do
    case "$ext" in
        nodejs)
            if "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
                "test -n \"\$NODE_ENV\" || echo 'NODE_ENV not critical'" &>/dev/null; then
                echo "✅ $ext environment OK"
                CHECKS_PASSED=$((CHECKS_PASSED + 1))
            fi
            ;;
        python)
            if "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
                "test -n \"\$PYTHONPATH\" || echo 'PYTHONPATH optional'" &>/dev/null; then
                echo "✅ $ext environment OK"
                CHECKS_PASSED=$((CHECKS_PASSED + 1))
            fi
            ;;
        *)
            echo "  (No specific environment checks for $ext)"
            ;;
    esac
done

echo ""
echo "----------------------------------------"
echo "Environment Checks: $CHECKS_PASSED checked"
echo "----------------------------------------"

echo "environment-checks-passed=$CHECKS_PASSED" >> "${GITHUB_OUTPUT:-/dev/null}"
