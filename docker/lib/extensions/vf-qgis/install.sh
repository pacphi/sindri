#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-qgis
# VisionFlow capability: QGIS GIS operations

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-qgis"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-qgis/resources"

print_status "Installing QGIS..."

# Install QGIS via apt
sudo apt-get update -qq
sudo apt-get install -y -qq qgis qgis-plugin-grass

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install MCP dependencies
pip install --user mcp pydantic httpx

# Make tools executable
if [[ -d "${EXTENSION_DIR}/tools" ]]; then
    chmod +x "${EXTENSION_DIR}/tools"/*.py 2>/dev/null || true
fi

print_success "vf-qgis installed successfully"
print_status "Requires desktop environment (xfce-ubuntu) for GUI"
print_status "MCP socket: 9877"
