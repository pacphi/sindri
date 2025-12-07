#!/bin/bash
set -euo pipefail
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing OpenSkills CLI..."

# Check if already installed
if command -v openskills >/dev/null 2>&1; then
    openskills_version=$(openskills --version 2>/dev/null || echo "installed")
    print_warning "OpenSkills already installed: $openskills_version"
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

# Check Node.js version (requires 20.6+)
node_version=$(node --version 2>/dev/null | sed 's/v//')
required_major=20
required_minor=6
current_major=$(echo "$node_version" | cut -d. -f1)
current_minor=$(echo "$node_version" | cut -d. -f2)

if (( current_major < required_major )) || \
   (( current_major == required_major && current_minor < required_minor )); then
    print_error "Node.js $required_major.$required_minor+ required (found: v$node_version)"
    print_status "Upgrade Node.js with: mise use node@latest"
    exit 1
fi

print_success "Node.js $node_version meets requirements"

# Install openskills globally via npm
print_status "Installing openskills via npm..."
if npm install -g openskills; then
    print_success "OpenSkills installed successfully"

    # Refresh mise shims so new command is discoverable
    if command -v mise >/dev/null 2>&1; then
        mise reshim 2>/dev/null || true
    fi
    hash -r 2>/dev/null || true

    # Verify installation
    if command -v openskills >/dev/null 2>&1; then
        version=$(openskills --version 2>/dev/null || echo "version check failed")
        print_success "OpenSkills CLI installed: $version"
    else
        print_warning "OpenSkills installed but command not found in PATH"
        print_status "You may need to reload your shell or run: mise reshim"
    fi
else
    print_error "Failed to install OpenSkills"
    exit 1
fi

# Create openskills config directory
mkdir -p "$HOME/.openskills"
print_success "Created config directory: ~/.openskills"

# Add to PATH if needed
bin_path="$HOME/.local/bin"
if [[ -d "$bin_path" ]] && ! grep -q "$bin_path" "$HOME/.bashrc" 2>/dev/null; then
    {
        echo ""
        echo "# openskills - binary path"
        echo "export PATH=\"$bin_path:\$PATH\""
    } >> "$HOME/.bashrc"
    print_success "Added ~/.local/bin to PATH"
fi

print_success "OpenSkills installation complete"
print_status "Get started with: openskills --help"
