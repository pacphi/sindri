#!/usr/bin/env bash
set -euo pipefail

# Install script for context7-mcp
# Context7 MCP server using Claude Code's native HTTP transport
# See: https://github.com/upstash/context7

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/context7-mcp"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/context7-mcp/resources"
MCP_SERVER_NAME="context7"
CONTEXT7_MCP_URL="https://mcp.context7.com/mcp"

print_status "Installing Context7 MCP server (native HTTP transport)..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources (SKILL.md and other files)
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Check if claude CLI is available
if ! command -v claude &>/dev/null; then
    print_error "Claude Code CLI not found"
    print_status "Please ensure Claude Code is installed"
    exit 1
fi

# Check if API key is set (optional)
HAS_API_KEY=false
if [[ -n "${CONTEXT7_API_KEY:-}" ]]; then
    HAS_API_KEY=true
    print_status "API key detected - will configure with authentication"
else
    print_status "No API key detected - installing without authentication (limited rate limits)"
    print_status "To get an API key: https://context7.com/dashboard"
fi

# Add Context7 MCP server using native HTTP transport
print_status "Adding Context7 MCP to Claude Code (user scope, HTTP transport)..."

# Build command with optional API key header
if [[ "${HAS_API_KEY}" == "true" ]]; then
    # Use --header flag for API key authentication
    if claude mcp add --transport http --scope user --header "CONTEXT7_API_KEY: ${CONTEXT7_API_KEY}" "${MCP_SERVER_NAME}" "${CONTEXT7_MCP_URL}" 2>/dev/null; then
        print_success "Context7 MCP added to user scope (with API key)"
    else
        # Fallback: Try add-json with headers
        print_status "Trying add-json approach with API key..."
        MCP_CONFIG='{"type":"http","url":"'"${CONTEXT7_MCP_URL}"'","headers":{"CONTEXT7_API_KEY":"'"${CONTEXT7_API_KEY}"'"}}'
        if claude mcp add-json --scope user "${MCP_SERVER_NAME}" "${MCP_CONFIG}" 2>/dev/null; then
            print_success "Context7 MCP added via add-json (with API key)"
        else
            print_warning "Could not add via CLI, creating config snippet for manual setup"
            save_manual_config_with_api_key
        fi
    fi
else
    # No API key - simple HTTP transport
    if claude mcp add --transport http --scope user "${MCP_SERVER_NAME}" "${CONTEXT7_MCP_URL}" 2>/dev/null; then
        print_success "Context7 MCP added to user scope (no API key)"
    else
        # Fallback: Check if already exists
        if claude mcp list --scope user 2>/dev/null | grep -q "${MCP_SERVER_NAME}"; then
            print_warning "Context7 MCP already configured in user scope"
        else
            # Try add-json as alternative
            print_status "Trying add-json approach..."
            MCP_CONFIG='{"type":"http","url":"'"${CONTEXT7_MCP_URL}"'"}'
            if claude mcp add-json --scope user "${MCP_SERVER_NAME}" "${MCP_CONFIG}" 2>/dev/null; then
                print_success "Context7 MCP added via add-json (no API key)"
            else
                print_warning "Could not add via CLI, creating config snippet for manual setup"
                save_manual_config_no_api_key
            fi
        fi
    fi
fi

# Save installation metadata
cat > "${EXTENSION_DIR}/installation-info.json" << EOF
{
  "version": "1.0.0",
  "type": "remote-mcp-http",
  "transport": "http",
  "url": "${CONTEXT7_MCP_URL}",
  "scope": "user",
  "installed_at": "$(date -Iseconds)",
  "auth_method": "$(if [[ "${HAS_API_KEY}" == "true" ]]; then echo "api_key"; else echo "none"; fi)",
  "has_api_key": ${HAS_API_KEY}
}
EOF

print_success "context7-mcp installed successfully"
print_status ""
print_status "Context7 MCP Features:"
print_status "  - resolve-library-id: Find correct library identifiers"
print_status "  - get-library-docs: Get version-specific documentation"
print_status ""
if [[ "${HAS_API_KEY}" == "false" ]]; then
    print_status "Note: Running without API key (limited rate limits)"
    print_status "For higher limits, get a free API key at: https://context7.com/dashboard"
    print_status "Then add to sindri.yaml secrets and reinstall"
fi
print_status ""
print_status "Example usage:"
print_status "  - 'What's the latest React documentation?'"
print_status "  - 'Show me how to use pandas 2.0 DataFrame'"
print_status "  - 'Get FastAPI authentication docs'"
print_status ""

# Helper functions for manual config
save_manual_config_with_api_key() {
    cat > "${EXTENSION_DIR}/claude-mcp-config.json" << EOF
{
  "mcpServers": {
    "${MCP_SERVER_NAME}": {
      "type": "http",
      "url": "${CONTEXT7_MCP_URL}",
      "headers": {
        "CONTEXT7_API_KEY": "${CONTEXT7_API_KEY}"
      }
    }
  }
}
EOF
    print_status "Config saved to: ${EXTENSION_DIR}/claude-mcp-config.json"
    print_status "To add manually: claude mcp add --transport http --scope user --header 'CONTEXT7_API_KEY: YOUR_KEY' ${MCP_SERVER_NAME} ${CONTEXT7_MCP_URL}"
}

save_manual_config_no_api_key() {
    cat > "${EXTENSION_DIR}/claude-mcp-config.json" << EOF
{
  "mcpServers": {
    "${MCP_SERVER_NAME}": {
      "type": "http",
      "url": "${CONTEXT7_MCP_URL}"
    }
  }
}
EOF
    print_status "Config saved to: ${EXTENSION_DIR}/claude-mcp-config.json"
    print_status "To add manually: claude mcp add --transport http --scope user ${MCP_SERVER_NAME} ${CONTEXT7_MCP_URL}"
}
