#!/bin/bash
set -euo pipefail

# ollama uninstall script

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Uninstalling Ollama..."

# Stop ollama service if running
if pgrep -x "ollama" > /dev/null 2>&1; then
    print_status "Stopping Ollama service..."
    pkill -x "ollama" 2>/dev/null || true
    sleep 2
fi

# Remove binary (installed to /usr/local/bin by default)
if [[ -f /usr/local/bin/ollama ]]; then
    print_status "Removing Ollama binary..."
    sudo rm -f /usr/local/bin/ollama 2>/dev/null || rm -f /usr/local/bin/ollama 2>/dev/null || true
fi

# Remove configuration and model data
if [[ -d "${HOME}/.ollama" ]]; then
    print_status "Removing Ollama data directory..."
    rm -rf "${HOME}/.ollama"
fi

# Remove workspace directory
WORKSPACE="${WORKSPACE:-${HOME}/workspace}"
if [[ -d "${WORKSPACE}/extensions/ollama" ]]; then
    rm -rf "${WORKSPACE}/extensions/ollama"
fi

print_success "Ollama uninstalled"
