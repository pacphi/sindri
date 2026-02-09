#!/bin/bash
set -euo pipefail

# Bootstrap pnpm via direct npm install
# This works around mise npm backend bug where pnpm itself times out
# After bootstrap, mise can use pnpm for all npm: packages
#
# IMPORTANT: Installs to ~/.npm-global/bin to match mise.toml NPM_CONFIG_PREFIX
# This ensures pnpm is in a known PATH location for validation

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

# Set npm global prefix to match mise.toml configuration
# This ensures pnpm is installed to ~/.npm-global/bin which is in PATH
NPM_GLOBAL_PREFIX="${HOME}/.npm-global"
mkdir -p "$NPM_GLOBAL_PREFIX"

# Install pnpm globally with explicit prefix
# Pinned version for consistency (researched 2026-02-09)
print_status "Installing pnpm@10.29.2 to ${NPM_GLOBAL_PREFIX}..."
"$NODE_BIN" "$NPM_CLI" install -g --prefix "$NPM_GLOBAL_PREFIX" pnpm@10.29.2 || {
    print_error "Failed to install pnpm"
    exit 1
}

# Verify pnpm is available in the expected location
PNPM_BIN="${NPM_GLOBAL_PREFIX}/bin/pnpm"
if [[ -f "$PNPM_BIN" ]]; then
    PNPM_VERSION=$("$PNPM_BIN" --version 2>/dev/null || echo "unknown")
    print_success "pnpm $PNPM_VERSION installed to ${NPM_GLOBAL_PREFIX}/bin"
else
    print_error "pnpm binary not found at $PNPM_BIN after installation"
    exit 1
fi

# Refresh mise shims (for other tools, not pnpm which is in npm-global)
print_status "Refreshing mise shims..."
mise reshim 2>/dev/null || true
hash -r 2>/dev/null || true

print_success "pnpm bootstrap complete"
