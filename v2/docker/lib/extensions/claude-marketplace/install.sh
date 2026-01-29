#!/bin/bash
set -euo pipefail

# claude-marketplace install script
# Merges JSON marketplace configuration into ~/.claude/settings.json

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing Claude marketplace configuration..."

CLAUDE_CONFIG_DIR="$HOME/.claude"
SETTINGS_FILE="$CLAUDE_CONFIG_DIR/settings.json"

# Ensure config directory exists
mkdir -p "$CLAUDE_CONFIG_DIR"

# Select JSON template based on CI environment
if [[ "${CI:-}" == "true" ]] || [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  JSON_TEMPLATE="$(dirname "${BASH_SOURCE[0]}")/marketplaces.ci.json"
  print_status "Using CI marketplace configuration (3 marketplaces)"
else
  JSON_TEMPLATE="$(dirname "${BASH_SOURCE[0]}")/marketplaces.local.json"
  print_status "Using local marketplace configuration (8 marketplaces)"
fi

# Initialize settings.json if it doesn't exist
[[ ! -f "$SETTINGS_FILE" ]] && echo '{}' > "$SETTINGS_FILE"

# Merge marketplace JSON into settings.json using jq
TEMP_FILE=$(mktemp)
jq -s '.[0] * .[1]' "$SETTINGS_FILE" "$JSON_TEMPLATE" > "$TEMP_FILE"
mv "$TEMP_FILE" "$SETTINGS_FILE"

print_success "Claude marketplace configuration complete"
print_status "Run 'claude' to activate marketplace plugins"
