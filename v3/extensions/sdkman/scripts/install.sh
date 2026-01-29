#!/bin/bash
set -eo pipefail
# Note: We don't use 'set -u' (nounset) because SDKMAN scripts have unbound variables

# SDKMAN install script
# Installs SDKMAN and verifies the installation

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing SDKMAN..."

# Set SDKMAN directory
export SDKMAN_DIR="${SDKMAN_DIR:-$HOME/.sdkman}"

# Install SDKMAN
if [[ -d "$SDKMAN_DIR" ]] && [[ -f "$SDKMAN_DIR/bin/sdkman-init.sh" ]]; then
    print_warning "SDKMAN already installed at $SDKMAN_DIR"
    # Update to latest version
    print_status "Updating SDKMAN to latest version..."
    # shellcheck source=/dev/null
    source "$SDKMAN_DIR/bin/sdkman-init.sh" || true
    sdk selfupdate force 2>/dev/null || true
else
    print_status "Downloading and installing SDKMAN..."
    if curl -s "https://get.sdkman.io" | bash; then
        print_success "SDKMAN installed"
    else
        print_error "Failed to install SDKMAN"
        exit 1
    fi
fi

# Source SDKMAN
# shellcheck source=/dev/null
source "$SDKMAN_DIR/bin/sdkman-init.sh" || {
    print_error "Failed to source SDKMAN init script"
    exit 1
}

# Verify SDKMAN is working
if ! command -v sdk &>/dev/null; then
    print_error "SDKMAN 'sdk' command not available after sourcing"
    exit 1
fi

print_success "SDKMAN installed: $(sdk version 2>/dev/null | head -1)"
print_status "SDKMAN directory: $SDKMAN_DIR"
print_status "Note: Run 'source ~/.bashrc' or start a new shell to use 'sdk' command"
