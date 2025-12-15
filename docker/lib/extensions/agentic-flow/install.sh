#!/bin/bash
set -euo pipefail
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing Agentic Flow..."

# Check if already installed
if command -v agentic-flow >/dev/null 2>&1; then
    af_version=$(agentic-flow --version 2>/dev/null || echo "installed")
    print_warning "Agentic Flow already installed: $af_version"
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

# Install agentic-flow globally via npm
print_status "Installing agentic-flow via npm..."
if npm install -g agentic-flow; then
    print_success "Agentic Flow installed successfully"

    # Get the npm global bin directory and node version for reshimming
    # Note: 'npm bin -g' was deprecated in npm 9+, use 'npm config get prefix' instead
    NPM_PREFIX=$(npm config get prefix 2>/dev/null || echo "")
    NPM_BIN="${NPM_PREFIX}/bin"
    NODE_VERSION=$(node --version 2>/dev/null | sed 's/v//')

    # Refresh mise shims with explicit node version context
    if command -v mise >/dev/null 2>&1; then
        print_status "Reshimming mise $NODE_VERSION..."
        mise reshim 2>/dev/null || true
        # Also try reshimming with explicit tool
        mise reshim node 2>/dev/null || true
    fi

    # Clear shell's command hash table
    hash -r 2>/dev/null || true

    # Add npm global bin to PATH for immediate availability
    if [[ -n "$NPM_BIN" && -d "$NPM_BIN" ]]; then
        export PATH="$NPM_BIN:$PATH"
    fi

    # Also check mise shims directory
    MISE_SHIMS="${HOME}/.local/share/mise/shims"
    if [[ -d "$MISE_SHIMS" ]]; then
        export PATH="$MISE_SHIMS:$PATH"
    fi

    # Ensure npm global bin is in PATH permanently
    if [[ -n "$NPM_BIN" && -d "$NPM_BIN" ]]; then
        if ! grep -q "NPM_GLOBAL_BIN" "$HOME/.bashrc" 2>/dev/null; then
            echo "" >> "$HOME/.bashrc"
            echo "# npm global bin (added by agentic-flow extension)" >> "$HOME/.bashrc"
            echo "export NPM_GLOBAL_BIN=\"\$HOME/.npm-global/bin\"" >> "$HOME/.bashrc"
            echo 'export PATH="$NPM_GLOBAL_BIN:$PATH"' >> "$HOME/.bashrc"
            print_status "Added npm global bin to ~/.bashrc"
        fi
    fi

    # Verify installation - check multiple locations
    if command -v agentic-flow >/dev/null 2>&1; then
        version=$(agentic-flow --version 2>/dev/null || echo "version check failed")
        print_success "Agentic Flow installed: $version"
    elif [[ -n "$NPM_BIN" && -x "$NPM_BIN/agentic-flow" ]]; then
        print_success "Agentic Flow installed at: $NPM_BIN/agentic-flow"
        print_status "Restart your shell or run: source ~/.bashrc"
    else
        print_warning "Agentic Flow installed but command not found in PATH"
        print_status "You may need to reload your shell or run: source ~/.bashrc"
    fi
else
    print_error "Failed to install Agentic Flow"
    exit 1
fi

print_success "Agentic Flow installation complete"
print_status "Get started with: agentic-flow --help"
print_status "Use agents: af-coder, af-reviewer, af-researcher"
