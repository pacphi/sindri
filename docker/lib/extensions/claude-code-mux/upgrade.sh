#!/bin/bash
set -euo pipefail

# claude-code-mux upgrade script
# Upgrades CCM to the latest version

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Upgrading Claude Code Mux (CCM)..."

# Get current version
if command -v ccm >/dev/null 2>&1; then
    CURRENT_VERSION=$(ccm --version 2>&1 | grep -oP '\d+\.\d+\.\d+' | head -n1 || echo "unknown")
    print_status "Current version: $CURRENT_VERSION"
else
    print_error "CCM not found. Please install it first."
    exit 1
fi

# Stop CCM if running
if [[ -f "$HOME/.claude-code-mux/ccm.pid" ]]; then
    print_status "Stopping CCM server..."
    ccm-start stop || true
fi

# Re-run installation script (it will fetch latest version)
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
bash "$SCRIPT_DIR/install.sh"

# Get new version
NEW_VERSION=$(ccm --version 2>&1 | grep -oP '\d+\.\d+\.\d+' | head -n1 || echo "unknown")

if [[ "$NEW_VERSION" != "$CURRENT_VERSION" ]]; then
    print_success "CCM upgraded from $CURRENT_VERSION to $NEW_VERSION"
else
    print_status "CCM is already at the latest version: $NEW_VERSION"
fi

print_status "Restart CCM server: ccm-start start"
