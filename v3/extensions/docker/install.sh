#!/usr/bin/env bash
set -euo pipefail

# Install script for Docker
# Handles Docker daemon startup, user permissions, and DinD configuration

# Find common.sh relative to this script's location
# Script is at: /opt/sindri/extensions/docker/install.sh
# common.sh is at: /opt/sindri/common.sh (go up 2 levels)
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$(dirname "$(dirname "$SCRIPT_DIR")")/common.sh"

print_status "Configuring Docker..."

# ============================================================================
# DinD Mode Detection
# ============================================================================

# Get DinD mode from environment (set by docker-compose from adapter)
DIND_MODE="${SINDRI_DIND_MODE:-none}"
DIND_STORAGE_SIZE="${SINDRI_DIND_STORAGE_SIZE:-20GB}"
DIND_STORAGE_DRIVER="${SINDRI_DIND_STORAGE_DRIVER:-auto}"

print_status "DinD Mode: $DIND_MODE"

# ============================================================================
# User Configuration
# ============================================================================

# Add developer user to docker group for rootless access
# Use literal "developer" instead of $USER to match sudoers pattern:
# /usr/sbin/usermod -aG * developer
if ! groups developer 2>/dev/null | grep -q docker; then
    print_status "Adding developer to docker group..."
    sudo usermod -aG docker developer
    print_success "User added to docker group (re-login required for effect)"
fi

# ============================================================================
# Storage Driver Configuration
# ============================================================================

# Determine best storage driver based on mode and environment
determine_storage_driver() {
    case "$DIND_MODE" in
        sysbox)
            # Sysbox handles everything - use overlay2 natively
            echo "overlay2"
            ;;
        socket)
            # Socket mode - no daemon, no storage driver needed
            echo "none"
            ;;
        privileged)
            # Privileged mode - select based on configured driver
            case "$DIND_STORAGE_DRIVER" in
                overlay2|fuse-overlayfs|vfs)
                    echo "$DIND_STORAGE_DRIVER"
                    ;;
                auto)
                    # Auto-detect: check if /var/lib/docker is a real filesystem (volume)
                    local docker_fs
                    docker_fs=$(df -T /var/lib/docker 2>/dev/null | tail -1 | awk '{print $2}' || echo "overlay")

                    if [[ "$docker_fs" == "ext4" || "$docker_fs" == "xfs" ]]; then
                        # Real filesystem - try overlay2
                        if modprobe overlay 2>/dev/null && grep -q overlay /proc/filesystems 2>/dev/null; then
                            echo "overlay2"
                            return
                        fi
                    fi

                    # Try fuse-overlayfs
                    if command -v fuse-overlayfs >/dev/null 2>&1; then
                        echo "fuse-overlayfs"
                        return
                    fi

                    # Fallback to vfs
                    echo "vfs"
                    ;;
                *)
                    echo "vfs"
                    ;;
            esac
            ;;
        *)
            # Standard mode - auto-detect for nested environments
            if grep -q docker /proc/1/cgroup 2>/dev/null || [[ -f /.dockerenv ]]; then
                # Running nested - likely need fallback
                if command -v fuse-overlayfs >/dev/null 2>&1; then
                    echo "fuse-overlayfs"
                else
                    echo "vfs"
                fi
            else
                # Not nested - use default
                echo "overlay2"
            fi
            ;;
    esac
}

# ============================================================================
# Docker Daemon Configuration
# ============================================================================

configure_docker_daemon() {
    local storage_driver
    storage_driver=$(determine_storage_driver)

    # Socket mode - no daemon to configure
    if [[ "$storage_driver" == "none" ]]; then
        print_status "Socket mode - using host Docker daemon"
        return 0
    fi

    print_status "Configuring storage driver: $storage_driver"

    local config_file="/etc/docker/daemon.json"
    sudo mkdir -p /etc/docker

    case "$storage_driver" in
        vfs)
            # VFS needs storage limits to prevent disk exhaustion
            local size_num="${DIND_STORAGE_SIZE%GB}"
            size_num="${size_num%MB}"
            sudo tee "$config_file" > /dev/null << EOF
{
    "storage-driver": "vfs",
    "storage-opts": ["size=${size_num}G"],
    "data-root": "/var/lib/docker",
    "log-level": "warn",
    "live-restore": true
}
EOF
            ;;
        fuse-overlayfs)
            sudo tee "$config_file" > /dev/null << 'EOF'
{
    "storage-driver": "fuse-overlayfs",
    "data-root": "/var/lib/docker",
    "log-level": "warn",
    "live-restore": true
}
EOF
            ;;
        overlay2)
            sudo tee "$config_file" > /dev/null << 'EOF'
{
    "storage-driver": "overlay2",
    "data-root": "/var/lib/docker",
    "log-level": "warn",
    "live-restore": true
}
EOF
            ;;
        *)
            # Minimal config - let Docker choose
            sudo tee "$config_file" > /dev/null << 'EOF'
{
    "data-root": "/var/lib/docker",
    "log-level": "warn",
    "live-restore": true
}
EOF
            ;;
    esac
}

