#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-web-summary
# VisionFlow capability: URL and YouTube transcript summarization

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-web-summary"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-web-summary/resources"

print_status "Installing Web Summary MCP server..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Python dependencies
pip install --user youtube-transcript-api beautifulsoup4 httpx mcp pydantic

# Install Node.js MCP dependencies
if [[ -d "${EXTENSION_DIR}/mcp-server" && -f "${EXTENSION_DIR}/mcp-server/package.json" ]]; then
    cd "${EXTENSION_DIR}/mcp-server"
    npm install --production
fi

print_success "vf-web-summary installed successfully"
