#!/bin/bash
set -euo pipefail
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Upgrading Claudeup TUI..."

# Check if installed
if ! command -v claudeup >/dev/null 2>&1; then
    print_error "Claudeup is not installed"
    print_status "Install with: extension-manager install claudeup"
    exit 1
fi

old_version=$(claudeup --version 2>/dev/null || echo "unknown")
print_status "Current version: $old_version"

# Upgrade via npm
print_status "Upgrading claudeup via npm..."
if npm update -g claudeup; then
    new_version=$(claudeup --version 2>/dev/null || echo "unknown")
    if [[ "$old_version" != "$new_version" ]]; then
        print_success "Claudeup upgraded: $old_version -> $new_version"
    else
        print_success "Claudeup is already at the latest version: $new_version"
    fi
else
    print_error "Failed to upgrade Claudeup"
    exit 1
fi

print_success "Claudeup upgrade complete"
