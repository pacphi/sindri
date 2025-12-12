#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-deepseek-reasoning
# VisionFlow capability: Deepseek AI reasoning

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-deepseek-reasoning"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-deepseek-reasoning/resources"

print_status "Installing Deepseek Reasoning MCP server..."

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

print_success "vf-deepseek-reasoning installed successfully"
print_warning "Requires DEEPSEEK_API_KEY environment variable"
