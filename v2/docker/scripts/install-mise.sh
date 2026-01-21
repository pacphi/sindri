#!/bin/bash
# Install mise (tool version manager) system-wide
# Binary goes to /usr/local/bin, tools are installed by users to their home directory

set -euo pipefail

# Source common utilities if available
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/../lib/common.sh" ]]; then
    source "$SCRIPT_DIR/../lib/common.sh"
else
    print_status() { echo "==> $1"; }
    print_success() { echo "[OK] $1"; }
    print_error() { echo "[ERROR] $1" >&2; }
fi

MISE_INSTALL_PATH="/usr/local/bin/mise"
PROFILE_SCRIPT="/etc/profile.d/01-mise.sh"

print_status "Installing mise to $MISE_INSTALL_PATH..."

# Install mise binary system-wide
curl -fsSL https://mise.run | MISE_INSTALL_PATH="$MISE_INSTALL_PATH" sh

# Verify installation
if [[ ! -x "$MISE_INSTALL_PATH" ]]; then
    print_error "mise installation failed"
    exit 1
fi

chmod +x "$MISE_INSTALL_PATH"
print_success "mise installed: $($MISE_INSTALL_PATH --version)"

# Create profile.d script for user environment
print_status "Creating mise profile script..."
cat > "$PROFILE_SCRIPT" << 'EOF'
# mise - tool version manager
# Tools are installed to user's home directory (on persistent volume)

if command -v mise >/dev/null 2>&1; then
    export MISE_DATA_DIR="${HOME}/.local/share/mise"
    export MISE_CONFIG_DIR="${HOME}/.config/mise"
    export MISE_CACHE_DIR="${HOME}/.cache/mise"
    export MISE_STATE_DIR="${HOME}/.local/state/mise"

    # Add shims to PATH
    case ":$PATH:" in
        *":${MISE_DATA_DIR}/shims:"*) ;;
        *) export PATH="${MISE_DATA_DIR}/shims:$PATH" ;;
    esac
fi
EOF
chmod 644 "$PROFILE_SCRIPT"

print_success "mise installation complete"
print_status "Users can install tools via: extension-manager install nodejs"
