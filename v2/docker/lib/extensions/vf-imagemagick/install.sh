#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-imagemagick
# VisionFlow capability: ImageMagick image processing with MCP server

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-imagemagick"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-imagemagick/resources"

print_status "Installing ImageMagick..."

# Install ImageMagick via apt
sudo apt-get update -qq
sudo apt-get install -y -qq imagemagick imagemagick-doc

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Copy MCP server if present
if [[ -d "${EXTENSION_DIR}/mcp-server" ]]; then
    print_status "Setting up MCP server..."
    chmod +x "${EXTENSION_DIR}/mcp-server"/*.py 2>/dev/null || true
fi

print_success "vf-imagemagick installed successfully"
