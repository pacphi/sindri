#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-pbr-rendering
# VisionFlow capability: PBR material generation

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-pbr-rendering"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-pbr-rendering/resources"

print_status "Installing PBR Rendering tools..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Python dependencies for PBR
pip install --user numpy pillow mcp pydantic

# Note: nvdiffrast requires CUDA and is installed separately
print_warning "For full GPU support, install nvdiffrast separately"

print_success "vf-pbr-rendering installed successfully"
print_status "Requires GPU (NVIDIA) for rendering"
print_status "MCP socket: 9878"
