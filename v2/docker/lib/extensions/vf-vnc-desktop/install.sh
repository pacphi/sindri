#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-vnc-desktop
# VisionFlow capability: VNC desktop with 9 color-coded terminals

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-vnc-desktop"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-vnc-desktop/resources"

print_status "Installing VNC Desktop..."

# Install VNC and display packages
sudo apt-get update -qq
sudo apt-get install -y -qq \
    xvfb \
    x11vnc \
    openbox \
    tint2 \
    xfce4-terminal \
    dbus-x11

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources (supervisord config, terminal init scripts)
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Make scripts executable
chmod +x "${EXTENSION_DIR}"/*.sh 2>/dev/null || true
if [[ -d "${EXTENSION_DIR}/terminal-init" ]]; then
    chmod +x "${EXTENSION_DIR}/terminal-init"/*.sh 2>/dev/null || true
fi

print_success "vf-vnc-desktop installed successfully"
print_status "VNC port: 5901"
print_status "9 color-coded terminals in 3x3 grid"
