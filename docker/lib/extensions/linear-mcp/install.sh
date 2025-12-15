#!/usr/bin/env bash
set -euo pipefail

# Install script for linear-mcp
# Linear MCP server for AI-powered project management integration

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/linear-mcp"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/linear-mcp/resources"

print_status "Installing Linear MCP server..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources (SKILL.md and other files)
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install linear-mcp-server package globally for npx usage
# This pre-caches the package so npx runs faster
print_status "Pre-caching linear-mcp-server npm package..."
npm install -g linear-mcp-server 2>/dev/null || true

# Create a wrapper script for easy invocation
cat > "${EXTENSION_DIR}/run-linear-mcp.sh" << 'EOF'
#!/usr/bin/env bash
# Run Linear MCP server
# Requires LINEAR_API_KEY environment variable

if [[ -z "${LINEAR_API_KEY:-}" ]]; then
    echo "Error: LINEAR_API_KEY environment variable is not set"
    echo "Get your API key from: https://linear.app/YOUR-TEAM/settings/api"
    exit 1
fi

exec npx -y linear-mcp-server "$@"
EOF
chmod +x "${EXTENSION_DIR}/run-linear-mcp.sh"

# Create Claude Code MCP configuration snippet
cat > "${EXTENSION_DIR}/claude-mcp-config.json" << 'EOF'
{
  "mcpServers": {
    "linear": {
      "command": "npx",
      "args": ["-y", "linear-mcp-server"],
      "env": {
        "LINEAR_API_KEY": "${LINEAR_API_KEY}"
      }
    }
  }
}
EOF

print_success "linear-mcp installed successfully"

# Only warn if LINEAR_API_KEY is not set
if [[ -z "${LINEAR_API_KEY:-}" ]]; then
    print_warning "Requires LINEAR_API_KEY environment variable"
    print_status "Get your API key from: https://linear.app/YOUR-TEAM/settings/api"
else
    print_success "LINEAR_API_KEY is configured"
fi

print_status "Claude Code config snippet saved to: ${EXTENSION_DIR}/claude-mcp-config.json"
