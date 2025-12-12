#!/bin/bash
set -euo pipefail

# goose install script - Block's open-source AI agent
# Installs the Goose CLI from official release

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing Goose AI agent CLI..."

# Ensure ~/.local/bin exists and is in PATH
mkdir -p "$HOME/.local/bin"
export PATH="$HOME/.local/bin:$PATH"

# Check if already installed
if command_exists goose; then
    current_version=$(goose --version 2>/dev/null || echo "unknown")
    print_warning "Goose already installed: $current_version"
    print_status "To upgrade, run: goose update"
    exit 0
fi

# Install using official Block installer script
# CONFIGURE=false skips interactive setup - user can run 'goose configure' later
print_status "Downloading and installing Goose from official release..."

if curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash 2>&1; then
    # Refresh PATH to find newly installed binary
    hash -r 2>/dev/null || true

    if command_exists goose; then
        version=$(goose --version 2>/dev/null || echo "installed")
        print_success "Goose installed successfully: $version"
        print_status ""
        print_status "Next steps:"
        print_status "  1. Configure your LLM provider: goose configure"
        print_status "  2. Start a session: goose session"
        print_status ""
        print_status "Goose works best with Claude 4 models."
        print_status "Documentation: https://block.github.io/goose/docs/quickstart"
    else
        print_error "Goose installation completed but binary not found in PATH"
        print_status "Expected location: ~/.local/bin/goose"
        exit 1
    fi
else
    print_error "Failed to download or install Goose"
    print_status "Try manual installation: https://block.github.io/goose/docs/getting-started/installation"
    exit 1
fi
