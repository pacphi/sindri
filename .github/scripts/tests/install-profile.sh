#!/usr/bin/env bash
# Phase 1: Install extension profile
# Usage: install-profile.sh <provider> <profile> <target-id>
# Returns: Sets PROFILE_INSTALL_RESULT and PROFILE_INSTALL_STATUS in GitHub Actions output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test-helpers.sh"
source "$SCRIPT_DIR/../providers/run-on-provider.sh"

PROVIDER="${1:?Provider required}"
PROFILE="${2:-minimal}"
TARGET_ID="${3:?Target ID required}"

print_phase "1" "PROFILE INSTALLATION"
echo "Profile: $PROFILE"
echo "Provider: $PROVIDER"
echo ""

# Install profile with timeout
set +e
"$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" "extension-manager install-profile $PROFILE"
PROFILE_EXIT=$?
set -e

# Report results
if [[ $PROFILE_EXIT -eq 0 ]]; then
    echo ""
    echo "✅ Profile '$PROFILE' installed successfully"
    log_result "profile_install" "passed"

    # Set GitHub Actions output
    echo "profile-install-result=passed" >> "${GITHUB_OUTPUT:-/dev/null}"
    echo "profile-install-status=success" >> "${GITHUB_OUTPUT:-/dev/null}"
    exit 0
else
    echo ""
    echo "❌ Profile '$PROFILE' installation failed (exit code: $PROFILE_EXIT)"
    log_result "profile_install" "failed"

    # Set GitHub Actions output
    echo "profile-install-result=failed" >> "${GITHUB_OUTPUT:-/dev/null}"
    echo "profile-install-status=failure" >> "${GITHUB_OUTPUT:-/dev/null}"
    exit 1
fi
