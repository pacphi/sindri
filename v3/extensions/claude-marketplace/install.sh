#!/bin/bash
set -euo pipefail

# claude-marketplace install script
# Marketplace JSON templates are merged via extension.yaml conditions.
# Standalone plugins (repos without marketplace.json) are wrapped in the
# sindri-standalone local marketplace and registered here via CLI.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Ensure claude CLI is on PATH (installed to ~/.local/bin by claude-cli extension)
export PATH="${HOME}/.local/bin:${PATH}"

print_status "Installing Claude marketplace configuration..."

# Ensure Claude config directory exists (configure system will merge JSON templates)
CLAUDE_CONFIG_DIR="$HOME/.claude"
mkdir -p "$CLAUDE_CONFIG_DIR"

# Register the sindri-standalone local marketplace (wraps standalone plugins
# that lack marketplace.json into a proper marketplace catalog).
# This enables installing them via "claude plugin install plugin@sindri-standalone".
STANDALONE_MARKETPLACE="${SCRIPT_DIR}/sindri-standalone"
if command -v claude &>/dev/null && [[ -d "$STANDALONE_MARKETPLACE" ]]; then
  print_status "Registering sindri-standalone marketplace..."
  if claude plugin marketplace add "${STANDALONE_MARKETPLACE}" 2>/dev/null; then
    print_success "Registered sindri-standalone marketplace"
  else
    print_status "sindri-standalone marketplace already registered or registration deferred"
  fi
else
  if ! command -v claude &>/dev/null; then
    print_status "Claude CLI not available — standalone marketplace will be registered on first 'claude' invocation"
  fi
fi

print_success "Claude marketplace configuration complete"
print_status "Run 'claude' to activate marketplace plugins"
