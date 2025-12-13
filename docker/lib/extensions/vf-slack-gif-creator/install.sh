#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-slack-gif-creator
# VisionFlow capability: Slack GIF generation with 13 animation templates

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-slack-gif-creator"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-slack-gif-creator/resources"

print_status "Installing Slack GIF Creator..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Python dependencies for GIF processing
pip install --user Pillow imageio imageio-ffmpeg

# Make tools executable
if [[ -d "${EXTENSION_DIR}/tools" ]]; then
    chmod +x "${EXTENSION_DIR}/tools"/*.py 2>/dev/null || true
fi

print_success "vf-slack-gif-creator installed successfully"
print_status "13 animation templates available"
