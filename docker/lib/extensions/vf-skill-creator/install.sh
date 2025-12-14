#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-skill-creator
# VisionFlow capability: Claude Code skill scaffolding

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-skill-creator"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-skill-creator/resources"

print_status "Installing Skill Creator tools..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources (templates, reference files)
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Make scripts executable
if [[ -d "${EXTENSION_DIR}/scripts" ]]; then
    chmod +x "${EXTENSION_DIR}/scripts"/*.sh 2>/dev/null || true
fi

print_success "vf-skill-creator installed successfully"
print_status "Templates available in: ${EXTENSION_DIR}/"
