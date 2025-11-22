#!/bin/bash
# Fast initialization entrypoint

set -e

# Source common functions
source /docker/lib/common.sh

# Setup MOTD banner
setup_motd() {
    if [[ -f "/docker/scripts/setup-motd.sh" ]]; then
        bash /docker/scripts/setup-motd.sh
    fi
}

# Check if workspace is initialized
if [[ ! -f "/workspace/.initialized" ]]; then
    print_status "Initializing workspace..."

    # Create directory structure
    mkdir -p /workspace/{projects,config,scripts,bin,.local,.config}
    mkdir -p /workspace/.system/{manifest,installed,logs}

    # Initialize mise for developer
    if command -v mise >/dev/null 2>&1; then
        mise activate bash >> /workspace/.bashrc
    fi

    # Copy extension manager to workspace
    if [[ -d /docker/lib/cli ]]; then
        cp -r /docker/lib/cli /workspace/.system/
    fi

    # Setup MOTD
    setup_motd

    # Mark as initialized
    touch /workspace/.initialized
    print_success "Workspace initialized"
fi

# Execute command or shell
exec "$@"
