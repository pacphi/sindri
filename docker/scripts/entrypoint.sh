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

# Fix workspace ownership with robust fallback
# Volume may be created as root-owned; we need to fix this before any other operations
fix_workspace_permissions() {
    local workspace="${1:-/workspace}"

    # Check if workspace is writable by current user
    if [[ -w "$workspace" ]]; then
        return 0
    fi

    # Try sudo chown first (preferred method)
    if command -v sudo >/dev/null 2>&1; then
        if sudo -n chown -R developer:developer "$workspace" 2>/dev/null; then
            return 0
        fi
    fi

    # If sudo failed, workspace might still be root-owned
    # This is a critical error - report it clearly
    if [[ ! -w "$workspace" ]]; then
        print_warning "Workspace $workspace is not writable by developer user"
        print_warning "Volume may need manual permission fix: docker exec --user root <container> chown -R developer:developer /workspace"
        # Don't fail - let subsequent operations report their own errors
        return 1
    fi

    return 0
}

# Initialize workspace directories
initialize_workspace_dirs() {
    local workspace="${1:-/workspace}"

    # Attempt to create directories - will fail gracefully if permissions are wrong
    mkdir -p "$workspace"/{projects,config,scripts,bin} 2>/dev/null || {
        print_error "Failed to create workspace directories - permission denied"
        return 1
    }
    mkdir -p "$workspace"/.local/{share/mise,state/mise,bin} 2>/dev/null || return 1
    mkdir -p "$workspace"/.config/mise 2>/dev/null || return 1
    mkdir -p "$workspace"/.cache/mise 2>/dev/null || return 1
    mkdir -p "$workspace"/.system/{manifest,installed,logs} 2>/dev/null || return 1

    # Create essential files if they don't exist
    touch "$workspace"/.bashrc 2>/dev/null || true
    touch "$workspace"/.profile 2>/dev/null || true

    # Ensure directories are accessible
    chmod -R 755 "$workspace"/.local "$workspace"/.config "$workspace"/.cache 2>/dev/null || true

    return 0
}

# Fix workspace permissions first
fix_workspace_permissions /workspace

# Check if workspace is initialized
if [[ ! -f "/workspace/.initialized" ]]; then
    print_status "Initializing workspace..."

    # Create directory structure
    if initialize_workspace_dirs /workspace; then
        # Setup MOTD
        setup_motd

        # Mark as initialized
        touch /workspace/.initialized
        print_success "Workspace initialized"
    else
        print_error "Failed to initialize workspace"
        # Continue anyway - let subsequent commands fail with better error messages
    fi
fi

# Execute command or shell
exec "$@"