# Configure daemon
configure_docker_daemon

# ============================================================================
# Docker Daemon Startup Script
# ============================================================================

# Socket mode doesn't need daemon startup
if [[ "$DIND_MODE" == "socket" ]]; then
    print_status "Socket mode - skipping daemon startup configuration"

    # Test Docker access via socket
    if docker info > /dev/null 2>&1; then
        print_success "Docker socket accessible"
    else
        print_warning "Docker socket not accessible - check socket mount"
    fi
else
    # Create Docker daemon startup script
    # In containerized environments without systemd, we need to start dockerd manually
    DOCKER_START_SCRIPT="$HOME/.local/bin/start-dockerd.sh"
    ensure_directory "$(dirname "$DOCKER_START_SCRIPT")"
    ensure_directory "$HOME/.local/state"

    cat > "$DOCKER_START_SCRIPT" << 'STARTUP_EOF'
#!/usr/bin/env bash
# Auto-start Docker daemon if not running
# Runs in background and is idempotent

# Socket mode - skip daemon start
if [[ "${SINDRI_DIND_MODE:-}" == "socket" ]]; then
    exit 0
fi

if ! pgrep -x dockerd > /dev/null 2>&1; then
    # Start dockerd in background with proper logging
    sudo nohup dockerd \
        > "$HOME/.local/state/dockerd.log" 2>&1 &

    # Wait for daemon to be ready (max 15 seconds)
    for i in {1..15}; do
        if docker info > /dev/null 2>&1; then
            exit 0
        fi
        sleep 1
    done

    echo "Warning: Docker daemon may not be fully ready" >&2
fi
STARTUP_EOF

    chmod +x "$DOCKER_START_SCRIPT"

    # Add to .bashrc to auto-start on login
    if ! grep -q "start-dockerd.sh" "$HOME/.bashrc" 2>/dev/null; then
        cat >> "$HOME/.bashrc" << 'BASHRC_EOF'

# Auto-start Docker daemon
if command -v docker >/dev/null 2>&1 && [[ "${SINDRI_DIND_MODE:-}" != "socket" ]]; then
    "$HOME/.local/bin/start-dockerd.sh" 2>/dev/null || true
fi
BASHRC_EOF
    fi

    # Start Docker daemon now
    print_status "Starting Docker daemon..."
    "$DOCKER_START_SCRIPT"

    # Wait a moment for daemon
    sleep 3
fi

# ============================================================================
# Verification
# ============================================================================

# Test docker access (with sudo since group membership requires re-login)
if docker info > /dev/null 2>&1; then
    print_success "Docker daemon accessible"

    # Show storage driver info
    storage_info=$(docker info 2>/dev/null | grep "Storage Driver" || echo "Storage Driver: unknown")
    print_status "$storage_info"
elif sudo docker info > /dev/null 2>&1; then
    print_success "Docker daemon started successfully (sudo required)"
    print_status "Note: Log out and back in for docker group access (to use docker without sudo)"
else
    print_warning "Docker daemon may not be fully ready yet"
    print_status "Try: sudo docker info"
fi

# ============================================================================
# DinD Mode Summary
# ============================================================================

case "$DIND_MODE" in
    sysbox)
        print_success "Docker configured for Sysbox DinD (secure, unprivileged)"
        print_status "Inner Docker will use overlay2 natively"
        ;;
    privileged)
        print_success "Docker configured for privileged DinD"
        print_status "Inner Docker using $(determine_storage_driver) storage driver"
        print_warning "Note: Privileged mode provides full host access - use with caution"
        ;;
    socket)
        print_success "Docker configured for socket binding"
        print_status "Using host Docker daemon via /var/run/docker.sock"
        print_warning "Note: Containers created will be visible on host"
        ;;
    *)
        print_success "Docker configuration complete"
        print_status "Note: Log out and back in for docker group access (to use docker without sudo)"
        ;;
esac
