#!/bin/bash
set -euo pipefail

# ollama install script - Installs Ollama LLM runtime
# Uses official installer with extended timeout for large binary download

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing Ollama..."

# Check if running in CI mode - skip large downloads
if [[ "${CI:-}" == "true" ]] || [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
    print_warning "CI mode detected - skipping Ollama installation (large binary)"
    print_status "Ollama can be installed manually with: curl -fsSL https://ollama.com/install.sh | sh"
    exit 0
fi

# Check if already installed
if command_exists ollama; then
    current_version=$(ollama --version 2>/dev/null | head -1 || echo "unknown")
    print_warning "Ollama already installed: $current_version"
    print_status "To upgrade, run: curl -fsSL https://ollama.com/install.sh | sh"
    exit 0
fi

# Install Ollama using official installer
# The installer downloads ~800MB binary, so this may take time on slow networks
print_status "Downloading Ollama binary (this may take several minutes)..."
print_status "The binary is approximately 800MB - download time depends on network speed"

if curl -fsSL https://ollama.com/install.sh | sh 2>&1; then
    if command_exists ollama; then
        installed_version=$(ollama --version 2>/dev/null | head -1 || echo "unknown")
        print_success "Ollama installed: $installed_version"
        print_status "Start Ollama server with: ollama serve"
        print_status "Or run in background: nohup ollama serve > ~/ollama.log 2>&1 &"
        print_status "Pull a model with: ollama pull llama3.2"
    else
        print_error "Ollama installation completed but binary not found in PATH"
        exit 1
    fi
else
    print_error "Failed to install Ollama"
    exit 1
fi

# Create workspace directory for models info
WORKSPACE="${WORKSPACE:-${HOME}/workspace}"
mkdir -p "${WORKSPACE}/extensions/ollama"

print_success "Ollama installation complete"
