#!/usr/bin/env bash
set -euo pipefail

# Install script for linear-mcp
# Linear MCP server using official OAuth-based remote MCP
# See: https://linear.app/docs/mcp

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/linear-mcp"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/linear-mcp/resources"
MCP_SERVER_NAME="linear"
LINEAR_MCP_URL="https://mcp.linear.app/sse"

print_status "Installing Linear MCP server (OAuth-based remote MCP)..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources (SKILL.md and other files)
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Pre-cache mcp-remote package for faster startup
print_status "Pre-caching mcp-remote npm package..."
npm install -g mcp-remote 2>/dev/null || npm cache add mcp-remote 2>/dev/null || true

# Check if claude CLI is available
if ! command -v claude &>/dev/null; then
    print_error "Claude Code CLI not found"
    print_status "Please ensure Claude Code is installed: npm install -g @anthropic-ai/claude-code"
    exit 1
fi

# Add Linear MCP server to user scope using claude mcp add-json
# This merges with existing MCP servers, doesn't overwrite
# See: https://code.claude.com/docs/en/mcp#add-mcp-servers-from-json-configuration
print_status "Adding Linear MCP to Claude Code (user scope)..."

# Build the JSON configuration for the remote MCP server
MCP_CONFIG='{"command":"npx","args":["-y","mcp-remote","'"${LINEAR_MCP_URL}"'"]}'

# Use claude mcp add-json with user scope
# This automatically merges with existing mcpServers in ~/.claude.json
if claude mcp add-json --scope user "${MCP_SERVER_NAME}" "${MCP_CONFIG}" 2>/dev/null; then
    print_success "Linear MCP added to user scope"
else
    # Fallback: Check if already exists
    if claude mcp list --scope user 2>/dev/null | grep -q "${MCP_SERVER_NAME}"; then
        print_warning "Linear MCP already configured in user scope"
    else
        print_warning "Could not add via CLI, creating config snippet for manual setup"
        # Save config snippet for manual installation
        cat > "${EXTENSION_DIR}/claude-mcp-config.json" << EOF
{
  "mcpServers": {
    "${MCP_SERVER_NAME}": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "${LINEAR_MCP_URL}"]
    }
  }
}
EOF
        print_status "Config saved to: ${EXTENSION_DIR}/claude-mcp-config.json"
        print_status "To add manually: claude mcp add-json --scope user ${MCP_SERVER_NAME} '${MCP_CONFIG}'"
    fi
fi

# Save installation metadata
cat > "${EXTENSION_DIR}/installation-info.json" << EOF
{
  "version": "2.0.0",
  "type": "remote-mcp",
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
