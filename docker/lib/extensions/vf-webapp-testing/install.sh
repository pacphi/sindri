#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-webapp-testing
# VisionFlow capability: Web app testing framework

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-webapp-testing"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-webapp-testing/resources"

print_status "Installing Web App Testing framework..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install testing dependencies
cd "${EXTENSION_DIR}"
npm init -y 2>/dev/null || true
npm install playwright @playwright/test

print_success "vf-webapp-testing installed successfully"
