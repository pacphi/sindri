#!/usr/bin/env bash
set -euo pipefail

# Install script for linear-mcp
# Linear MCP server using Claude Code's native HTTP transport
# See: https://linear.app/docs/mcp

# Find common.sh and resources relative to this script's location
# Script is at: /opt/sindri/extensions/linear-mcp/install.sh
# common.sh is at: /opt/sindri/common.sh (go up 2 levels)
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$(dirname "$(dirname "$SCRIPT_DIR")")/common.sh"

EXTENSION_DIR="${HOME}/extensions/linear-mcp"
RESOURCE_DIR="$SCRIPT_DIR/resources"
MCP_SERVER_NAME="linear"
LINEAR_MCP_URL="https://mcp.linear.app/mcp"

print_status "Installing Linear MCP server (native HTTP transport)..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources (SKILL.md and other files)
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Ensure ~/.local/bin is in PATH (where claude CLI is installed)
export PATH="${HOME}/.local/bin:${PATH}"

# Check if claude CLI is available
if ! command -v claude &>/dev/null; then
    print_error "Claude Code CLI not found"
    print_status "Please ensure Claude Code is installed"
    exit 1
fi

# Add Linear MCP server using native HTTP transport
# Claude Code supports direct HTTP connections to remote MCP servers
# See: https://code.claude.com/docs/en/mcp
print_status "Adding Linear MCP to Claude Code (user scope, HTTP transport)..."

# Use claude mcp add with HTTP transport - no mcp-remote wrapper needed
if claude mcp add --transport http --scope user "${MCP_SERVER_NAME}" "${LINEAR_MCP_URL}" 2>/dev/null; then
    print_success "Linear MCP added to user scope"
else
    # Fallback: Check if already exists
    if claude mcp list --scope user 2>/dev/null | grep -q "${MCP_SERVER_NAME}"; then
        print_warning "Linear MCP already configured in user scope"
    else
        # Try add-json as alternative
        print_status "Trying add-json approach..."
        MCP_CONFIG='{"type":"http","url":"'"${LINEAR_MCP_URL}"'"}'
        if claude mcp add-json --scope user "${MCP_SERVER_NAME}" "${MCP_CONFIG}" 2>/dev/null; then
            print_success "Linear MCP added via add-json"
        else
            print_warning "Could not add via CLI, creating config snippet for manual setup"
            # Save config snippet for manual installation
            cat > "${EXTENSION_DIR}/claude-mcp-config.json" << EOF
{
  "mcpServers": {
    "${MCP_SERVER_NAME}": {
      "type": "http",
      "url": "${LINEAR_MCP_URL}"
    }
  }
}
EOF
            print_status "Config saved to: ${EXTENSION_DIR}/claude-mcp-config.json"
            print_status "To add manually: claude mcp add --transport http --scope user ${MCP_SERVER_NAME} ${LINEAR_MCP_URL}"
        fi
    fi
fi

# Save installation metadata
cat > "${EXTENSION_DIR}/installation-info.json" << EOF
{
  "version": "2.1.0",
  "type": "remote-mcp-http",
  "transport": "http",
  "url": "${LINEAR_MCP_URL}",
  "scope": "user",
  "installed_at": "$(date -Iseconds)",
  "auth_method": "oauth"
}
EOF

print_success "linear-mcp installed successfully"
print_status ""
print_status "To complete setup:"
print_status "  1. Run '/mcp' in Claude Code to trigger OAuth authentication"
print_status "  2. Authorize Linear access in your browser"
print_status "  3. Start using Linear tools in Claude Code"
print_status ""
print_status "Available commands after authentication:"
print_status "  - 'List my Linear issues'"
print_status "  - 'Create a new issue in [project]'"
print_status "  - 'Search for issues about [topic]'"
