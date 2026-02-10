#!/usr/bin/env bash
set -euo pipefail

# Install script for jira-mcp
# Atlassian MCP server using Claude Code's native SSE transport
# See: https://support.atlassian.com/atlassian-rovo-mcp-server/

# Find common.sh and resources relative to this script's location
# Script is at: /opt/sindri/extensions/jira-mcp/install.sh
# common.sh is at: /opt/sindri/common.sh (go up 2 levels)
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$(dirname "$(dirname "$SCRIPT_DIR")")/common.sh"

EXTENSION_DIR="${HOME}/extensions/jira-mcp"
RESOURCE_DIR="$SCRIPT_DIR/resources"
MCP_SERVER_NAME="atlassian"
ATLASSIAN_MCP_URL="https://mcp.atlassian.com/v1/sse"

print_status "Installing Atlassian MCP server (native SSE transport)..."

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

# Add Atlassian MCP server using native SSE transport
# Claude Code supports direct SSE connections to remote MCP servers
# See: https://docs.anthropic.com/en/docs/claude-code/mcp
print_status "Adding Atlassian MCP to Claude Code (user scope, SSE transport)..."

# Use claude mcp add with SSE transport - no mcp-remote wrapper needed
if claude mcp add --transport sse --scope user "${MCP_SERVER_NAME}" "${ATLASSIAN_MCP_URL}" 2>/dev/null; then
    print_success "Atlassian MCP added to user scope"
else
    # Fallback: Check if already exists
    if claude mcp list --scope user 2>/dev/null | grep -q "${MCP_SERVER_NAME}"; then
        print_warning "Atlassian MCP already configured in user scope"
    else
        # Try add-json as alternative
        print_status "Trying add-json approach..."
        MCP_CONFIG='{"type":"sse","url":"'"${ATLASSIAN_MCP_URL}"'"}'
        if claude mcp add-json --scope user "${MCP_SERVER_NAME}" "${MCP_CONFIG}" 2>/dev/null; then
            print_success "Atlassian MCP added via add-json"
        else
            print_warning "Could not add via CLI, creating config snippet for manual setup"
            # Save config snippet for manual installation
            cat > "${EXTENSION_DIR}/claude-mcp-config.json" << EOF
{
  "mcpServers": {
    "${MCP_SERVER_NAME}": {
      "type": "sse",
      "url": "${ATLASSIAN_MCP_URL}"
    }
  }
}
EOF
            print_status "Config saved to: ${EXTENSION_DIR}/claude-mcp-config.json"
            print_status "To add manually: claude mcp add --transport sse --scope user ${MCP_SERVER_NAME} ${ATLASSIAN_MCP_URL}"
        fi
    fi
fi

# Save installation metadata
cat > "${EXTENSION_DIR}/installation-info.json" << EOF
{
  "version": "2.0.0",
  "type": "remote-mcp-sse",
  "transport": "sse",
  "url": "${ATLASSIAN_MCP_URL}",
  "scope": "user",
  "installed_at": "$(date -Iseconds)",
  "auth_method": "oauth"
}
EOF

print_success "jira-mcp installed successfully"
print_status ""
print_status "To complete setup:"
print_status "  1. Run '/mcp' in Claude Code to trigger OAuth authentication"
print_status "  2. Click 'Connect Atlassian Account' and authorize in your browser"
print_status "  3. Grant access to Jira and/or Confluence"
print_status "  4. Start using Atlassian tools in Claude Code"
print_status ""
print_status "Available commands after authentication:"
print_status "  - 'Search for open bugs in project BACKEND'"
print_status "  - 'Create a new issue in [project]'"
print_status "  - 'What issues are assigned to me?'"
print_status "  - 'Find Confluence pages about [topic]'"
