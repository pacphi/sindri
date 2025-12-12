#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-kicad
# VisionFlow capability: KiCad PCB design

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-kicad"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-kicad/resources"

print_status "Installing KiCad PCB design suite..."

# Install KiCad via apt
sudo apt-get update -qq
sudo apt-get install -y -qq kicad kicad-libraries

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Python dependencies for scripting
pip install --user kicad-skip mcp pydantic

print_success "vf-kicad installed successfully"
print_status "Requires desktop environment (xfce-ubuntu) for GUI"
