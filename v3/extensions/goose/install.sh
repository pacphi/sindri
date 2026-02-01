#!/bin/bash
set -euo pipefail

# goose install script - Block's open-source AI agent
# Installs the Goose CLI from official release

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing Goose AI agent CLI..."

# Determine if we have root access (for installing dependencies)
if [[ $(id -u) -eq 0 ]]; then
  SUDO=""
elif sudo -n true 2>/dev/null; then
  SUDO="sudo"
else
  SUDO=""
fi

# Install required X11 dependencies (goose uses libxcb for clipboard/GUI features)
print_status "Checking for required system libraries..."
if ! ldconfig -p 2>/dev/null | grep -q "libxcb.so.1"; then
  print_status "Installing X11 libraries required by Goose..."
  if [[ -n "$SUDO" ]] || [[ $(id -u) -eq 0 ]]; then
    $SUDO apt-get update -qq 2>/dev/null || true
    $SUDO apt-get install -y -qq libxcb1 libxcb-render0 libxcb-shape0 libxcb-xfixes0 2>/dev/null || {
      print_warning "Could not install X11 libraries - goose may not work properly"
      print_warning "Run as root: apt-get install -y libxcb1 libxcb-render0 libxcb-shape0 libxcb-xfixes0"
    }
  else
    print_warning "Cannot install X11 libraries without root access"
    print_warning "Ask admin to run: apt-get install -y libxcb1 libxcb-render0 libxcb-shape0 libxcb-xfixes0"
  fi
fi

# Ensure ~/.local/bin exists and is in PATH
mkdir -p "$HOME/.local/bin"
export PATH="$HOME/.local/bin:$PATH"

# Check if already installed AND working
if command_exists goose; then
    # Verify it actually runs (check for missing libraries)
    if goose --version >/dev/null 2>&1; then
        current_version=$(goose --version 2>/dev/null)
        print_warning "Goose already installed: $current_version"
        print_status "To upgrade, run: goose update"
        exit 0
    else
        print_warning "Goose binary exists but is broken (missing libraries?)"
        print_status "Removing broken installation and reinstalling..."
        rm -f "$HOME/.local/bin/goose" 2>/dev/null || true
    fi
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
