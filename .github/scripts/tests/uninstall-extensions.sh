#!/usr/bin/env bash
# Phase 8: Uninstall extensions and verify cleanup
# Usage: uninstall-extensions.sh <provider> <profile> <target-id>
# Returns: Sets uninstall results in GitHub Actions output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test-helpers.sh"

PROVIDER="${1:?Provider required}"
PROFILE="${2:-minimal}"
TARGET_ID="${3:?Target ID required}"

print_phase "8" "UNINSTALL & CLEANUP"

echo "Uninstalling profile: $PROFILE"
echo ""

# Uninstall profile
set +e
"$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
    "extension-manager uninstall-profile $PROFILE --force"
UNINSTALL_EXIT=$?
set -e

if [[ $UNINSTALL_EXIT -eq 0 ]]; then
    echo "✅ Profile uninstalled successfully"
    log_result "uninstall" "passed"
else
    echo "❌ Profile uninstall failed (exit code: $UNINSTALL_EXIT)"
    log_result "uninstall" "failed"
    echo "uninstall-result=failed" >> "${GITHUB_OUTPUT:-/dev/null}"
    exit 1
fi

# Verify cleanup
echo ""
echo "Verifying cleanup..."
if "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
    "extension-manager list | grep -q 'No extensions installed'" &>/dev/null; then
    echo "✅ All extensions removed"
    log_result "cleanup_verified" "passed"
else
    echo "⚠️  Some extensions may remain (check manually)"
    log_result "cleanup_verified" "warning"
fi

echo ""
echo "----------------------------------------"
echo "Uninstall & Cleanup: COMPLETED"
echo "----------------------------------------"

echo "uninstall-result=passed" >> "${GITHUB_OUTPUT:-/dev/null}"
