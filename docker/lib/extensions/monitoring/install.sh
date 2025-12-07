#!/bin/bash
set -euo pipefail

# monitoring install script - Simplified for YAML-driven architecture
# Installs Claude monitoring tools: UV, claude-monitor, claude-usage-cli

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing monitoring tools..."

# Install UV package manager
if ! command_exists uv; then
  print_status "Installing UV package manager..."
  if curl -LsSf https://astral.sh/uv/install.sh | sh; then
    print_success "UV installed"
    [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
    export PATH="$HOME/.cargo/bin:$PATH"
  else
    print_warning "UV installation failed"
  fi
else
  print_success "UV already installed"
fi

# Install claude-monitor
print_status "Installing claude-monitor..."
if command_exists claude-monitor; then
  print_warning "claude-monitor already installed"
elif command_exists uv; then
  if uv tool list 2>/dev/null | grep -q "claude-monitor"; then
    print_warning "claude-monitor already installed via UV"
  elif timeout 180 uv tool install claude-monitor 2>&1; then
    print_success "claude-monitor installed via UV"
  else
    print_warning "UV install failed, trying pip3..."
    command_exists pip3 || sudo apt-get install -y python3-pip
    timeout 120 pip3 install claude-monitor 2>&1 && print_success "claude-monitor installed via pip3"
  fi
else
  # Fallback to pip3
  command_exists pip3 || sudo apt-get install -y python3-pip
  if pip3 show claude-monitor >/dev/null 2>&1; then
    print_warning "claude-monitor already installed"
  else
    timeout 120 pip3 install claude-monitor 2>&1 && print_success "claude-monitor installed"
  fi
fi

# Install claude-usage-cli via npm
if command_exists npm; then
  print_status "Installing claude-usage-cli..."
  if command_exists claude-usage; then
    print_warning "claude-usage-cli already installed"
  else
    npm install -g claude-usage-cli 2>&1 && print_success "claude-usage-cli installed"
  fi
else
  print_warning "npm not found - skipping claude-usage-cli"
fi

# Refresh mise shims for all installed tools
if command_exists mise; then
    mise reshim 2>/dev/null || true
fi
hash -r 2>/dev/null || true

print_success "Monitoring tools installation complete"
