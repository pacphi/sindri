#!/bin/bash
set -eo pipefail
# Note: We don't use 'set -u' (nounset) because SDKMAN scripts have unbound variables

# SDKMAN install script
# Installs SDKMAN and verifies the installation

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
# Note: In some environments (e.g., Fly.io), sdkman-init.sh may return non-zero
# even when successful. We rely on file existence checks instead.
source "$SDKMAN_DIR/bin/sdkman-init.sh" 2>/dev/null || true

# Verify SDKMAN installation by checking critical files
# Note: In non-interactive shells, the sdk function might not be available
# immediately after sourcing. The function will be loaded in interactive shells
# when .bashrc is sourced.
if [[ ! -f "$SDKMAN_DIR/bin/sdkman-init.sh" ]]; then
    print_error "SDKMAN initialization script not found at $SDKMAN_DIR"
    exit 1
fi

# Try to verify sdk function is available (may fail in non-interactive context)
if command -v sdk &>/dev/null; then
    # Function is available - verify version works
    sdk_ver=$(sdk version 2>/dev/null || true)
    sdk_ver_line=$(echo "$sdk_ver" | head -1)
    print_success "SDKMAN installed and initialized: ${sdk_ver_line}"
else
    # Function not available yet - will be loaded in interactive shells via .bashrc
    print_success "SDKMAN installed (initialization deferred to interactive shells)"
fi

print_status "SDKMAN directory: $SDKMAN_DIR"

# Install sdk-validate wrapper for validation
mkdir -p "$HOME/.local/bin"
EXTENSION_DIR="$(dirname "${BASH_SOURCE[0]}")"
cp "$EXTENSION_DIR/sdk-validate" "$HOME/.local/bin/sdk-validate"
chmod +x "$HOME/.local/bin/sdk-validate"
print_status "Installed sdk-validate wrapper to ~/.local/bin/"

print_status "Note: 'sdk' command will be available after sourcing ~/.bashrc or starting a new shell"
