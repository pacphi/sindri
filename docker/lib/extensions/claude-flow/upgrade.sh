#!/bin/bash
set -euo pipefail
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Upgrading Claude Flow..."

# Check if installed
if ! command -v claude-flow >/dev/null 2>&1; then
    print_error "Claude Flow is not installed"
    print_status "Install with: extension-manager install claude-flow"
    exit 1
fi

old_version=$(claude-flow --version 2>/dev/null || echo "unknown")
print_status "Current version: $old_version"

# Upgrade via npm (reinstall alpha tag to get latest alpha)
print_status "Upgrading claude-flow@alpha via npm..."
if npm install -g claude-flow@alpha; then
    # Refresh mise shims
    if command -v mise >/dev/null 2>&1; then
        mise reshim 2>/dev/null || true
    fi
    hash -r 2>/dev/null || true

    new_version=$(claude-flow --version 2>/dev/null || echo "unknown")
    if [[ "$old_version" != "$new_version" ]]; then
        print_success "Claude Flow upgraded: $old_version -> $new_version"
    else
        print_success "Claude Flow is already at the latest version: $new_version"
    fi
else
    print_error "Failed to upgrade Claude Flow"
    exit 1
fi

# Update Claude Code commands
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
COMMANDS_SRC="$SCRIPT_DIR/commands"
COMMANDS_DEST="$HOME/.claude/commands"

if [[ -d "$COMMANDS_SRC" ]]; then
    print_status "Updating Claude Code commands..."
    mkdir -p "$COMMANDS_DEST"

    for cmd_file in "$COMMANDS_SRC"/*.md; do
        if [[ -f "$cmd_file" ]]; then
            cmd_name=$(basename "$cmd_file")
            cp "$cmd_file" "$COMMANDS_DEST/$cmd_name"
            print_success "Updated command: /${cmd_name%.md}"
        fi
    done
fi

print_success "Claude Flow upgrade complete"
