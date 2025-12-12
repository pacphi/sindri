#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-canvas-design
# VisionFlow capability: Design system framework

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-canvas-design"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-canvas-design/resources"

print_status "Installing Canvas Design tools..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources (36+ font families, brand guidelines)
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install PDF generation dependencies
pip install --user reportlab matplotlib pillow svgwrite

print_success "vf-canvas-design installed successfully"
print_status "36+ font families available"
