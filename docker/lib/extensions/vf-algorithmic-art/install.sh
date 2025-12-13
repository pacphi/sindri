#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-algorithmic-art
# VisionFlow capability: Generative algorithmic art

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-algorithmic-art"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-algorithmic-art/resources"

print_status "Installing Algorithmic Art tools..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Python dependencies for art generation
pip install --user Pillow numpy matplotlib svgwrite

# Make tools executable
if [[ -d "${EXTENSION_DIR}/tools" ]]; then
    chmod +x "${EXTENSION_DIR}/tools"/*.py 2>/dev/null || true
fi

print_success "vf-algorithmic-art installed successfully"
