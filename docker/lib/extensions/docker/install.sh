#!/usr/bin/env bash
set -euo pipefail

# Install script for Docker
# Handles Docker daemon startup and user permissions

source "${DOCKER_LIB:-/docker/lib}/common.sh"

print_status "Configuring Docker..."

# Add developer user to docker group for rootless access
# Use literal "developer" instead of $USER to match sudoers pattern:
# /usr/sbin/usermod -aG * developer
if ! groups developer 2>/dev/null | grep -q docker; then
    print_status "Adding developer to docker group..."
    sudo usermod -aG docker developer
    print_success "User added to docker group (re-login required for effect)"
fi

# Create Docker daemon startup script
# In Fly.io containers without systemd, we need to start dockerd manually
DOCKER_START_SCRIPT="$HOME/.local/bin/start-dockerd.sh"
ensure_directory "$(dirname "$DOCKER_START_SCRIPT")"

cat > "$DOCKER_START_SCRIPT" << 'EOF'
#!/usr/bin/env bash
# Auto-start Docker daemon if not running
# Runs in background and is idempotent

if ! pgrep -x dockerd > /dev/null 2>&1; then
    # Start dockerd in background with proper logging
    sudo nohup dockerd \
        --data-root=/var/lib/docker \
        --log-level=error \
        > "$HOME/.local/state/dockerd.log" 2>&1 &

    # Wait for daemon to be ready (max 10 seconds)
    for i in {1..10}; do
        if sudo docker info > /dev/null 2>&1; then
            exit 0
        fi
        sleep 1
    done
fi
EOF

chmod +x "$DOCKER_START_SCRIPT"

# Add to .bashrc to auto-start on login
if ! grep -q "start-dockerd.sh" "$HOME/.bashrc" 2>/dev/null; then
    cat >> "$HOME/.bashrc" << 'EOF'

# Auto-start Docker daemon
if command -v docker >/dev/null 2>&1; then
    "$HOME/.local/bin/start-dockerd.sh" 2>/dev/null || true
fi
EOF
fi

# Start Docker daemon now
print_status "Starting Docker daemon..."
"$DOCKER_START_SCRIPT"

# Wait for daemon to be ready
sleep 2

# Test docker access (with sudo since group membership requires re-login)
if sudo docker info > /dev/null 2>&1; then
    print_success "Docker daemon started successfully"
else
    print_warning "Docker daemon may not be fully ready yet"
    print_status "Try: sudo docker info"
fi

print_success "Docker configuration complete"
print_status "Note: Log out and back in for docker group access (to use docker without sudo)"
