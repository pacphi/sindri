#!/usr/bin/env bash
set -euo pipefail

# Remove script for Docker
# Stops daemon and cleans up auto-start configuration

# Find common.sh relative to this script's location
# Script is at: /opt/sindri/extensions/docker/remove.sh
# common.sh is at: /opt/sindri/common.sh (go up 2 levels)
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$(dirname "$(dirname "$SCRIPT_DIR")")/common.sh"

print_status "Stopping Docker daemon..."

# Stop Docker daemon if running
if pgrep -x dockerd > /dev/null 2>&1; then
    sudo pkill dockerd || true
    sleep 2
    print_success "Docker daemon stopped"
else
    print_status "Docker daemon not running"
fi

# Remove auto-start script
if [[ -f "$HOME/.local/bin/start-dockerd.sh" ]]; then
    rm -f "$HOME/.local/bin/start-dockerd.sh"
    print_status "Removed Docker startup script"
fi

# Remove from .bashrc
if [[ -f "$HOME/.bashrc" ]]; then
    # Create temp file without Docker auto-start section
    grep -v "start-dockerd.sh\|Auto-start Docker daemon" "$HOME/.bashrc" > "$HOME/.bashrc.tmp" || true
    mv "$HOME/.bashrc.tmp" "$HOME/.bashrc"
    print_status "Removed Docker auto-start from .bashrc"
fi

# Remove user from docker group (optional, commented out to preserve permissions)
# sudo gpasswd -d "$USER" docker 2>/dev/null || true

print_success "Docker cleanup complete"
