#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-gemini-flow
# VisionFlow capability: Gemini multi-agent orchestration

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-gemini-flow"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-gemini-flow/resources"

print_status "Installing Gemini Flow orchestration..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install dependencies
cd "${EXTENSION_DIR}"
npm init -y 2>/dev/null || true
npm install @google/generative-ai pm2

print_success "vf-gemini-flow installed successfully"
print_warning "Requires GOOGLE_GEMINI_API_KEY environment variable"
