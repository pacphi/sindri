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
# Note: In some environments (e.g., Fly.io), sdkman-init.sh may return non-zero
# even when successful. We check if sdk command is available after sourcing.
source "$SDKMAN_DIR/bin/sdkman-init.sh" 2>/dev/null || true

if ! command -v sdk &>/dev/null; then
    print_error "Failed to source SDKMAN - sdk command not available"
    exit 1
fi

# Force update
if sdk selfupdate force; then
    print_success "SDKMAN upgraded: $(sdk version 2>/dev/null | head -1)"
else
    print_error "Failed to upgrade SDKMAN"
    exit 1
fi
