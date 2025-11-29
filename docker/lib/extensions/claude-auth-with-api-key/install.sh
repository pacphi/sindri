#!/bin/bash
set -euo pipefail

# claude-auth-with-api-key install script - Simplified for YAML-driven architecture
# Sets up Claude Code CLI authentication via wrapper script

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Configuring Claude Code authentication..."

# Claude CLI is pre-installed in base image
print_success "Claude Code CLI pre-installed"

# Use WORKSPACE from environment or derive from HOME
WORKSPACE="${WORKSPACE:-${HOME}/workspace}"

# Install wrapper script for automatic API key authentication
wrapper_source="$(dirname "${BASH_SOURCE[0]}")/claude-wrapper.sh"
wrapper_dest="${WORKSPACE}/bin/claude"

if [[ -f "$wrapper_source" ]]; then
  print_status "Installing Claude wrapper for automatic authentication..."

  # Ensure workspace bin exists
  mkdir -p "${WORKSPACE}/bin"

  # Copy wrapper script
  cp "$wrapper_source" "$wrapper_dest"
  chmod +x "$wrapper_dest"

  # Ensure workspace/bin is first in PATH
  if ! grep -q 'workspace/bin' "$HOME/.bashrc" 2>/dev/null; then
    echo '' >> "$HOME/.bashrc"
    echo '# Ensure workspace/bin is first in PATH (for Claude wrapper)' >> "$HOME/.bashrc"
    echo 'export PATH="${WORKSPACE:-${HOME}/workspace}/bin:$PATH"' >> "$HOME/.bashrc"
    print_status "Added workspace/bin to PATH"
  fi

  print_success "Claude wrapper installed - authentication will be automatic"
else
  print_warning "Wrapper script not found"
fi

print_success "Claude Code authentication configuration complete"
