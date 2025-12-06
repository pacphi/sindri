#!/usr/bin/env bash
# Phase 2: Discover extensions in profile
# Usage: discover-extensions.sh <provider> <profile> <target-id>
# Returns: Sets extensions list and count in GitHub Actions output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test-helpers.sh"

PROVIDER="${1:?Provider required}"
PROFILE="${2:-minimal}"
TARGET_ID="${3:?Target ID required}"

print_phase "2" "EXTENSION DISCOVERY"

# Discover extensions from profile
EXTENSIONS=$("$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
    "yq '.profiles.${PROFILE}.extensions[]' /docker/lib/profiles.yaml" 2>/dev/null || echo "")

EXT_COUNT=$(echo "$EXTENSIONS" | wc -w | tr -d ' ')

echo "Extensions in profile '$PROFILE': $EXT_COUNT"
if [[ -n "$EXTENSIONS" ]]; then
    echo ""
    for ext in $EXTENSIONS; do
        echo "  - $ext"
    done
fi

# Set GitHub Actions output
echo "extensions=$EXTENSIONS" >> "${GITHUB_OUTPUT:-/dev/null}"
echo "extension-count=$EXT_COUNT" >> "${GITHUB_OUTPUT:-/dev/null}"

log_result "discovery" "completed" "found_${EXT_COUNT}_extensions"
