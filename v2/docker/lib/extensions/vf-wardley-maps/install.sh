#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-wardley-maps
# VisionFlow capability: Strategic Wardley mapping

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-wardley-maps"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-wardley-maps/resources"

print_status "Installing Wardley Maps tools..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Node.js dependencies if package.json exists
if [[ -f "${EXTENSION_DIR}/package.json" ]]; then
    cd "${EXTENSION_DIR}"
    npm install --production
fi

# Make tools executable
if [[ -d "${EXTENSION_DIR}/tools" ]]; then
    chmod +x "${EXTENSION_DIR}/tools"/*.py 2>/dev/null || true
    chmod +x "${EXTENSION_DIR}/tools"/*.js 2>/dev/null || true
fi

print_success "vf-wardley-maps installed successfully"
