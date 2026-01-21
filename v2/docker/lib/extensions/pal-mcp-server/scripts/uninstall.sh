#!/usr/bin/env bash
set -euo pipefail

# pal-mcp-server Uninstaller

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/pal-mcp-server"
CLAUDE_SETTINGS="${HOME}/.claude/settings.json"

info "Uninstalling pal-mcp-server..."

# Remove from Claude Code settings
if [[ -f "${CLAUDE_SETTINGS}" ]]; then
    info "Removing pal-mcp-server from Claude Code configuration..."

    python3 -c "
import json
import sys

try:
    with open('${CLAUDE_SETTINGS}', 'r') as f:
        settings = json.load(f)

    if 'mcpServers' in settings and 'pal' in settings['mcpServers']:
        del settings['mcpServers']['pal']

        with open('${CLAUDE_SETTINGS}', 'w') as f:
            json.dump(settings, f, indent=2)

        print('Removed pal from Claude Code configuration')
        sys.exit(0)
    else:
        print('pal not found in Claude Code configuration')
        sys.exit(0)
except Exception as e:
    print(f'Warning: Could not update Claude Code settings: {e}', file=sys.stderr)
    sys.exit(0)
" && success "Removed from Claude Code" || warning "Failed to update Claude Code settings"
fi

# Remove installation directory
if [[ -d "${EXTENSION_DIR}" ]]; then
    info "Removing ${EXTENSION_DIR}..."
    rm -rf "${EXTENSION_DIR}"
    success "Removed pal-mcp-server directory"
else
    info "Installation directory not found (already removed)"
fi

success "Uninstalled pal-mcp-server"
info "You may need to restart Claude Code for changes to take effect"
