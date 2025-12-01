#!/bin/bash
# Install mise (tool version manager) system-wide
# This script installs mise to /usr/local/bin and configures it for all users
#
# Usage: install-mise.sh [--with-tools]
#   --with-tools: Also install default tools (node@lts, python@3.13)

set -euo pipefail

# Source common utilities if available
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/../lib/common.sh" ]]; then
    source "$SCRIPT_DIR/../lib/common.sh"
else
    # Fallback print functions
    print_status() { echo "==> $1"; }
    print_success() { echo "[OK] $1"; }
    print_error() { echo "[ERROR] $1" >&2; }
fi

# Configuration
MISE_INSTALL_PATH="/usr/local/bin/mise"
PROFILE_SCRIPT="/etc/profile.d/01-mise-activation.sh"
SKEL_BASHRC="/etc/skel/.bashrc"
ALT_HOME="${ALT_HOME:-/alt/home/developer}"

# Parse arguments
WITH_TOOLS=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --with-tools)
            WITH_TOOLS=true
            shift
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

print_status "Installing mise to $MISE_INSTALL_PATH..."

# Install mise system-wide
curl -fsSL https://mise.run | MISE_INSTALL_PATH="$MISE_INSTALL_PATH" sh

# Verify installation
if [[ ! -x "$MISE_INSTALL_PATH" ]]; then
    print_error "mise installation failed"
    exit 1
fi

chmod +x "$MISE_INSTALL_PATH"
print_success "mise installed: $($MISE_INSTALL_PATH --version)"

# Create profile.d script for system-wide activation
print_status "Creating system-wide activation script..."
cat > "$PROFILE_SCRIPT" << 'EOF'
# mise - unified tool version manager
# This script activates mise for all users in login shells

if command -v mise >/dev/null 2>&1; then
    # Set XDG directories relative to HOME (which may be on a volume)
    export MISE_DATA_DIR="${MISE_DATA_DIR:-$HOME/.local/share/mise}"
    export MISE_CONFIG_DIR="${MISE_CONFIG_DIR:-$HOME/.config/mise}"
    export MISE_CACHE_DIR="${MISE_CACHE_DIR:-$HOME/.cache/mise}"
    export MISE_STATE_DIR="${MISE_STATE_DIR:-$HOME/.local/state/mise}"

    # Add shims to PATH if not already present
    if [[ ":$PATH:" != *":$MISE_DATA_DIR/shims:"* ]]; then
        export PATH="$MISE_DATA_DIR/shims:$PATH"
    fi

    # Activate mise (lazy loading for faster shell startup)
    eval "$(mise activate bash 2>/dev/null)" || true
fi
EOF
chmod 644 "$PROFILE_SCRIPT"

# Add mise activation to /etc/skel/.bashrc for new home directories
print_status "Adding mise activation to /etc/skel/.bashrc..."
if ! grep -q "mise activate" "$SKEL_BASHRC" 2>/dev/null; then
    cat >> "$SKEL_BASHRC" << 'EOF'

# mise - unified tool version manager
if command -v mise >/dev/null 2>&1; then
    eval "$(mise activate bash)"
fi
EOF
fi

# Install default tools if requested
if [[ "$WITH_TOOLS" == "true" ]]; then
    print_status "Installing default tools (node@lts, python@3.13)..."
    "$MISE_INSTALL_PATH" use -g node@lts python@3.13
    print_success "Default tools installed"
fi

print_success "mise installation complete"
