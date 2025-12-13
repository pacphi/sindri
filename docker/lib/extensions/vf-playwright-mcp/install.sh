#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-playwright-mcp
# VisionFlow capability: Playwright browser automation MCP

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-playwright-mcp"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-playwright-mcp/resources"

print_status "Installing Playwright MCP server..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install MCP dependencies
if [[ -d "${EXTENSION_DIR}/mcp-server" ]]; then
    cd "${EXTENSION_DIR}/mcp-server"
    if [[ -f "package.json" ]]; then
        npm install --production
    else
        npm install @modelcontextprotocol/sdk playwright
    fi
fi

# Make tools executable
if [[ -d "${EXTENSION_DIR}/tools" ]]; then
    chmod +x "${EXTENSION_DIR}/tools"/*.js 2>/dev/null || true
fi

print_success "vf-playwright-mcp installed successfully"
