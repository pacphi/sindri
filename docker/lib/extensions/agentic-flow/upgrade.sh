#!/bin/bash
set -euo pipefail
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Upgrading Agentic Flow..."

# Check if installed
if ! command -v agentic-flow >/dev/null 2>&1; then
    print_error "Agentic Flow is not installed"
    print_status "Install with: extension-manager install agentic-flow"
    exit 1
fi

old_version=$(agentic-flow --version 2>/dev/null || echo "unknown")
print_status "Current version: $old_version"

# Upgrade via npm
print_status "Upgrading agentic-flow via npm..."
if npm update -g agentic-flow; then
    # Refresh mise shims
    if command -v mise >/dev/null 2>&1; then
        mise reshim 2>/dev/null || true
    fi
    hash -r 2>/dev/null || true

    new_version=$(agentic-flow --version 2>/dev/null || echo "unknown")
    if [[ "$old_version" != "$new_version" ]]; then
        print_success "Agentic Flow upgraded: $old_version -> $new_version"
    else
        print_success "Agentic Flow is already at the latest version: $new_version"
    fi
else
    print_error "Failed to upgrade Agentic Flow"
    exit 1
fi

print_success "Agentic Flow upgrade complete"
