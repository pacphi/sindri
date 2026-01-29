#!/bin/bash
set -euo pipefail

# claude-marketplace install script
# Template selection is now handled declaratively via extension.yaml conditions

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing Claude marketplace configuration..."

# Ensure Claude config directory exists (configure system will merge JSON templates)
CLAUDE_CONFIG_DIR="$HOME/.claude"
mkdir -p "$CLAUDE_CONFIG_DIR"

print_success "Claude marketplace configuration complete"
print_status "Run 'claude' to activate marketplace plugins"
print_status "Note: Template selection (CI vs local) is handled automatically based on environment"
