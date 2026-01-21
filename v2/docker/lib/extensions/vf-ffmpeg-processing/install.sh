#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-ffmpeg-processing
# VisionFlow capability: FFmpeg professional video/audio transcoding

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-ffmpeg-processing"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-ffmpeg-processing/resources"

print_status "Installing FFmpeg..."

# Install FFmpeg via apt
sudo apt-get update -qq
sudo apt-get install -y -qq ffmpeg

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

print_success "vf-ffmpeg-processing installed successfully"
