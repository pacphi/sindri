#!/bin/bash
# Fast initialization entrypoint
# Supports both interactive shell (Docker) and SSH server mode (Fly.io)

set -e

# Source common functions
source /docker/lib/common.sh

# Use environment variables with fallbacks
HOME="${HOME:-/alt/home/developer}"
WORKSPACE="${WORKSPACE:-${HOME}/workspace}"
SSH_PORT="${SSH_PORT:-2222}"

# Setup MOTD banner
setup_motd() {
    if [[ -f "/docker/scripts/setup-motd.sh" ]]; then
        sudo bash /docker/scripts/setup-motd.sh 2>/dev/null || true
    fi
}

# Configure SSH server for non-standard port
# Required for Fly.io where port 22 is reserved for internal use
# See: https://fly.io/docs/blueprints/opensshd/
configure_sshd() {
    local ssh_port="${1:-2222}"

    # Create sshd config directory if needed
    sudo mkdir -p /run/sshd 2>/dev/null || true

    # Configure sshd to listen on non-standard port
    if [[ -f /etc/ssh/sshd_config ]]; then
        # Update port if not already set correctly
        if ! grep -q "^Port ${ssh_port}" /etc/ssh/sshd_config 2>/dev/null; then
            sudo sed -i "s/^#*Port .*/Port ${ssh_port}/" /etc/ssh/sshd_config 2>/dev/null || true
            # If Port line doesn't exist, add it
            if ! grep -q "^Port " /etc/ssh/sshd_config 2>/dev/null; then
                echo "Port ${ssh_port}" | sudo tee -a /etc/ssh/sshd_config >/dev/null 2>&1 || true
            fi
        fi

        # Enable password authentication for developer user (can be disabled via config)
        sudo sed -i 's/^#*PasswordAuthentication .*/PasswordAuthentication yes/' /etc/ssh/sshd_config 2>/dev/null || true
        sudo sed -i 's/^#*PermitRootLogin .*/PermitRootLogin no/' /etc/ssh/sshd_config 2>/dev/null || true

        # Allow the developer user
        if ! grep -q "^AllowUsers" /etc/ssh/sshd_config 2>/dev/null; then
            echo "AllowUsers developer" | sudo tee -a /etc/ssh/sshd_config >/dev/null 2>&1 || true
        fi
    fi

    # Generate host keys if they don't exist
    if [[ ! -f /etc/ssh/ssh_host_rsa_key ]]; then
        sudo ssh-keygen -A 2>/dev/null || true
    fi

    print_status "SSH server configured on port ${ssh_port}"
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

    # Create .ssh directory for SSH keys
    mkdir -p "$home"/.ssh 2>/dev/null || true
    chmod 700 "$home"/.ssh 2>/dev/null || true

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

        # Copy skeleton files to home (including welcome.sh)
        if [[ -d /etc/skel ]]; then
            cp -rn /etc/skel/. "${HOME}/" 2>/dev/null || true
        fi

        # Mark as initialized
        touch "${HOME}/.initialized"
        print_success "Home directory initialized"

        # Show first-login welcome message (only for interactive shells)
        if [[ -t 0 ]] && [[ -x "${HOME}/welcome.sh" ]]; then
            "${HOME}/welcome.sh"
        fi
    else
        print_error "Failed to initialize home directory"
        # Continue anyway - let subsequent commands fail with better error messages
    fi
fi

# Determine if we're starting SSH server
# This is triggered by:
# 1. START_SSHD=true environment variable
# 2. Command containing sshd
is_sshd_command() {
    [[ "${START_SSHD:-}" == "true" ]] && return 0
    [[ "$*" == *"sshd"* ]] && return 0
    return 1
}

# If running sshd, configure and start it
if is_sshd_command "$@"; then
    configure_sshd "${SSH_PORT}"

    # sshd requires root to bind to privileged ports and manage sessions
    # Use sudo to start sshd, then it will handle user sessions
    print_status "Starting SSH server..."

    # If the command is just sshd with options, run it via sudo
    if [[ "$1" == *"sshd"* ]] || [[ "$1" == "/usr/sbin/sshd" ]]; then
        exec sudo "$@"
    elif [[ "${START_SSHD:-}" == "true" ]]; then
        # For Fly.io: START_SSHD=true means run sshd in foreground
        # This keeps the container alive and accepts SSH connections
        # -D: Don't daemonize, -e: Log to stderr
        exec sudo /usr/sbin/sshd -D -e
    else
        # Start sshd in the background and run the provided command
        sudo /usr/sbin/sshd
        exec "$@"
    fi
fi

# Execute command or shell (default behavior for Docker)
exec "$@"
