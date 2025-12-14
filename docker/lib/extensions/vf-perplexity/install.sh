#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-perplexity
# VisionFlow capability: Perplexity AI real-time web research MCP server

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-perplexity"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-perplexity/resources"

print_status "Installing Perplexity MCP server..."

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
        npm install @modelcontextprotocol/sdk axios
    fi
fi

# Install Python client if present
if [[ -d "${EXTENSION_DIR}/tools" ]]; then
    pip install --user requests httpx
fi

print_success "vf-perplexity installed successfully"
print_warning "Requires PERPLEXITY_API_KEY environment variable"
