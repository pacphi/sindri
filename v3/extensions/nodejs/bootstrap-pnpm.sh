#!/bin/bash
set -euo pipefail

# Bootstrap pnpm via direct npm install
# This works around mise npm backend bug where pnpm itself times out
# After bootstrap, mise can use pnpm for all npm: packages

source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Bootstrapping pnpm via npm..."

# Find node installation
NODE_DIR=$(ls -d ~/.local/share/mise/installs/node/*/ 2>/dev/null | head -1)

if [[ -z "$NODE_DIR" ]]; then
    print_error "Node.js not found - install nodejs extension first"
    exit 1
fi

NODE_BIN="${NODE_DIR}bin/node"
NPM_CLI="${NODE_DIR}lib/node_modules/npm/bin/npm-cli.js"

if [[ ! -f "$NPM_CLI" ]]; then
    print_error "npm not found at $NPM_CLI"
    exit 1
fi

# Install pnpm globally via direct npm (bypasses mise)
print_status "Installing pnpm@10 via npm..."
"$NODE_BIN" "$NPM_CLI" install -g pnpm@10 || {
    print_error "Failed to install pnpm"
    exit 1
}

# Verify pnpm is available
PNPM_BIN="${NODE_DIR}bin/pnpm"
if [[ -f "$PNPM_BIN" ]]; then
    PNPM_VERSION=$("$PNPM_BIN" --version 2>/dev/null || echo "unknown")
    print_success "pnpm $PNPM_VERSION installed successfully"
else
    print_error "pnpm binary not found after installation"
    exit 1
fi

# Refresh mise shims to include pnpm
print_status "Refreshing mise shims..."
mise reshim 2>/dev/null || true
hash -r 2>/dev/null || true

print_success "pnpm bootstrap complete"
