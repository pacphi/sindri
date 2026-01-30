#!/bin/bash
set -eo pipefail

# SDKMAN upgrade script
# Updates SDKMAN to the latest version

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Upgrading SDKMAN..."

export SDKMAN_DIR="${SDKMAN_DIR:-$HOME/.sdkman}"

if [[ ! -f "$SDKMAN_DIR/bin/sdkman-init.sh" ]]; then
    print_error "SDKMAN not installed"
    exit 1
fi

# shellcheck source=/dev/null
source "$SDKMAN_DIR/bin/sdkman-init.sh" || {
    print_error "Failed to source SDKMAN init script"
    exit 1
}

# Force update
if sdk selfupdate force; then
    print_success "SDKMAN upgraded: $(sdk version 2>/dev/null | head -1)"
else
    print_error "Failed to upgrade SDKMAN"
    exit 1
fi
