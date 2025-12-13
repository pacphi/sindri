#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-docker-manager
# VisionFlow capability: Docker container lifecycle management

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-docker-manager"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-docker-manager/resources"

print_status "Installing Docker Manager MCP server..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Python Docker SDK
pip install --user docker mcp pydantic

# Make tools executable
if [[ -d "${EXTENSION_DIR}/tools" ]]; then
    chmod +x "${EXTENSION_DIR}/tools"/*.py 2>/dev/null || true
fi

print_success "vf-docker-manager installed successfully"
print_status "Requires Docker socket access"
