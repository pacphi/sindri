#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-pptx
# VisionFlow capability: PowerPoint manipulation

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-pptx"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-pptx/resources"

print_status "Installing PPTX processing tools..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Python dependencies
pip install --user python-pptx Pillow lxml

# Make scripts executable
if [[ -d "${EXTENSION_DIR}/scripts" ]]; then
    chmod +x "${EXTENSION_DIR}/scripts"/*.py 2>/dev/null || true
fi

print_success "vf-pptx installed successfully"
