#!/usr/bin/env bash
set -euo pipefail

# Uninstall script for linear-mcp
# Removes Linear MCP from Claude Code user scope

MCP_SERVER_NAME="linear"

print_status "Removing Linear MCP server..."

# Ensure ~/.local/bin is in PATH (where claude CLI is installed)
export PATH="${HOME}/.local/bin:${PATH}"

# Remove from Claude Code MCP configuration
if command -v claude &>/dev/null; then
    if claude mcp remove --scope user "${MCP_SERVER_NAME}" 2>/dev/null; then
        print_success "Linear MCP removed from user scope"
    else
        print_warning "Could not remove Linear MCP from Claude Code config"
        print_status "You may need to manually edit ~/.claude.json"
    fi
fi

# Note: Extension directory cleanup is handled by the remove.paths in extension.yaml

print_success "linear-mcp uninstalled"
