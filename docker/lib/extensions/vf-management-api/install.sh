#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-management-api
# VisionFlow capability: HTTP REST API for task orchestration

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-management-api"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-management-api/resources"

print_status "Installing Management API..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install dependencies
cd "${EXTENSION_DIR}"
npm init -y 2>/dev/null || true
npm install express cors helmet pm2 pino pino-pretty uuid

print_success "vf-management-api installed successfully"
print_status "Port: 9090"
print_warning "Set MANAGEMENT_API_KEY for authentication"
