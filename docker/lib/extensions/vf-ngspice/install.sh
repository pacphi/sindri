#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-ngspice
# VisionFlow capability: NGSpice circuit simulation

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-ngspice"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-ngspice/resources"

print_status "Installing NGSpice..."

# Install NGSpice via apt
sudo apt-get update -qq
sudo apt-get install -y -qq ngspice

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Set up tools if present
if [[ -d "${EXTENSION_DIR}/tools" ]]; then
    chmod +x "${EXTENSION_DIR}/tools"/*.py 2>/dev/null || true
fi

print_success "vf-ngspice installed successfully"
