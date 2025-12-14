#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-mcp-builder
# VisionFlow capability: MCP server scaffolding

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-mcp-builder"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-mcp-builder/resources"

print_status "Installing MCP Builder tools..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources (templates, reference files)
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install MCP SDK if not already installed
npm install -g @modelcontextprotocol/sdk 2>/dev/null || true

# Make scripts executable
if [[ -d "${EXTENSION_DIR}/scripts" ]]; then
    chmod +x "${EXTENSION_DIR}/scripts"/*.sh 2>/dev/null || true
fi

print_success "vf-mcp-builder installed successfully"
print_status "Templates available in: ${EXTENSION_DIR}/"
