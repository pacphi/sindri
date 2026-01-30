#!/bin/bash
set -euo pipefail

# Haskell removal script - Removes ghcup and all Haskell tools

echo "Removing Haskell development environment..."

# Remove ghcup and all managed tools
if command -v ghcup &>/dev/null; then
    echo "Running ghcup nuke to remove all Haskell tools..."
    ghcup nuke || echo "ghcup nuke failed, removing manually..."
fi

# Remove ghcup directories
rm -rf "${HOME}/.ghcup"
rm -rf "${HOME}/.cabal"
rm -rf "${HOME}/.stack"

# Clean up PATH entries from profile (best effort)
if [[ -f "${HOME}/.profile" ]]; then
    # Create backup
    cp "${HOME}/.profile" "${HOME}/.profile.bak"
    # Remove Haskell-related lines
    grep -v -E '(ghcup|cabal|GHCUP|CABAL_DIR|STACK_ROOT|\.ghcup|\.cabal)' "${HOME}/.profile" > "${HOME}/.profile.tmp" || true
    mv "${HOME}/.profile.tmp" "${HOME}/.profile"
fi

echo "Haskell development environment removed successfully"
