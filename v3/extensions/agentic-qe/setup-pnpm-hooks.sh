#!/usr/bin/env bash
# setup-pnpm-hooks.sh - Configure pnpm to handle agentic-qe dependency issues
#
# agentic-qe@3.2.x requires lodash@^4.17.23 which doesn't exist (latest is 4.17.21)
# This script sets up pnpm hooks to remap the invalid version requirement.

set -euo pipefail

PNPMFILE="$HOME/.pnpmfile.cjs"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_PNPMFILE="$SCRIPT_DIR/pnpmfile.cjs"

echo "[agentic-qe] Checking pnpm configuration for dependency resolution..."

# Check if pnpm is available
if ! command -v pnpm &> /dev/null; then
    echo "[agentic-qe] pnpm not found, skipping hooks setup (will use npm fallback)"
    exit 0
fi

# Check if global pnpmfile is already configured
CURRENT_PNPMFILE=$(pnpm config get global-pnpmfile 2>/dev/null || echo "")

if [[ "$CURRENT_PNPMFILE" == "undefined" ]] || [[ -z "$CURRENT_PNPMFILE" ]]; then
    # No global pnpmfile configured
    if [[ -f "$PNPMFILE" ]]; then
        # File exists but not configured - check if it has lodash override
        if grep -q "lodash" "$PNPMFILE" 2>/dev/null; then
            echo "[agentic-qe] Existing pnpmfile has lodash override, configuring pnpm to use it..."
        else
            echo "[agentic-qe] Existing pnpmfile missing lodash override, backing up and replacing..."
            cp "$PNPMFILE" "$PNPMFILE.bak.$(date +%s)"
            cp "$SOURCE_PNPMFILE" "$PNPMFILE"
        fi
    else
        echo "[agentic-qe] Creating global pnpmfile with dependency overrides..."
        cp "$SOURCE_PNPMFILE" "$PNPMFILE"
    fi

    # Configure pnpm to use the global pnpmfile
    pnpm config set global-pnpmfile "$PNPMFILE" --global
    echo "[agentic-qe] Configured pnpm global-pnpmfile: $PNPMFILE"
else
    # Already configured - verify it handles lodash
    if [[ -f "$CURRENT_PNPMFILE" ]] && grep -q "lodash" "$CURRENT_PNPMFILE" 2>/dev/null; then
        echo "[agentic-qe] pnpm hooks already configured with lodash override"
    else
        echo "[agentic-qe] WARNING: global-pnpmfile configured but may not handle lodash"
        echo "[agentic-qe] If installation fails, manually add lodash override to: $CURRENT_PNPMFILE"
    fi
fi

echo "[agentic-qe] pnpm hooks setup complete"
