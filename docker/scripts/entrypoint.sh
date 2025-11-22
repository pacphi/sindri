#!/bin/bash
# Fast initialization entrypoint

set -e

# Source common functions
source /docker/lib/common.sh

# Setup MOTD banner
setup_motd() {
    if [[ -f "/docker/scripts/setup-motd.sh" ]]; then
        sudo bash /docker/scripts/setup-motd.sh 2>/dev/null || true
    fi
}

# Fix workspace ownership (volume may be created as root or have root-owned subdirs)
# Always run to ensure all subdirectories have correct ownership
sudo chown -R developer:developer /workspace 2>/dev/null || true

# Check if workspace is initialized
if [[ ! -f "/workspace/.initialized" ]]; then
    print_status "Initializing workspace..."

    # Create directory structure
    mkdir -p /workspace/{projects,config,scripts,bin}
    mkdir -p /workspace/.local/{share/mise,state/mise,bin}
    mkdir -p /workspace/.config/mise
    mkdir -p /workspace/.cache/mise
    mkdir -p /workspace/.system/{manifest,installed,logs}

    # Ensure mise directories are writable
    chmod -R 755 /workspace/.local /workspace/.config /workspace/.cache 2>/dev/null || true

    # Setup MOTD
    setup_motd

    # Mark as initialized
    touch /workspace/.initialized
    print_success "Workspace initialized"
fi

# Execute command or shell
exec "$@"
