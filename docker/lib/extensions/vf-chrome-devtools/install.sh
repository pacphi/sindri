#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-chrome-devtools
# VisionFlow capability: Chrome DevTools Protocol integration

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-chrome-devtools"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-chrome-devtools/resources"

print_status "Installing Chrome DevTools MCP server..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Node.js dependencies
npm install -g chrome-devtools-mcp 2>/dev/null || {
    cd "${EXTENSION_DIR}"
    npm install chrome-remote-interface @modelcontextprotocol/sdk
}

print_success "vf-chrome-devtools installed successfully"
