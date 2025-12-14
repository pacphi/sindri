#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-zai-service
# VisionFlow capability: Cost-effective Claude API wrapper

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-zai-service"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-zai-service/resources"

print_status "Installing Z.AI Service..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install dependencies
cd "${EXTENSION_DIR}"
npm init -y 2>/dev/null || true
npm install @anthropic-ai/sdk express pm2

print_success "vf-zai-service installed successfully"
print_warning "Requires ZAI_ANTHROPIC_API_KEY environment variable"
print_status "Port: 9600 (internal)"
