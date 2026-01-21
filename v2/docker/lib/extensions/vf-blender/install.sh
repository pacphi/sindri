#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-blender
# VisionFlow capability: Blender 3D modeling with MCP server

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-blender"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-blender/resources"

print_status "Installing Blender 3D..."

# Install Blender via apt
sudo apt-get update -qq
sudo apt-get install -y -qq blender

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install MCP addon if present
ADDON_DIR="${HOME}/.config/blender/4.0/scripts/addons"
mkdir -p "${ADDON_DIR}"
if [[ -d "${EXTENSION_DIR}/addon" ]]; then
    cp -r "${EXTENSION_DIR}/addon"/* "${ADDON_DIR}/"
fi

# Install Python dependencies
pip install --user uvx bpy mcp pydantic

print_success "vf-blender installed successfully"
print_status "Requires desktop environment (xfce-ubuntu) for GUI"
print_status "MCP socket: 9876"
