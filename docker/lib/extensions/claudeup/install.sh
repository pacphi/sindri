#!/bin/bash
set -euo pipefail
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing Claudeup TUI..."

# Check if already installed
if command -v claudeup >/dev/null 2>&1; then
    claudeup_version=$(claudeup --version 2>/dev/null || echo "installed")
    print_warning "Claudeup already installed: $claudeup_version"
    print_status "Skipping installation (remove first to reinstall)"
    exit 0
fi

# Verify Node.js prerequisites
if ! command -v node >/dev/null 2>&1; then
    print_error "Node.js is required but not installed"
    print_status "Install with: extension-manager install nodejs"
    exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
    print_error "npm is required but not found"
    print_status "Install with: extension-manager install nodejs"
    exit 1
fi

# Check Node.js version (requires 18+)
node_version=$(node --version 2>/dev/null | sed 's/v//')
required_major=18
current_major=$(echo "$node_version" | cut -d. -f1)

if (( current_major < required_major )); then
    print_error "Node.js $required_major+ required (found: v$node_version)"
    print_status "Upgrade Node.js with: mise use node@latest"
    exit 1
fi

print_success "Node.js $node_version meets requirements"

# Install claudeup globally via npm
print_status "Installing claudeup via npm..."
if npm install -g claudeup; then
    print_success "Claudeup installed successfully"

    # Refresh mise shims so new command is discoverable
    if command -v mise >/dev/null 2>&1; then
        mise reshim 2>/dev/null || true
    fi
    hash -r 2>/dev/null || true

    # Verify installation
    if command -v claudeup >/dev/null 2>&1; then
        version=$(claudeup --version 2>/dev/null || echo "version check failed")
        print_success "Claudeup TUI installed: $version"
    else
        print_warning "Claudeup installed but command not found in PATH"
        print_status "You may need to reload your shell or run: mise reshim"
    fi
else
    print_error "Failed to install Claudeup"
    exit 1
fi

# Create config directory
mkdir -p "$HOME/.config/claudeup"
print_success "Created config directory: ~/.config/claudeup"

print_success "Claudeup installation complete"
print_status "Get started with: claudeup"
print_status "Features: MCP server setup, plugin management, status line config"
