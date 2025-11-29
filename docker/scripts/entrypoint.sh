#!/bin/bash
# Fast initialization entrypoint

set -e

# Source common functions
source /docker/lib/common.sh

# Use environment variables with fallbacks
HOME="${HOME:-/alt/home/developer}"
WORKSPACE="${WORKSPACE:-${HOME}/workspace}"

# Setup MOTD banner
setup_motd() {
    if [[ -f "/docker/scripts/setup-motd.sh" ]]; then
        sudo bash /docker/scripts/setup-motd.sh 2>/dev/null || true
    fi
}

# Fix home directory ownership with robust fallback
# Volume may be created as root-owned; we need to fix this before any other operations
fix_home_permissions() {
    local target="${1:-${HOME}}"

    # Check if target is writable by current user
    if [[ -w "$target" ]]; then
        return 0
    fi

    # Try sudo chown first (preferred method)
    if command -v sudo >/dev/null 2>&1; then
        if sudo -n chown -R developer:developer "$target" 2>/dev/null; then
            return 0
        fi
    fi

    # If sudo failed, target might still be root-owned
    # This is a critical error - report it clearly
    if [[ ! -w "$target" ]]; then
        print_warning "Home directory $target is not writable by developer user"
        print_warning "Volume may need manual permission fix: docker exec --user root <container> chown -R developer:developer $target"
        # Don't fail - let subsequent operations report their own errors
        return 1
    fi

    return 0
}

# Initialize home directories (including workspace)
initialize_home_dirs() {
    local home="${1:-${HOME}}"
    local workspace="${WORKSPACE:-${home}/workspace}"

    # Create workspace structure
    mkdir -p "$workspace"/{projects,config,scripts,bin} 2>/dev/null || {
        print_error "Failed to create workspace directories - permission denied"
        return 1
    }

    # Create XDG directories
    mkdir -p "$home"/.local/{share/mise,state/mise,bin} 2>/dev/null || return 1
    mkdir -p "$home"/.config/mise/conf.d 2>/dev/null || return 1
    mkdir -p "$home"/.cache/mise 2>/dev/null || return 1

    # Create extension state directories
    mkdir -p "$workspace"/.system/{manifest,installed,logs} 2>/dev/null || return 1

    # Create shell config files
    touch "$home"/.bashrc 2>/dev/null || true
    touch "$home"/.profile 2>/dev/null || true

    # Ensure directories are accessible
    chmod -R 755 "$home"/.local "$home"/.config "$home"/.cache 2>/dev/null || true

    return 0
}

# Fix permissions on home directory (which is the volume mount)
fix_home_permissions "${HOME}"

# Check if home is initialized
if [[ ! -f "${HOME}/.initialized" ]]; then
    print_status "Initializing home directory..."

    # Create directory structure
    if initialize_home_dirs "${HOME}"; then
        # Setup MOTD
        setup_motd

        # Mark as initialized
        touch "${HOME}/.initialized"
        print_success "Home directory initialized"
    else
        print_error "Failed to initialize home directory"
        # Continue anyway - let subsequent commands fail with better error messages
    fi
fi

# Execute command or shell
exec "$@"
