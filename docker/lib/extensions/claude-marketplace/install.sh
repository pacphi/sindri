#!/bin/bash
set -euo pipefail

# claude-marketplace install script - Simplified for YAML-driven architecture
# Configures Claude Code marketplace YAML and merges into settings.json

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing Claude marketplace configuration..."

CLAUDE_CONFIG_DIR="$HOME/.claude"
WORKSPACE="${WORKSPACE:-${HOME}/workspace}"
MARKETPLACES_CONFIG_DIR="${WORKSPACE}/config"

# Ensure config directory exists
mkdir -p "$MARKETPLACES_CONFIG_DIR"
mkdir -p "$CLAUDE_CONFIG_DIR"

# Select YAML based on CI mode
if [[ "${CI:-}" == "true" ]] || [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  YAML_FILE="marketplaces.ci.yml"
else
  YAML_FILE="marketplaces.yml"
fi

MARKETPLACES_FILE="$MARKETPLACES_CONFIG_DIR/$YAML_FILE"
EXAMPLE_FILE="$(dirname "${BASH_SOURCE[0]}")/$YAML_FILE.example"

# Copy example if needed
[[ ! -f "$MARKETPLACES_FILE" ]] && [[ -f "$EXAMPLE_FILE" ]] && cp "$EXAMPLE_FILE" "$MARKETPLACES_FILE"

print_success "Claude marketplace configuration complete"
print_status "Run 'claude' to activate marketplace plugins"
